import { describe, expect, it } from "vitest";
import { focusAdjacentRow, type KeyPress, nextRowIndex, rowCommand } from "./keyboard.ts";

const press = (key: string, change: Partial<KeyPress> = {}): KeyPress => ({
  key,
  shiftKey: false,
  altKey: false,
  ctrlKey: false,
  metaKey: false,
  ...change,
});

describe("row keys", () => {
  it("maps the message shortcuts", () => {
    expect(rowCommand(press("e"))).toBe("edit");
    expect(rowCommand(press("R"))).toBe("react");
    expect(rowCommand(press("q"))).toBe("reply");
    expect(rowCommand(press("t"))).toBe("thread");
    expect(rowCommand(press("p"))).toBe("pin");
    expect(rowCommand(press("s"))).toBe("save");
    expect(rowCommand(press("f"))).toBe("forward");
    expect(rowCommand(press("l"))).toBe("link");
    expect(rowCommand(press("Delete"))).toBe("delete");
    expect(rowCommand(press("Backspace"))).toBe("delete");
    expect(rowCommand(press("Escape"))).toBe("composer");
  });

  it("moves with the arrows, Home/End and j/k", () => {
    expect(rowCommand(press("ArrowUp"))).toBe("up");
    expect(rowCommand(press("ArrowDown"))).toBe("down");
    expect(rowCommand(press("Home"))).toBe("first");
    expect(rowCommand(press("End"))).toBe("last");
    expect(rowCommand(press("k"))).toBe("up");
    expect(rowCommand(press("j"))).toBe("down");
  });

  it("opens the menu with Shift+F10 and the menu key", () => {
    expect(rowCommand(press("F10", { shiftKey: true }))).toBe("menu");
    expect(rowCommand(press("ContextMenu"))).toBe("menu");
    expect(rowCommand(press("F10"))).toBeNull();
  });

  it("lets app chords and shifted letters through", () => {
    expect(rowCommand(press("ArrowUp", { altKey: true }))).toBeNull();
    expect(rowCommand(press("k", { metaKey: true }))).toBeNull();
    expect(rowCommand(press("e", { ctrlKey: true }))).toBeNull();
    expect(rowCommand(press("E", { shiftKey: true }))).toBeNull();
    expect(rowCommand(press("+", { shiftKey: true }))).toBe("react");
    expect(rowCommand(press("x"))).toBeNull();
  });
});

describe("roving focus", () => {
  it("steps within the list and stops at the ends", () => {
    expect(nextRowIndex(2, 5, "up")).toBe(1);
    expect(nextRowIndex(0, 5, "up")).toBeNull();
    expect(nextRowIndex(4, 5, "down")).toBeNull();
    expect(nextRowIndex(2, 5, "first")).toBe(0);
    expect(nextRowIndex(2, 5, "last")).toBe(4);
    expect(nextRowIndex(0, 0, "down")).toBeNull();
    expect(nextRowIndex(1, 5, "edit")).toBeNull();
  });

  it("moves focus between the rows of a log", () => {
    document.body.innerHTML = `
      <div role="log">
        <article data-message-row tabindex="-1" id="a"></article>
        <div><article data-message-row tabindex="-1" id="b"></article></div>
        <article data-message-row tabindex="-1" id="c"></article>
      </div>`;

    const rows = [...document.querySelectorAll<HTMLElement>("[data-message-row]")];
    const [first, second, third] = rows;

    Element.prototype.scrollIntoView = () => undefined;

    expect(first === undefined || second === undefined || third === undefined).toBe(false);
    expect(focusAdjacentRow(second ?? document.body, "up")).toBe(true);
    expect(document.activeElement).toBe(first);
    expect(focusAdjacentRow(first ?? document.body, "up")).toBe(false);
    expect(focusAdjacentRow(first ?? document.body, "last")).toBe(true);
    expect(document.activeElement).toBe(third);
  });
});
