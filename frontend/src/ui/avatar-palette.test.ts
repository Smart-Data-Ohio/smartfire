// @vitest-environment node

import { describe, expect, it } from "vitest";
import { AVATAR_HUES, avatarHue } from "./avatar-palette.ts";

describe("avatarHue", () => {
  it("is deterministic", () => {
    expect(avatarHue(42)).toBe(avatarHue(42));
    expect(avatarHue("42")).toBe(avatarHue(42));
    expect(avatarHue("a1b2c3")).toBe(avatarHue("a1b2c3"));
  });

  it("never gives consecutive numeric ids the same hue", () => {
    for (let id = 1; id < 2000; id += 1) {
      expect(avatarHue(id), `ids ${id} and ${id + 1}`).not.toBe(avatarHue(id + 1));
    }
  });

  it("uses every hue for a run of ids", () => {
    const used = new Set(Array.from({ length: AVATAR_HUES.length }, (_, id) => avatarHue(id + 1)));

    expect(used.size).toBe(AVATAR_HUES.length);
  });

  it("spreads string keys across the palette", () => {
    const counts = new Map<number, number>();

    for (let index = 0; index < 4000; index += 1) {
      const hue = avatarHue(`user-${index}`);

      counts.set(hue, (counts.get(hue) ?? 0) + 1);
    }

    const expected = 4000 / AVATAR_HUES.length;

    expect(counts.size).toBe(AVATAR_HUES.length);

    for (const count of counts.values()) {
      expect(Math.abs(count - expected) / expected).toBeLessThan(0.15);
    }
  });

  it("keeps violet for agents", () => {
    for (const hue of AVATAR_HUES) {
      expect(hue >= 260 && hue <= 330, `hue ${hue}`).toBe(false);
    }
  });
});
