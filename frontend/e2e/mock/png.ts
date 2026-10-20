/* Hand-built PNGs for the mock e2e specs: real files a browser decodes, small enough to inline. */
import { crc32, deflateSync } from "node:zlib";

/** One PNG chunk: length, type, data and the CRC over type and data. */
export function chunk(type: string, data: Buffer): Buffer {
  const length = Buffer.alloc(4);
  const crc = Buffer.alloc(4);
  const body = Buffer.concat([Buffer.from(type, "latin1"), data]);

  length.writeUInt32BE(data.length);
  crc.writeUInt32BE(crc32(body));

  return Buffer.concat([length, body, crc]);
}

export type Rgb = readonly [number, number, number];

export const PNG_SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

/** An RGB PNG's IHDR chunk. */
export function pngHeader(width: number, height: number): Buffer {
  const header = Buffer.alloc(13);

  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header.set([8, 2, 0, 0, 0], 8);

  return chunk("IHDR", header);
}

/** A PNG's compressed scanlines, shading diagonally from one colour to another. */
export function gradientRows(width: number, height: number, from: Rgb, to: Rgb): Buffer {
  const rows = Buffer.alloc((width * 3 + 1) * height);

  for (let y = 0; y < height; y += 1) {
    const row = y * (width * 3 + 1);

    for (let x = 0; x < width; x += 1) {
      const t = (x / width + y / height) / 2;

      for (let channel = 0; channel < 3; channel += 1) {
        const start = from[channel] ?? 0;
        const end = to[channel] ?? 0;

        rows[row + 1 + x * 3 + channel] = Math.round(start + (end - start) * t);
      }
    }
  }

  return deflateSync(rows);
}

/** A `width` × `height` PNG shading diagonally from one colour to another, for a logo or banner. */
export function gradientPng(width: number, height: number, from: Rgb, to: Rgb): Buffer {
  return Buffer.concat([
    PNG_SIGNATURE,
    pngHeader(width, height),
    chunk("IDAT", gradientRows(width, height, from, to)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/** A `width` × `height` RGBA PNG whose pixel at (x, y) is `paint(x, y)`, for transparent logos. */
export function rgbaPng(
  width: number,
  height: number,
  paint: (x: number, y: number) => readonly [number, number, number, number],
): Buffer {
  const header = Buffer.alloc(13);
  const rows = Buffer.alloc((width * 4 + 1) * height);

  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header.set([8, 6, 0, 0, 0], 8);

  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      rows.set(paint(x, y), y * (width * 4 + 1) + 1 + x * 4);
    }
  }

  return Buffer.concat([
    PNG_SIGNATURE,
    chunk("IHDR", header),
    chunk("IDAT", deflateSync(rows)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/** An APNG frame control chunk: the whole canvas, shown for a tenth of a second. */
function frameControl(sequence: number, width: number, height: number): Buffer {
  const data = Buffer.alloc(26);

  data.writeUInt32BE(sequence, 0);
  data.writeUInt32BE(width, 4);
  data.writeUInt32BE(height, 8);
  data.writeUInt16BE(1, 20);
  data.writeUInt16BE(10, 22);

  return chunk("fcTL", data);
}

/** A two-frame animated PNG, which the server treats as a still PNG. */
export function twoFrameApng(width: number, height: number): Buffer {
  const control = Buffer.alloc(8);

  control.writeUInt32BE(2, 0);

  const sequence = Buffer.alloc(4);

  sequence.writeUInt32BE(2, 0);

  return Buffer.concat([
    PNG_SIGNATURE,
    pngHeader(width, height),
    chunk("acTL", control),
    frameControl(0, width, height),
    chunk("IDAT", gradientRows(width, height, [255, 122, 69], [168, 38, 120])),
    frameControl(1, width, height),
    chunk(
      "fdAT",
      Buffer.concat([sequence, gradientRows(width, height, [39, 92, 196], [24, 180, 160])]),
    ),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/**
 * A real two-frame looping GIF, each frame one solid colour. Its LZW stream clears before every
 * pixel, so codes stay 3 bits wide and need no table.
 */
export function twoFrameGif(width: number, height: number): Buffer {
  const screen = Buffer.alloc(7);

  screen.writeUInt16LE(width, 0);
  screen.writeUInt16LE(height, 2);
  screen[4] = 0xf0;

  const frame = (index: number): Buffer => {
    const codes: number[] = [];

    for (let pixel = 0; pixel < width * height; pixel += 1) codes.push(4, index);

    codes.push(5);

    const packed: number[] = [];
    let bits = 0;
    let filled = 0;

    for (const code of codes) {
      bits |= code << filled;
      filled += 3;

      while (filled >= 8) {
        packed.push(bits & 0xff);
        bits >>= 8;
        filled -= 8;
      }
    }

    if (filled > 0) packed.push(bits & 0xff);

    const blocks: number[] = [];

    for (let at = 0; at < packed.length; at += 255) {
      const block = packed.slice(at, at + 255);

      blocks.push(block.length, ...block);
    }

    const descriptor = Buffer.alloc(10);

    descriptor[0] = 0x2c;
    descriptor.writeUInt16LE(width, 5);
    descriptor.writeUInt16LE(height, 7);

    return Buffer.concat([
      Buffer.from([0x21, 0xf9, 0x04, 0x00, 10, 0, 0, 0]),
      descriptor,
      Buffer.from([2, ...blocks, 0]),
    ]);
  };

  return Buffer.concat([
    Buffer.from("GIF89a", "latin1"),
    screen,
    Buffer.from([255, 122, 69, 39, 92, 196]),
    Buffer.from([0x21, 0xff, 0x0b, ...Buffer.from("NETSCAPE2.0", "latin1"), 3, 1, 0, 0, 0]),
    frame(0),
    frame(1),
    Buffer.from([0x3b]),
  ]);
}
