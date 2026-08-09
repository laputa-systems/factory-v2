/** The supervised executable. It never opens a database or a control socket. */

import { PiSdkHost, localRuntimeIdentity } from "./host.js";
import { MAX_JSONL_FRAME_BYTES, decodeInboundJsonl, sessionIdentity, sha256Digest, spawnNonce } from "./protocol.js";
import { PinnedPiSdkRuntime } from "./sdk.js";

interface ProcessArguments {
	readonly sessionIdentity: string;
	readonly spawnNonce: string;
	readonly nodeExecutableSha256: string;
	readonly lockfileSha256: string;
	readonly adapterBuildSha256: string;
	readonly piTransitivePackageSetSha256: string;
}

export async function run(): Promise<void> {
	const argumentsValue = parseProcessArguments(process.argv.slice(2));
	let stdoutFailed = false;
	let host: PiSdkHost | undefined;
	const containOutboundTransport = (): void => {
		stdoutFailed = true;
		process.exitCode = 1;
		host?.outboundTransportFailed();
	};
	process.stdout.on("error", () => {
		// EPIPE is an ordinary supervisor-containment outcome. Do not let Node
		// convert it into an unhandled EventEmitter error or write diagnostics to
		// the JSONL stdout channel.
		containOutboundTransport();
	});
	host = new PiSdkHost(
		{
			sessionIdentity: sessionIdentity(argumentsValue.sessionIdentity),
			spawnNonce: spawnNonce(argumentsValue.spawnNonce),
			pid: process.pid,
			runtime: localRuntimeIdentity(process.version, {
				nodeExecutableSha256: sha256Digest(argumentsValue.nodeExecutableSha256),
				lockfileSha256: sha256Digest(argumentsValue.lockfileSha256),
				adapterBuildSha256: sha256Digest(argumentsValue.adapterBuildSha256),
				piTransitivePackageSetSha256: sha256Digest(argumentsValue.piTransitivePackageSetSha256),
			}),
		},
		new PinnedPiSdkRuntime(),
		(frame) => {
			if (stdoutFailed) {
				containOutboundTransport();
				return;
			}
			try {
				if (!process.stdout.write(`${JSON.stringify(frame)}\n`)) containOutboundTransport();
			} catch {
				containOutboundTransport();
			}
		},
	);
	if (stdoutFailed) containOutboundTransport();
	await consumeInboundJsonl(process.stdin, (line) => {
		try {
			void host?.accept(decodeInboundJsonl(line)).catch(() => process.exitCode = 1);
		} catch {
			host.protocolDecodeFailed();
			process.exitCode = 1;
		}
	});
	try {
		await host.onControlPipeEof();
	} catch {
		process.exitCode = 1;
	}
}

/**
 * A bounded byte-level JSONL decoder. readline builds an unbounded string for
 * a line before its callback runs; this decoder abandons an overlong line as
 * soon as its byte budget is crossed and only retains bounded chunks.
 */
export async function consumeInboundJsonl(input: AsyncIterable<Uint8Array | string>, onLine: (line: string) => void): Promise<void> {
	let buffered: Buffer[] = [];
	let bufferedBytes = 0;
	let overlong = false;
	for await (const sourceChunk of input) {
		const chunk = typeof sourceChunk === "string"
			? Buffer.from(sourceChunk, "utf8")
			: Buffer.isBuffer(sourceChunk) ? sourceChunk : Buffer.from(sourceChunk);
		let start = 0;
		for (;;) {
			const newline = chunk.indexOf(0x0a, start);
			const end = newline === -1 ? chunk.length : newline;
			const segment = chunk.subarray(start, end);
			if (!overlong) {
				if (bufferedBytes + segment.length > MAX_JSONL_FRAME_BYTES) {
					overlong = true;
					buffered = [];
					bufferedBytes = 0;
				} else {
					buffered.push(segment);
					bufferedBytes += segment.length;
				}
			}
			if (newline === -1) break;
			if (overlong) {
				onLine("{"); // deterministically enters the host's typed decode-fatal path.
			} else {
				let line = Buffer.concat(buffered, bufferedBytes).toString("utf8");
				if (line.endsWith("\r")) line = line.slice(0, -1);
				onLine(line);
			}
			buffered = [];
			bufferedBytes = 0;
			overlong = false;
			start = newline + 1;
			if (start === chunk.length) break;
		}
	}
	if (overlong) {
		onLine("{");
	} else if (bufferedBytes !== 0) {
		let line = Buffer.concat(buffered, bufferedBytes).toString("utf8");
		if (line.endsWith("\r")) line = line.slice(0, -1);
		onLine(line);
	}
}

function parseProcessArguments(argumentsValue: readonly string[]): ProcessArguments {
	if (
		argumentsValue.length !== 12 ||
		argumentsValue[0] !== "--session-identity" ||
		argumentsValue[2] !== "--spawn-nonce" ||
		argumentsValue[4] !== "--node-executable-sha256" ||
		argumentsValue[6] !== "--lockfile-sha256" ||
		argumentsValue[8] !== "--adapter-build-sha256" ||
		argumentsValue[10] !== "--pi-transitive-package-set-sha256"
	) {
		throw new Error("usage: society-pi-host --session-identity <id> --spawn-nonce <nonce> --node-executable-sha256 <sha256> --lockfile-sha256 <sha256> --adapter-build-sha256 <sha256> --pi-transitive-package-set-sha256 <sha256>");
	}
	const session = argumentsValue[1];
	const nonce = argumentsValue[3];
	const nodeExecutableSha256 = argumentsValue[5];
	const lockfileSha256 = argumentsValue[7];
	const adapterBuildSha256 = argumentsValue[9];
	const piTransitivePackageSetSha256 = argumentsValue[11];
	if (
		session === undefined || nonce === undefined || nodeExecutableSha256 === undefined || lockfileSha256 === undefined ||
		adapterBuildSha256 === undefined || piTransitivePackageSetSha256 === undefined
	) throw new Error("missing_host_identity");
	return { sessionIdentity: session, spawnNonce: nonce, nodeExecutableSha256, lockfileSha256, adapterBuildSha256, piTransitivePackageSetSha256 };
}

if (import.meta.url === new URL(process.argv[1] ?? "", "file:").href) {
	void run().catch(() => {
		process.exitCode = 1;
	});
}
