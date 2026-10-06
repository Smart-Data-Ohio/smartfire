import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { Md5, md5Base64, toBase64, toHex } from "./md5.ts";

const encoder = new TextEncoder();

function hexOf(text: string): string {
  return toHex(new Md5().update(encoder.encode(text)).digest());
}

/** Deterministic bytes (a small LCG), so a failure reproduces. */
function bytes(length: number, seed = 7): Uint8Array {
  const out = new Uint8Array(length);
  let state = seed;

  for (let index = 0; index < length; index += 1) {
    state = (state * 1103515245 + 12345) >>> 0;
    out[index] = state >>> 24;
  }

  return out;
}

describe("Md5", () => {
  it("matches the RFC 1321 test suite", () => {
    expect(hexOf("")).toBe("d41d8cd98f00b204e9800998ecf8427e");
    expect(hexOf("a")).toBe("0cc175b9c0f1b6a831c399e269772661");
    expect(hexOf("abc")).toBe("900150983cd24fb0d6963f7d28e17f72");
    expect(hexOf("message digest")).toBe("f96b697d7cb7938d525a2f31aaf161d0");
    expect(hexOf("abcdefghijklmnopqrstuvwxyz")).toBe("c3fcd3d76192e4007dfb496cca67e13b");
    expect(hexOf("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789")).toBe(
      "d174ab98d277d9f5a5611c2c9f419d9f",
    );
    expect(hexOf("1234567890".repeat(8))).toBe("57edf4a22be3c955ac49da2e2107b67a");
  });

  it("handles the padding edges around 55, 56 and 64 bytes", () => {
    for (const length of [55, 56, 57, 63, 64, 65, 119, 120, 128]) {
      const data = bytes(length, length);

      expect(toHex(new Md5().update(data).digest())).toBe(
        createHash("md5").update(data).digest("hex"),
      );
    }
  });

  it("gives the same digest however the input is chunked", () => {
    const data = bytes(300_001);
    const expected = createHash("md5").update(data).digest("hex");

    for (const size of [1, 3, 63, 64, 65, 4096, 65_537]) {
      const hash = new Md5();

      for (let offset = 0; offset < data.length; offset += size) {
        hash.update(data.subarray(offset, offset + size));
      }

      expect(toHex(hash.digest())).toBe(expected);
    }
  });

  it("accepts ArrayBuffers and views into larger buffers", () => {
    const data = bytes(1000);
    const view = new Uint8Array(data.buffer, 10, 500);

    expect(toHex(new Md5().update(view).digest())).toBe(
      createHash("md5").update(view).digest("hex"),
    );
    expect(toHex(new Md5().update(data.slice().buffer).digest())).toBe(
      createHash("md5").update(data).digest("hex"),
    );
  });
});

describe("md5Base64", () => {
  it("hashes a Blob in chunks to the base64 Active Storage expects", async () => {
    const data = bytes(10_000);
    const expected = createHash("md5").update(data).digest("base64");

    await expect(md5Base64(new Blob([data.slice()]), undefined, 777)).resolves.toBe(expected);
    await expect(md5Base64(new Blob([]))).resolves.toBe("1B2M2Y8AsgTpgAmY7PhCfg==");
  });

  it("stops when aborted", async () => {
    const controller = new AbortController();

    controller.abort();

    await expect(md5Base64(new Blob(["x"]), controller.signal)).rejects.toThrow();
  });

  it("encodes base64 like btoa over the raw bytes", () => {
    expect(toBase64(Uint8Array.of(0, 255, 16))).toBe("AP8Q");
  });
});
