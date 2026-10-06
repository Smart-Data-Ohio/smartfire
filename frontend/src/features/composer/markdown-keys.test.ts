import { describe, expect, it } from "vitest";
import { insertLink, markerForChord, toggleWrap } from "./markdown-keys.ts";

describe("toggleWrap", () => {
  it("wraps the selection and keeps it selected", () => {
    expect(toggleWrap({ value: "say hi now", start: 4, end: 6 }, "**")).toEqual({
      value: "say **hi** now",
      start: 6,
      end: 8,
    });
  });

  it("unwraps a selection that sits inside markers", () => {
    expect(toggleWrap({ value: "say **hi** now", start: 6, end: 8 }, "**")).toEqual({
      value: "say hi now",
      start: 4,
      end: 6,
    });
  });

  it("unwraps a selection that includes its markers", () => {
    expect(toggleWrap({ value: "`x`", start: 0, end: 3 }, "`")).toEqual({
      value: "x",
      start: 0,
      end: 1,
    });
  });

  it("inserts an empty pair at a caret", () => {
    expect(toggleWrap({ value: "ab", start: 1, end: 1 }, "_")).toEqual({
      value: "a__b",
      start: 2,
      end: 2,
    });
  });
});

describe("insertLink", () => {
  it("selects the url placeholder after the label", () => {
    const edit = insertLink({ value: "see docs", start: 4, end: 8 });

    expect(edit.value).toBe("see [docs](url)");
    expect(edit.value.slice(edit.start, edit.end)).toBe("url");
  });

  it("uses a selected URL as the target", () => {
    expect(insertLink({ value: "https://a.example", start: 0, end: 17 })).toEqual({
      value: "[](https://a.example)",
      start: 1,
      end: 1,
    });
  });
});

describe("markerForChord", () => {
  it("maps the chords", () => {
    expect(markerForChord("B", false)).toBe("**");
    expect(markerForChord("x", true)).toBe("~~");
    expect(markerForChord("u", true)).toBe("link");
    expect(markerForChord("k", false)).toBeNull();
  });
});
