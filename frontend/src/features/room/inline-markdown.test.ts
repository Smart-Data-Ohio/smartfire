import { describe, expect, it } from "vitest";
import { inlineSegments } from "./inline-markdown.tsx";

describe("inlineSegments", () => {
  it("splits the composer's inline marks out of plain text", () => {
    const segments = inlineSegments("ship **it** with `cargo` ~~now~~ _soon_ *ok*");

    expect(segments.map(({ mark, text }) => [mark, text])).toEqual([
      ["plain", "ship "],
      ["strong", "it"],
      ["plain", " with "],
      ["code", "cargo"],
      ["plain", " "],
      ["strike", "now"],
      ["plain", " "],
      ["em", "soon"],
      ["plain", " "],
      ["em", "ok"],
    ]);
    expect(segments.map((segment) => segment.start)).toEqual([
      0, 5, 11, 17, 24, 25, 32, 33, 39, 40,
    ]);
  });

  it("leaves unmatched markers and line breaks as typed", () => {
    expect(inlineSegments("2 * 3 = 6\n**half")).toEqual([
      { mark: "plain", text: "2 * 3 = 6\n**half", start: 0 },
    ]);
  });

  it("keeps markup as text", () => {
    expect(inlineSegments("<b>x</b>")).toEqual([{ mark: "plain", text: "<b>x</b>", start: 0 }]);
  });
});
