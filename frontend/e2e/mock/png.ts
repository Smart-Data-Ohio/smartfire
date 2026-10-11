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
