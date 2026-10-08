import { useEffect } from "react";

/** Under this, a gap under the visual viewport is browser chrome folding away, not a keyboard. */
// A heuristic: short-landscape keyboards near this height are unverified on devices.
const KEYBOARD_MIN_HEIGHT = 120;

/** One reading of the viewports, in layout px. */
export interface ViewportSample {
  /**
   * The initial containing block, which neither pinch zoom nor an overlaid keyboard moves. It only
   * decides whether the keyboard is open; it can sit below `100dvh` while browser chrome is
   * retracted, so no geometry comes from it.
   */
  readonly layoutHeight: number;
  /** The layout viewport, which position: fixed surfaces are placed against. */
  readonly innerHeight: number;
  readonly viewportHeight: number;
  readonly offsetTop: number;
  readonly scale: number;
  /** A text field has focus, so the keyboard may be up. */
  readonly editing: boolean;
}

export interface KeyboardState {
  /** The layout viewport's strip above the visible area (iOS pans the page up to the field). */
  readonly top: number;
  /** The visible area's height while the keyboard is open, or undefined for the full `100dvh`. */
  readonly height: number | undefined;
  /** The layout viewport's strip below the visible area, which the keyboard covers. */
  readonly bottom: number;
  readonly open: boolean;
}

/**
 * Reads the keyboard from one sample, with no memory of earlier ones: it's open while a text field
 * has focus, the page isn't pinch-zoomed, and the visual viewport falls a keyboard's height short
 * of the layout. A hardware keyboard, a folding URL bar and a zoomed page all read closed, and the
 * keyboard reads closed the moment the visual viewport grows back, whatever happened before. While
 * it's open, the geometry is the visual viewport itself.
 */
export function readKeyboard(sample: ViewportSample): KeyboardState {
  const gap = sample.layoutHeight - sample.viewportHeight;
  const open = sample.editing && Math.abs(sample.scale - 1) < 0.01 && gap > KEYBOARD_MIN_HEIGHT;

  if (!open) {
    return { top: 0, height: undefined, bottom: 0, open };
  }

  const top = Math.max(0, sample.offsetTop);
  const bottom = Math.max(0, sample.innerHeight - top - sample.viewportHeight);

  return { top, height: sample.viewportHeight, bottom, open };
}

function editing(): boolean {
  const element = document.activeElement;

  return (
    element instanceof HTMLTextAreaElement ||
    (element instanceof HTMLInputElement &&
      !["checkbox", "radio", "range", "color", "button", "submit", "reset"].includes(
        element.type,
      )) ||
    (element instanceof HTMLElement && element.isContentEditable)
  );
}

/**
 * The on-screen keyboard, for the whole app; the shell runs it once, batched to a frame. On <html>,
 * while an overlaid keyboard (iOS) is up: `--viewport-top-inset` and `--viewport-height`, the
 * visible area's top and height, which the shell takes, so its header and composer stay on
 * screen; and `--keyboard-inset`, the layout viewport's strip below the visible area, which a
 * position: fixed surface keeps clear of. While an overlaid keyboard is up, <html> carries `data-keyboard="open"`. A resizing
 * keyboard (Android, `interactive-widget=resizes-content`) shrinks the layout itself, so the page
 * already fits above it and there is nothing to inset.
 */
export function useKeyboardInset(): void {
  useEffect(() => {
    const viewport = window.visualViewport;

    if (viewport === null || viewport === undefined) {
      return;
    }

    const root = document.documentElement;
    let frame = 0;

    const update = () => {
      frame = 0;

      const state = readKeyboard({
        // The ICB: WebKit holds it through the keyboard and pinch (innerHeight moves under pinch,
        // webkit.org/b/245361); Chromium and Gecko shrink it with a resizing keyboard.
        layoutHeight: root.clientHeight,
        innerHeight: window.innerHeight,
        viewportHeight: viewport.height,
        offsetTop: viewport.offsetTop,
        scale: viewport.scale,
        editing: editing(),
      });

      root.style.setProperty("--keyboard-inset", `${Math.round(state.bottom)}px`);
      root.style.setProperty("--viewport-top-inset", `${Math.round(state.top)}px`);

      if (state.height === undefined) {
        root.style.removeProperty("--viewport-height");
      } else {
        root.style.setProperty("--viewport-height", `${Math.round(state.height)}px`);
      }

      if (state.open) {
        root.dataset.keyboard = "open";
      } else {
        delete root.dataset.keyboard;
      }
    };

    const schedule = () => {
      if (frame === 0) {
        frame = requestAnimationFrame(update);
      }
    };

    update();
    viewport.addEventListener("resize", schedule);
    viewport.addEventListener("scroll", schedule);
    window.addEventListener("resize", schedule);
    // Focus moving into or out of a field changes the reading without moving the viewport.
    document.addEventListener("focusin", schedule);
    document.addEventListener("focusout", schedule);

    return () => {
      cancelAnimationFrame(frame);
      viewport.removeEventListener("resize", schedule);
      viewport.removeEventListener("scroll", schedule);
      window.removeEventListener("resize", schedule);
      document.removeEventListener("focusin", schedule);
      document.removeEventListener("focusout", schedule);
      root.style.removeProperty("--keyboard-inset");
      root.style.removeProperty("--viewport-top-inset");
      root.style.removeProperty("--viewport-height");
      delete root.dataset.keyboard;
    };
  }, []);
}
