/**
 * A small streaming MD5 (RFC 1321). Active Storage checks a direct upload against the base64 MD5
 * the client declares, and WebCrypto has no MD5, so the composer hashes files here: chunk by
 * chunk, so a large file never sits in memory twice.
 */

/** Per-round left-rotation amounts. */
const SHIFTS = [
  7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14,
  20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6,
  10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

/** `floor(abs(sin(i + 1)) * 2^32)`, the per-step constants. */
const CONSTANTS = Int32Array.from({ length: 64 }, (_, index) =>
  Math.floor(Math.abs(Math.sin(index + 1)) * 2 ** 32),
);

const BLOCK = 64;

function rotateLeft(value: number, amount: number): number {
  return (value << amount) | (value >>> (32 - amount));
}

/** An incremental MD5: `update` with as many chunks as you like, then `digest` once. */
export class Md5 {
  private state = Int32Array.of(0x67452301, 0xefcdab89 | 0, 0x98badcfe | 0, 0x10325476);

  private readonly buffer = new Uint8Array(BLOCK);

  private buffered = 0;

  private length = 0;

  private readonly words = new Int32Array(16);

  /** Feeds the next chunk of bytes. */
  update(chunk: ArrayBuffer | Uint8Array): this {
    const bytes = chunk instanceof Uint8Array ? chunk : new Uint8Array(chunk);
    let offset = 0;

    this.length += bytes.length;

    // Top up a partly filled block first.
    if (this.buffered > 0) {
      const take = Math.min(BLOCK - this.buffered, bytes.length);

      this.buffer.set(bytes.subarray(0, take), this.buffered);
      this.buffered += take;
      offset = take;

      if (this.buffered < BLOCK) {
        return this;
      }

      this.block(this.buffer, 0);
      this.buffered = 0;
    }

    while (offset + BLOCK <= bytes.length) {
      this.block(bytes, offset);
      offset += BLOCK;
    }

    this.buffer.set(bytes.subarray(offset), 0);
    this.buffered = bytes.length - offset;

    return this;
  }

  /** The 16-byte digest. Pads and finishes the hash; the instance is spent afterwards. */
  digest(): Uint8Array {
    const bitLength = this.length * 8;

    const padding = new Uint8Array(
      this.buffered < 56 ? 56 - this.buffered : BLOCK + 56 - this.buffered,
    );

    padding[0] = 0x80;

    const tail = new Uint8Array(8);
    const view = new DataView(tail.buffer);

    view.setUint32(0, bitLength >>> 0, true);
    view.setUint32(4, Math.floor(bitLength / 2 ** 32), true);

    // The length counts message bytes only: keep it while padding goes through update().
    const length = this.length;

    this.update(padding);
    this.update(tail);
    this.length = length;

    const out = new Uint8Array(16);
    const outView = new DataView(out.buffer);

    for (let index = 0; index < 4; index += 1) {
      outView.setInt32(index * 4, this.state[index] ?? 0, true);
    }

    return out;
  }

  private block(bytes: Uint8Array, offset: number): void {
    const words = this.words;
    const view = new DataView(bytes.buffer, bytes.byteOffset + offset, BLOCK);

    for (let index = 0; index < 16; index += 1) {
      words[index] = view.getInt32(index * 4, true);
    }

    let [a = 0, b = 0, c = 0, d = 0] = this.state;

    for (let step = 0; step < 64; step += 1) {
      let mixed: number;
      let wordIndex: number;

      if (step < 16) {
        mixed = (b & c) | (~b & d);
        wordIndex = step;
      } else if (step < 32) {
        mixed = (d & b) | (~d & c);
        wordIndex = (5 * step + 1) % 16;
      } else if (step < 48) {
        mixed = b ^ c ^ d;
        wordIndex = (3 * step + 5) % 16;
      } else {
        mixed = c ^ (b | ~d);
        wordIndex = (7 * step) % 16;
      }

      const sum = (a + mixed + (CONSTANTS[step] ?? 0) + (words[wordIndex] ?? 0)) | 0;

      a = d;
      d = c;
      c = b;
      b = (b + rotateLeft(sum, SHIFTS[step] ?? 0)) | 0;
    }

    const state = this.state;

    state[0] = ((state[0] ?? 0) + a) | 0;
    state[1] = ((state[1] ?? 0) + b) | 0;
    state[2] = ((state[2] ?? 0) + c) | 0;
    state[3] = ((state[3] ?? 0) + d) | 0;
  }
}

/** Lowercase hex, as `md5sum` prints it. */
export function toHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

/** Standard base64, as Active Storage's `checksum` expects. */
export function toBase64(bytes: Uint8Array): string {
  return btoa(String.fromCharCode(...bytes));
}

/** How much of a file is read per step while hashing. */
export const HASH_CHUNK_BYTES = 2 * 1024 * 1024;

/**
 * The base64 MD5 of a file, read `chunkBytes` at a time. Rejects with an `AbortError` once
 * `signal` aborts.
 */
export async function md5Base64(
  file: Blob,
  signal?: AbortSignal,
  chunkBytes: number = HASH_CHUNK_BYTES,
): Promise<string> {
  const hash = new Md5();

  for (let offset = 0; offset < file.size; offset += chunkBytes) {
    signal?.throwIfAborted();
    hash.update(await file.slice(offset, offset + chunkBytes).arrayBuffer());
  }

  signal?.throwIfAborted();

  return toBase64(hash.digest());
}
