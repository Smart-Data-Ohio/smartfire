/**
 * The checks a sticker file gets before it uploads, from its first bytes: the server's own rules
 * (crates/storage's sticker preparation), so a wrong file is refused at once rather than after
 * the upload. The server still checks everything, frames included.
 */

/** The image files a sticker may be: PNG (APNG included) or GIF. */
export const STICKER_FILE_TYPES = "image/png,image/apng,image/gif";

/** The largest sticker file, in bytes. */
export const STICKER_MAX_BYTES = 512 * 1024;

/** The largest sticker, in pixels each way. */
export const STICKER_MAX_SIDE = 320;

/** An image's format and size, read from its header. */
export interface ImageHeader {
  readonly format: "png" | "gif";
  readonly width: number;
  readonly height: number;
}

const PNG_SIGNATURE = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

/** A PNG's or GIF's format and size from its first bytes; `null` for anything else. */
export function readImageHeader(bytes: Uint8Array): ImageHeader | null {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const ascii = (from: number, to: number) => String.fromCharCode(...bytes.subarray(from, to));

  // The signature, then the IHDR chunk's length and type, then its width and height.
  if (
    bytes.length >= 24 &&
    PNG_SIGNATURE.every((byte, index) => bytes[index] === byte) &&
    ascii(12, 16) === "IHDR"
  ) {
    return { format: "png", width: view.getUint32(16), height: view.getUint32(20) };
  }

  // The header, then the logical screen's width and height, little-endian.
  if (bytes.length >= 10 && (ascii(0, 6) === "GIF87a" || ascii(0, 6) === "GIF89a")) {
    return { format: "gif", width: view.getUint16(6, true), height: view.getUint16(8, true) };
  }

  return null;
}

/** Why `file` can't be a sticker, as it shows under the field; `null` when it may upload. */
export async function stickerFileProblem(file: File): Promise<string | null> {
  if (file.size > STICKER_MAX_BYTES) {
    return "Image must be 512 KB or smaller.";
  }

  const header = readImageHeader(new Uint8Array(await file.slice(0, 32).arrayBuffer()));

  if (header === null) {
    return "Image must be a PNG, APNG or GIF image.";
  }

  if (header.width > STICKER_MAX_SIDE || header.height > STICKER_MAX_SIDE) {
    return `Image must be at most 320 × 320 pixels (this one is ${header.width} × ${header.height}).`;
  }

  return null;
}
