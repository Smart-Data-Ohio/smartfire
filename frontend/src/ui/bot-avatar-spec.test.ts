import { botAvatarPresets, botAvatarTypes } from "bot-avatars";
import { describe, expect, it } from "vitest";
import {
  BOT_AVATAR_TYPES,
  describeSpec,
  fitOf,
  keptSpec,
  ownColor,
  randomSpec,
  sameSpec,
  wearOf,
  withType,
} from "./bot-avatar-spec.ts";

const CAT = { type: "cat" as const, color: "#eaa06f", face: "mouth" as const, accessory: null };

describe("bot avatar specs", () => {
  it("names the library's bodies, in its order, with its own colours", () => {
    expect(BOT_AVATAR_TYPES.map((entry) => entry.type)).toEqual(botAvatarTypes);

    for (const entry of BOT_AVATAR_TYPES) {
      expect(entry.color).toBe(botAvatarPresets[entry.type].color.toLowerCase());
      expect(entry.label).toBe(botAvatarPresets[entry.type].label);
    }
  });

  it("starts a form on any body in its own colour, wearing nothing", () => {
    expect(randomSpec(() => 0)).toEqual({
      type: "clover",
      color: "#41c4ff",
      face: "eyes",
      accessory: null,
    });
    expect(randomSpec(() => 0.99)).toEqual({
      type: "puddle",
      color: "#ee8bdb",
      face: "mouth",
      accessory: null,
    });
  });

  it("swaps a body's own colour for the next body's, but keeps a picked one", () => {
    expect(withType(CAT, "ghost").color).toBe(ownColor("ghost"));
    expect(withType({ ...CAT, color: "#3a3a44" }, "ghost").color).toBe("#3a3a44");
  });

  it("dresses the library's props from one accessory", () => {
    expect(wearOf("crown")).toEqual({
      hat: "crown",
      glasses: "none",
      headphones: false,
      bowTie: false,
    });
    expect(wearOf("roundGlasses").glasses).toBe("round");
    expect(wearOf("shades").glasses).toBe("shades");
    expect(wearOf("bowTie").bowTie).toBe(true);
    expect(wearOf(null)).toEqual({
      hat: "none",
      glasses: "none",
      headphones: false,
      bowTie: false,
    });
  });

  it("draws a hatted toy smaller and lower, so the hat stays in its tile", () => {
    expect(fitOf(null, 44)).toEqual({ scale: 0.8, drop: 0 });
    expect(fitOf("shades", 20)).toEqual({ scale: 0.9, drop: 0 });
    expect(fitOf("crown", 44).scale).toBeLessThan(0.8);
    expect(fitOf("beanie", 44).drop).toBeGreaterThan(0);
  });

  it("reads a picked avatar out, and compares colours in any case", () => {
    expect(describeSpec(CAT)).toBe("Cat in its own colour");
    expect(describeSpec({ ...CAT, color: "#EB7575" })).toBe("Cat in Coral");
    expect(describeSpec({ ...CAT, color: "#eb7575", accessory: "party" })).toBe(
      "Cat in Coral, party hat",
    );
    expect(sameSpec(CAT, { ...CAT, color: "#EAA06F" })).toBe(true);
    expect(sameSpec(CAT, { ...CAT, face: "eyes" })).toBe(false);
    expect(sameSpec(null, null)).toBe(true);
  });

  it("takes back a draft's avatar only when the picker can show it", () => {
    expect(keptSpec(JSON.stringify(CAT))).toEqual(CAT);
    expect(keptSpec("")).toBeNull();
    expect(keptSpec("{")).toBeNull();
    expect(keptSpec(JSON.stringify({ ...CAT, type: "dragon" }))).toBeNull();
    expect(keptSpec(JSON.stringify({ ...CAT, color: "orange" }))).toBeNull();
    expect(keptSpec(JSON.stringify({ ...CAT, accessory: "monocle" }))).toBeNull();
  });
});
