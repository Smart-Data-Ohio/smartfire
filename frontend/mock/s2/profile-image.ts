/**
 * The mock's reading of a workspace logo or banner, as the server reads it: the format comes from
 * the bytes (not the declared type), the size from the header, and "animated" means a GIF or WebP
 * with more than one frame. An animated PNG counts as still, as the server's pipeline sees it.
 */
import { imageSize } from "./assets.ts";

export interface ProfileImageInfo {
  readonly format: "png" | "jpeg" | "gif" | "webp";
  readonly width: number;
  readonly height: number;
  readonly animated: boolean;
}

const ascii = (bytes: Uint8Array, at: number, length: number): string =>
  String.fromCharCode(...bytes.subarray(at, at + length));

/** Skips a GIF's data sub-blocks from `at`; the offset after the terminator, or `null` past the end. */
function skipSubBlocks(bytes: Uint8Array, at: number): number | null {
  let offset = at;

  while (offset < bytes.length) {
    const size = bytes[offset] ?? 0;

    offset += 1;

    if (size === 0) return offset;

    offset += size;
  }

  return null;
}

/** A GIF's frame count (stopping at 2), or `null` when its blocks don't parse. */
function gifFrames(bytes: Uint8Array): number | null {
  if (bytes.length < 13) return null;

  const packed = bytes[10] ?? 0;
  let offset = 13 + (packed & 0x80 ? 3 * 2 ** ((packed & 0x07) + 1) : 0);
  let frames = 0;

  while (offset < bytes.length) {
    const block = bytes[offset];

    if (block === 0x3b) return frames;

    if (block === 0x21) {
      const next = skipSubBlocks(bytes, offset + 2);

      if (next === null) return null;

      offset = next;
    } else if (block === 0x2c) {
      const descriptor = bytes[offset + 9] ?? 0;
      const local = descriptor & 0x80 ? 3 * 2 ** ((descriptor & 0x07) + 1) : 0;
      const next = skipSubBlocks(bytes, offset + 10 + local + 1);

      if (next === null) return null;

      frames += 1;

      if (frames > 1) return frames;

      offset = next;
    } else {
      return null;
    }
  }

  return null;
}

/** A WebP's size and whether it's animated, from its first chunk; `null` when unreadable. */
function webp(bytes: Uint8Array): Omit<ProfileImageInfo, "format"> | null {
  if (bytes.length < 30) return null;

  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const chunk = ascii(bytes, 12, 4);

  if (chunk === "VP8X") {
    const u24 = (at: number) =>
      (bytes[at] ?? 0) | ((bytes[at + 1] ?? 0) << 8) | ((bytes[at + 2] ?? 0) << 16);

    const width = 1 + u24(24);
    const height = 1 + u24(27);

    return { width, height, animated: ((bytes[20] ?? 0) & 0x02) !== 0 };
  }

  if (chunk === "VP8 ") {
    return {
      width: view.getUint16(26, true) & 0x3fff,
      height: view.getUint16(28, true) & 0x3fff,
      animated: false,
    };
  }

  if (chunk === "VP8L") {
    const bits = view.getUint32(21, true);

    return { width: (bits & 0x3fff) + 1, height: ((bits >> 14) & 0x3fff) + 1, animated: false };
  }

  return null;
}

/** The image `bytes` hold, or `null` when they aren't a readable PNG, JPEG, GIF or WebP. */
export function readProfileImage(bytes: Uint8Array): ProfileImageInfo | null {
  if (ascii(bytes, 0, 4) === "RIFF" && ascii(bytes, 8, 4) === "WEBP") {
    const info = webp(bytes);

    return info === null ? null : { format: "webp", ...info };
  }

  const png = bytes[0] === 0x89 && ascii(bytes, 1, 3) === "PNG" && ascii(bytes, 12, 4) === "IHDR";
  const gif = ascii(bytes, 0, 6) === "GIF87a" || ascii(bytes, 0, 6) === "GIF89a";
  const jpeg = bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff;
  const format = png ? "png" : gif ? "gif" : jpeg ? "jpeg" : null;

  if (format === null) return null;

  const size = imageSize(bytes, `image/${format}`);

  if (size === null || size[0] <= 0 || size[1] <= 0) return null;

  if (format !== "gif") return { format, width: size[0], height: size[1], animated: false };

  const frames = gifFrames(bytes);

  return frames === null || frames === 0
    ? null
    : { format, width: size[0], height: size[1], animated: frames > 1 };
}

/** The largest logo and banner the server takes, in pixels. */
export const PROFILE_MAX_SIZE = {
  logo: [4096, 4096],
  banner: [4096, 2304],
} as const;
