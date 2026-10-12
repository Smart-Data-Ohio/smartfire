import { describe, expect, it } from "vitest";
import { STICKER_APNG, STICKER_GIF, STICKER_PNG } from "../../../mock/s2/stickers.ts";
import { readImageHeader, stickerFileProblem } from "./sticker-file.ts";

const file = (bytes: Uint8Array<ArrayBuffer>, name: string) => new File([bytes], name);

describe("a sticker file's header", () => {
  it("reads a PNG's, an APNG's and a GIF's format and size", () => {
    expect(readImageHeader(STICKER_PNG)).toEqual({ format: "png", width: 2, height: 3 });
    expect(readImageHeader(STICKER_APNG)).toEqual({ format: "png", width: 2, height: 3 });
    expect(readImageHeader(STICKER_GIF)).toEqual({ format: "gif", width: 2, height: 3 });
  });

  it("knows nothing else", () => {
    expect(
      readImageHeader(new TextEncoder().encode("<svg xmlns='http://www.w3.org/2000/svg'/>")),
    ).toBeNull();
    expect(readImageHeader(STICKER_PNG.subarray(0, 12))).toBeNull();
  });
});

describe("a sticker file's checks before upload", () => {
  it("passes a small PNG, APNG or GIF", async () => {
    expect(await stickerFileProblem(file(STICKER_PNG, "wave.png"))).toBeNull();
    expect(await stickerFileProblem(file(STICKER_APNG, "hello.png"))).toBeNull();
    expect(await stickerFileProblem(file(STICKER_GIF, "dance.gif"))).toBeNull();
  });

  it("refuses a file past 512 KB before reading it", async () => {
    const heavy = file(STICKER_PNG, "heavy.png");

    Object.defineProperty(heavy, "size", { value: 512 * 1024 + 1 });

    expect(await stickerFileProblem(heavy)).toBe("Image must be 512 KB or smaller.");
  });

  it("refuses an image past 320 × 320", async () => {
    const tall = STICKER_GIF.slice();

    new DataView(tall.buffer).setUint16(8, 321, true);

    expect(await stickerFileProblem(file(tall, "tall.gif"))).toBe(
      "Image must be at most 320 × 320 pixels (this one is 2 × 321).",
    );
  });
});
