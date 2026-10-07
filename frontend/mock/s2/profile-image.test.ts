import { describe, expect, it } from "vitest";
import { readProfileImage } from "./profile-image.ts";

/** A GIF with `frames` empty image blocks (block structure only, which is all the reader walks). */
function gif(frames: number, width = 4, height = 3): Uint8Array {
  const image = [0x2c, 0, 0, 0, 0, width, 0, height, 0, 0, 2, 1, 0x44, 0];

  return Uint8Array.from([
    ...new TextEncoder().encode("GIF89a"),
    width,
    0,
    height,
    0,
    0,
    0,
    0,
    0x21,
    0xf9,
    4,
    0,
    10,
    0,
    0,
    0,
    ...Array.from({ length: frames }, () => image).flat(),
    0x3b,
  ]);
}

/** A PNG signature and IHDR claiming `width` × `height`, plus `extra` chunk types (empty). */
function png(width: number, height: number, extra: readonly string[] = []): Uint8Array {
  const bytes = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13];
  const view = new DataView(new ArrayBuffer(8));

  view.setUint32(0, width);
  view.setUint32(4, height);
  bytes.push(...new TextEncoder().encode("IHDR"), ...new Uint8Array(view.buffer), 8, 2, 0, 0, 0);
  bytes.push(0, 0, 0, 0);

  for (const type of extra) bytes.push(0, 0, 0, 0, ...new TextEncoder().encode(type), 0, 0, 0, 0);

  return Uint8Array.from(bytes);
}

/** A WebP whose first chunk is VP8X, with the animation flag as given. */
function webp(animated: boolean, width: number, height: number): Uint8Array {
  const bytes = new Uint8Array(30);
  const view = new DataView(bytes.buffer);

  bytes.set(new TextEncoder().encode("RIFF"), 0);
  bytes.set(new TextEncoder().encode("WEBP"), 8);
  bytes.set(new TextEncoder().encode("VP8X"), 12);
  bytes[20] = animated ? 0x02 : 0;
  view.setUint16(24, width - 1, true);
  view.setUint16(27, height - 1, true);

  return bytes;
}

describe("readProfileImage", () => {
  it("reads a still and an animated GIF from their blocks", () => {
    expect(readProfileImage(gif(1))).toEqual({
      format: "gif",
      width: 4,
      height: 3,
      animated: false,
    });
    expect(readProfileImage(gif(2))).toEqual({
      format: "gif",
      width: 4,
      height: 3,
      animated: true,
    });
  });

  it("refuses a GIF whose blocks are broken, and one with no frame", () => {
    const broken = gif(2);

    expect(readProfileImage(broken.subarray(0, broken.length - 6))).toBeNull();
    expect(readProfileImage(gif(0))).toBeNull();
  });

  it("takes an animated PNG as a still one, at its header's size", () => {
    expect(readProfileImage(png(5000, 10, ["acTL"]))).toEqual({
      format: "png",
      width: 5000,
      height: 10,
      animated: false,
    });
  });

  it("reads WebP's animation flag and size", () => {
    expect(readProfileImage(webp(true, 320, 180))).toEqual({
      format: "webp",
      width: 320,
      height: 180,
      animated: true,
    });
    expect(readProfileImage(webp(false, 64, 64))?.animated).toBe(false);
  });

  it("refuses bytes that aren't an image, whatever they claim", () => {
    expect(readProfileImage(new TextEncoder().encode("not really a png"))).toBeNull();
    expect(readProfileImage(new Uint8Array())).toBeNull();
  });
});
