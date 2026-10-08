import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { readKeyboard, useKeyboardInset, type ViewportSample } from "./keyboard-inset.ts";

/** A 360 x 740 phone in portrait, no keyboard, a text field focused. */
const PHONE: ViewportSample = {
  layoutHeight: 740,
  viewportHeight: 740,
  offsetTop: 0,
  scale: 1,
  editing: true,
};

function read(change: Partial<ViewportSample>) {
  return readKeyboard({ ...PHONE, ...change });
}

const CLOSED = { top: 0, bottom: 0, open: false };

describe("readKeyboard", () => {
  it("reads an overlaid keyboard as the strip the visual viewport leaves uncovered", () => {
    expect(read({})).toEqual(CLOSED);
    expect(read({ viewportHeight: 440 })).toEqual({ top: 0, bottom: 300, open: true });
  });

  it("splits a panned page into the strip above the visible area and the one below", () => {
    expect(read({ viewportHeight: 440, offsetTop: 200 })).toEqual({
      top: 200,
      bottom: 100,
      open: true,
    });
  });

  it("takes pinch zoom for no keyboard, though WebKit halves innerHeight with it", () => {
    // innerHeight isn't read: the layout height is the ICB, which pinch leaves at 740.
    expect(read({ viewportHeight: 370, offsetTop: 120, scale: 2 })).toEqual(CLOSED);
  });

  it("closes once the visual viewport grows back, whatever came before", () => {
    // Rotated to landscape with the keyboard up, then dismissed with Back, focus kept.
    const landscape = { layoutHeight: 340 } as const;

    expect(read({ ...landscape, viewportHeight: 160 }).open).toBe(true);
    expect(read({ ...landscape, viewportHeight: 340 })).toEqual(CLOSED);
  });

  it("doesn't take a folding URL bar for a keyboard", () => {
    expect(read({ viewportHeight: 680 })).toEqual(CLOSED);
  });

  it("doesn't take a focused field with a hardware keyboard for an open one", () => {
    // An iPad with a keyboard attached: focus, and nothing covers the page.
    expect(read({ layoutHeight: 1024, viewportHeight: 1024 })).toEqual(CLOSED);
  });

  it("needs a focused field", () => {
    expect(read({ viewportHeight: 440, editing: false })).toEqual(CLOSED);
  });

  it("finds nothing to inset when the keyboard resizes the layout", () => {
    // Android with interactive-widget=resizes-content: both heights drop together.
    expect(read({ layoutHeight: 420, viewportHeight: 420 })).toEqual(CLOSED);
  });

  it("doesn't take a shorter or zoomed desktop window for a keyboard", () => {
    expect(read({ layoutHeight: 500, viewportHeight: 500 })).toEqual(CLOSED);
    // Page zoom shrinks both heights in CSS px and leaves the visual viewport's scale at 1.
    expect(read({ layoutHeight: 450, viewportHeight: 450 })).toEqual(CLOSED);
  });
});

/** A visualViewport stand-in whose readings a test sets. */
class FakeViewport extends EventTarget {
  height = 740;
  offsetTop = 0;
  scale = 1;
}

describe("useKeyboardInset", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    Reflect.deleteProperty(document.documentElement, "clientHeight");
    document.body.replaceChildren();
  });

  it("publishes the insets on <html>, a frame after the viewport moves, and clears them", () => {
    const viewport = new FakeViewport();
    const frames: FrameRequestCallback[] = [];

    vi.stubGlobal("visualViewport", viewport);
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) =>
      frames.push(callback),
    );
    vi.stubGlobal("cancelAnimationFrame", () => undefined);

    const root = document.documentElement;
    const field = document.createElement("textarea");

    // jsdom lays nothing out, so the ICB is given.
    Object.defineProperty(root, "clientHeight", { configurable: true, value: 740 });
    document.body.append(field);
    field.focus();

    const hook = renderHook(() => useKeyboardInset());

    expect(root.style.getPropertyValue("--keyboard-inset")).toBe("0px");

    viewport.height = 440;
    viewport.offsetTop = 120;
    viewport.dispatchEvent(new Event("resize"));
    viewport.dispatchEvent(new Event("scroll"));

    // Batched: two events, one frame.
    expect(frames).toHaveLength(1);
    act(() => frames[0]?.(0));
    expect(root.style.getPropertyValue("--keyboard-inset")).toBe("180px");
    expect(root.style.getPropertyValue("--viewport-top-inset")).toBe("120px");
    expect(root.dataset.keyboard).toBe("open");

    // Leaving the field closes it with nothing else changing.
    frames.length = 0;
    field.blur();
    act(() => frames[0]?.(0));
    expect(root.style.getPropertyValue("--keyboard-inset")).toBe("0px");
    expect(root.dataset.keyboard).toBeUndefined();

    hook.unmount();
    expect(root.style.getPropertyValue("--keyboard-inset")).toBe("");
    expect(root.style.getPropertyValue("--viewport-top-inset")).toBe("");
    expect(root.dataset.keyboard).toBeUndefined();
  });
});
