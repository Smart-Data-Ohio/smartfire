/**
 * The checksums the mock needs without Node's crypto (it also runs in jsdom and the browser,
 * and WebCrypto has no MD5): MD5 for Active Storage's upload checksum, and CRC-32 and Adler-32
 * for the PNGs it draws.
 */

const MD5_SHIFTS = [
  7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14,
  20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6,
  10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

const MD5_CONSTANTS = Array.from(
  { length: 64 },
  (_, index) => Math.floor(Math.abs(Math.sin(index + 1)) * 2 ** 32) >>> 0,
);

/** The MD5 digest of `bytes` (16 bytes). */
export function md5(bytes: Uint8Array): Uint8Array {
  const length = bytes.length;
  const padded = new Uint8Array((((length + 8) >> 6) + 1) << 6);

  padded.set(bytes);
  padded[length] = 0x80;

  const view = new DataView(padded.buffer);

  view.setUint32(padded.length - 8, (length * 8) >>> 0, true);
  view.setUint32(padded.length - 4, Math.floor((length * 8) / 2 ** 32), true);

  let a0 = 0x67452301;
  let b0 = 0xefcdab89;
  let c0 = 0x98badcfe;
  let d0 = 0x10325476;

  for (let offset = 0; offset < padded.length; offset += 64) {
    let a = a0;
    let b = b0;
    let c = c0;
    let d = d0;

    for (let step = 0; step < 64; step++) {
      let f: number;
      let g: number;

      if (step < 16) {
        f = (b & c) | (~b & d);
        g = step;
      } else if (step < 32) {
        f = (d & b) | (~d & c);
        g = (5 * step + 1) % 16;
      } else if (step < 48) {
        f = b ^ c ^ d;
        g = (3 * step + 5) % 16;
      } else {
        f = c ^ (b | ~d);
        g = (7 * step) % 16;
      }

      const word = view.getUint32(offset + g * 4, true);
      const sum = (a + f + (MD5_CONSTANTS[step] ?? 0) + word) >>> 0;
      const shift = MD5_SHIFTS[step] ?? 0;

      a = d;
      d = c;
      c = b;
      b = (b + ((sum << shift) | (sum >>> (32 - shift)))) >>> 0;
    }

    a0 = (a0 + a) >>> 0;
    b0 = (b0 + b) >>> 0;
    c0 = (c0 + c) >>> 0;
    d0 = (d0 + d) >>> 0;
  }

  const digest = new Uint8Array(16);
  const out = new DataView(digest.buffer);

  [a0, b0, c0, d0].forEach((word, position) => {
    out.setUint32(position * 4, word, true);
  });

  return digest;
}

/** Standard base64 of `bytes`. */
export function base64(bytes: Uint8Array): string {
  let binary = "";

  for (const byte of bytes) binary += String.fromCharCode(byte);

  return btoa(binary);
}

/** Base64 of the MD5 digest: Active Storage's `checksum`. */
export function md5Base64(bytes: Uint8Array): string {
  return base64(md5(bytes));
}

let crcTable: Uint32Array | null = null;

/** CRC-32 (the PNG chunk checksum). */
export function crc32(bytes: Uint8Array): number {
  if (crcTable === null) {
    crcTable = new Uint32Array(256);

    for (let n = 0; n < 256; n++) {
      let c = n;

      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;

      crcTable[n] = c >>> 0;
    }
  }

  let crc = 0xffffffff;

  for (const byte of bytes) crc = (crcTable[(crc ^ byte) & 0xff] ?? 0) ^ (crc >>> 8);

  return (crc ^ 0xffffffff) >>> 0;
}

/** Adler-32 (the zlib stream checksum). */
export function adler32(bytes: Uint8Array): number {
  let a = 1;
  let b = 0;

  for (const byte of bytes) {
    a = (a + byte) % 65521;
    b = (b + a) % 65521;
  }

  return ((b << 16) | a) >>> 0;
}
