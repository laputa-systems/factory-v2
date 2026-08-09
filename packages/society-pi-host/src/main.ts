/** The supervised executable. It never opens a database or a control socket. */

import { isUtf8 } from "node:buffer";

import { PiSdkHost, localRuntimeIdentity } from "./host.js";
import { MAX_JSONL_FRAME_BYTES, decodeInboundJsonl, sessionIdentity, blake3Digest, spawnNonce } from "./protocol.js";
import { PinnedPiSdkRuntime } from "./sdk.js";

interface ProcessArguments {
	readonly sessionIdentity: string;
	readonly spawnNonce: string;
	readonly nodeExecutableBlake3: string;
	readonly lockfileBlake3: string;
	readonly adapterBuildBlake3: string;
	readonly piTransitivePackageSetBlake3: string;
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
				nodeExecutableBlake3: blake3Digest(argumentsValue.nodeExecutableBlake3),
				lockfileBlake3: blake3Digest(argumentsValue.lockfileBlake3),
				adapterBuildBlake3: blake3Digest(argumentsValue.adapterBuildBlake3),
				piTransitivePackageSetBlake3: blake3Digest(argumentsValue.piTransitivePackageSetBlake3),
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
		if (line === undefined) {
			host.protocolDecodeFailed();
			process.exitCode = 1;
			return;
		}
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
export async function consumeInboundJsonl(
	input: AsyncIterable<Uint8Array | string>,
	onLine: (line: string | undefined) => void,
): Promise<void> {
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
				onLine(undefined);
			} else {
				onLine(decodeInboundRecord(Buffer.concat(buffered, bufferedBytes)));
			}
			buffered = [];
			bufferedBytes = 0;
			overlong = false;
			start = newline + 1;
			if (start === chunk.length) break;
		}
	}
	if (overlong) {
		onLine(undefined);
	} else if (bufferedBytes !== 0) {
		onLine(decodeInboundRecord(Buffer.concat(buffered, bufferedBytes)));
	}
}

/**
 * `Buffer#toString("utf8")` replacement-decodes invalid bytes. That would
 * make the byte pipe and typed command protocol disagree, so validate the
 * complete bounded record before conversion (including a final EOF fragment).
 */
function decodeInboundRecord(record: Buffer): string | undefined {
	if (!isUtf8(record)) return undefined;
	let line = record.toString("utf8");
	if (line.endsWith("\r")) line = line.slice(0, -1);
	return line;
}

function parseProcessArguments(argumentsValue: readonly string[]): ProcessArguments {
	if (
		argumentsValue.length !== 12 ||
		argumentsValue[0] !== "--session-identity" ||
		argumentsValue[2] !== "--spawn-nonce" ||
		argumentsValue[4] !== "--node-executable-blake3" ||
		argumentsValue[6] !== "--lockfile-blake3" ||
		argumentsValue[8] !== "--adapter-build-blake3" ||
		argumentsValue[10] !== "--pi-transitive-package-set-blake3"
	) {
		throw new Error("usage: society-pi-host --session-identity <id> --spawn-nonce <nonce> --node-executable-blake3 <blake3> --lockfile-blake3 <blake3> --adapter-build-blake3 <blake3> --pi-transitive-package-set-blake3 <blake3>");
	}
	const session = argumentsValue[1];
	const nonce = argumentsValue[3];
	const nodeExecutableBlake3 = argumentsValue[5];
	const lockfileBlake3 = argumentsValue[7];
	const adapterBuildBlake3 = argumentsValue[9];
	const piTransitivePackageSetBlake3 = argumentsValue[11];
	if (
		session === undefined || nonce === undefined || nodeExecutableBlake3 === undefined || lockfileBlake3 === undefined ||
		adapterBuildBlake3 === undefined || piTransitivePackageSetBlake3 === undefined
	) throw new Error("missing_host_identity");
	return { sessionIdentity: session, spawnNonce: nonce, nodeExecutableBlake3, lockfileBlake3, adapterBuildBlake3, piTransitivePackageSetBlake3 };
}

if (import.meta.url === new URL(process.argv[1] ?? "", "file:").href) {
	void run().catch(() => {
		process.exitCode = 1;
	});
}
