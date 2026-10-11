import { describe, expect, it } from "vitest";
import { caseFold, caselessEqual } from "./case-fold.ts";

// The same table as board_catalog_case_folding_pins_the_spa_table (crates/db), which checks it
// against the server's `caseless` crate.
const SERVER_FOLDS: readonly (readonly [string, string])[] = [
  ["Straße", "strasse"],
  ["STRASSE", "strasse"],
  ["\u{1e9e}", "ss"],
  ["İstanbul", "i\u{307}stanbul"],
  ["ISTANBUL", "istanbul"],
  ["ı", "ı"],
  ["ΟΔΟΣ", "οδοσ"],
  ["οδος", "οδοσ"],
  ["ﬁle", "file"],
  ["ﬀ", "ff"],
  ["Ꭰ", "Ꭰ"],
  ["ꭰ", "Ꭰ"],
  ["ǅ", "ǆ"],
  ["\u{212a}", "k"],
  ["µ", "μ"],
  ["Équipe", "équipe"],
];

describe("caseFold", () => {
  it.each(SERVER_FOLDS)("folds %s as the server does", (name, folded) => {
    expect(caseFold(name)).toBe(folded);
  });

  it("matches names the server matches and only those", () => {
    expect(caselessEqual("Straße", "STRASSE")).toBe(true);
    expect(caselessEqual("ﬁle", "FILE")).toBe(true);
    expect(caselessEqual("ΟΔΟΣ", "οδος")).toBe(true);
    expect(caselessEqual("ı", "I")).toBe(false);
    expect(caselessEqual("İ", "i")).toBe(false);
  });
});
