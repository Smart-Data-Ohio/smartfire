import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { bindReaderInput, duringAppFocus, readerMovedFocus } from "./reader-focus.ts";

describe("readerMovedFocus", () => {
  let release: (() => void) | null = null;

  beforeEach(() => {
    release = bindReaderInput(document);
  });

  afterEach(() => {
    release?.();
    release = null;
    document.body.replaceChildren();
  });

  it("counts Tab, with or without Shift, until another key follows", () => {
    const link = document.createElement("a");

    document.body.append(link);
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", bubbles: true }));
    expect(readerMovedFocus(link)).toBe(true);

    document.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Tab", shiftKey: true, bubbles: true }),
    );
    expect(readerMovedFocus(link)).toBe(true);

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    expect(readerMovedFocus(link)).toBe(false);
  });

  it("ignores Escape, Enter and Space", () => {
    const link = document.createElement("a");

    document.body.append(link);

    for (const key of ["Escape", "Enter", " "]) {
      document.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
      expect(readerMovedFocus(link)).toBe(false);
    }
  });

  it("counts a click on the focused element or an ancestor, not a descendant", () => {
    const row = document.createElement("div");
    const link = document.createElement("a");
    const outside = document.createElement("button");

    row.append(link);
    document.body.append(row, outside);

    row.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
    expect(readerMovedFocus(link)).toBe(true);
    expect(readerMovedFocus(row)).toBe(true);
    expect(readerMovedFocus(outside)).toBe(false);

    link.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
    expect(readerMovedFocus(link)).toBe(true);
    expect(readerMovedFocus(row)).toBe(false);
  });

  it("stays quiet inside duringAppFocus even after Tab", () => {
    const link = document.createElement("a");

    document.body.append(link);
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", bubbles: true }));
    expect(duringAppFocus(() => readerMovedFocus(link))).toBe(false);
    expect(readerMovedFocus(link)).toBe(true);
  });
});
