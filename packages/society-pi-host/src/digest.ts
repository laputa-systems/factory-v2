import { blake3 } from "@noble/hashes/blake3.js";

/** Return the canonical 32-byte BLAKE3 digest as lowercase hexadecimal. */
export function blake3Hex(bytes: Uint8Array | string): string {
	const input = typeof bytes === "string" ? Buffer.from(bytes, "utf8") : bytes;
	return Buffer.from(blake3(input)).toString("hex");
}
