import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  type KeyboardMemory,
  keyboardMemory,
  readKeyboard,
  useKeyboardInset,
  type ViewportSample,
} from "./keyboard-inset.ts";

/** A 360 x 740 touch phone in portrait, no keyboard, nothing focused. */
const PHONE: ViewportSample = {
  innerHeight: 740,
  viewportHeight: 740,
  offsetTop: 0,
  scale: 1,
  coarse: true,
  orientation: "portrait",
  editing: false,
};

function read(memory: KeyboardMemory, change: Partial<ViewportSample>) {
  return readKeyboard({ ...PHONE, ...change }, memory);
}

describe("readKeyboard", () => {
  it("reads an overlaid keyboard as the strip the visual viewport leaves uncovered", () => {
    const memory = keyboardMemory();

    expect(read(memory, {})).toEqual({ top: 0, bottom: 0, open: false });
    expect(read(memory, { viewportHeight: 440, editing: true })).toEqual({
      top: 0,
      bottom: 300,
      open: true,
    });
  });

  it("splits a panned page into the strip above the visible area and the one below", () => {
    const memory = keyboardMemory();

    read(memory, {});

    expect(read(memory, { viewportHeight: 440, offsetTop: 200, editing: true })).toEqual({
      top: 200,
      bottom: 100,
      open: true,
    });
  });

  it("takes pinch zoom for no keyboard", () => {
    const memory = keyboardMemory();

    read(memory, {});

    expect(read(memory, { viewportHeight: 370, offsetTop: 120, scale: 2 })).toEqual({
      top: 0,
      bottom: 0,
      open: false,
    });
  });

  it("reads a resizing keyboard as the page dropping below its full height", () => {
    const memory = keyboardMemory();

    read(memory, {});

    expect(read(memory, { innerHeight: 420, viewportHeight: 420, editing: true }).open).toBe(true);
    expect(read(memory, {}).open).toBe(false);
  });

  it("keeps a resizing keyboard open through a rotation, then learns the new full height", () => {
    const memory = keyboardMemory();

    read(memory, {});
    read(memory, { innerHeight: 420, viewportHeight: 420, editing: true });

    // Landscape with the keyboard still up: the short height must not become the baseline.
    const landscape = { orientation: "landscape", innerHeight: 160, viewportHeight: 160 } as const;

    expect(read(memory, { ...landscape, editing: true }).open).toBe(true);
    expect(read(memory, { ...landscape, editing: true }).open).toBe(true);

    // The keyboard closes: the full landscape height is learned, and a later keyboard reads open.
    expect(
      read(memory, { orientation: "landscape", innerHeight: 340, viewportHeight: 340 }).open,
    ).toBe(false);
    expect(read(memory, { ...landscape, editing: true }).open).toBe(true);
    expect(memory.baselines.get("landscape")).toBe(340);
    expect(memory.baselines.get("portrait")).toBe(740);
  });

  it("doesn't take a shorter desktop window for a keyboard", () => {
    const memory = keyboardMemory();

    read(memory, { coarse: false, innerHeight: 900, viewportHeight: 900 });

    expect(read(memory, { coarse: false, innerHeight: 500, viewportHeight: 500 }).open).toBe(false);
  });
});

/** A visualViewport stand-in whose readings a test sets. */
class FakeViewport extends EventTarget {
  height = window.innerHeight;
  offsetTop = 0;
  scale = 1;
}

describe("useKeyboardInset", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("publishes the insets on <html>, a frame after the viewport moves, and clears them", () => {
    const viewport = new FakeViewport();
    const frames: FrameRequestCallback[] = [];

    vi.stubGlobal("visualViewport", viewport);
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) =>
      frames.push(callback),
    );
    vi.stubGlobal("cancelAnimationFrame", () => undefined);
    vi.stubGlobal("matchMedia", () => ({ matches: true }));

    const root = document.documentElement;
    const hook = renderHook(() => useKeyboardInset());

    expect(root.style.getPropertyValue("--keyboard-inset")).toBe("0px");

    viewport.height = window.innerHeight - 300;
    viewport.offsetTop = 120;
    viewport.dispatchEvent(new Event("resize"));
    viewport.dispatchEvent(new Event("scroll"));

    // Batched: two events, one frame.
    expect(frames).toHaveLength(1);
    act(() => frames[0]?.(0));
    expect(root.style.getPropertyValue("--keyboard-inset")).toBe("180px");
    expect(root.style.getPropertyValue("--viewport-top-inset")).toBe("120px");
    expect(root.dataset.keyboard).toBe("open");

    hook.unmount();
    expect(root.style.getPropertyValue("--keyboard-inset")).toBe("");
    expect(root.style.getPropertyValue("--viewport-top-inset")).toBe("");
    expect(root.dataset.keyboard).toBeUndefined();
  });
});
