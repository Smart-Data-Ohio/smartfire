import { useEffect } from "react";
import { COARSE_QUERY } from "./breakpoints.ts";

/** Under this, a change in the viewport's height is browser chrome folding away, not a keyboard. */
const KEYBOARD_MIN_HEIGHT = 120;

export type Orientation = "portrait" | "landscape";

/** One reading of the viewports, in layout px. */
export interface ViewportSample {
  readonly innerHeight: number;
  readonly viewportHeight: number;
  readonly offsetTop: number;
  readonly scale: number;
  readonly coarse: boolean;
  readonly orientation: Orientation;
  /** A text field has focus, so a keyboard may be up and the height isn't a baseline. */
  readonly editing: boolean;
}

/** What the readings so far have learned: the full height per orientation, and the last state. */
export interface KeyboardMemory {
  readonly baselines: Map<Orientation, number>;
  open: boolean;
}

export interface KeyboardState {
  /** The layout viewport's strip above the visible area (iOS pans the page up to the field). */
  readonly top: number;
  /** The strip below it that an overlaid keyboard covers. */
  readonly bottom: number;
  readonly open: boolean;
}

export function keyboardMemory(): KeyboardMemory {
  return { baselines: new Map(), open: false };
}

/**
 * Reads the keyboard from one sample. An overlaid keyboard (iOS) shows as the visual viewport
 * falling short of the layout viewport; a resizing one (Android, `interactive-widget=
 * resizes-content`) as the layout viewport dropping below this orientation's full height, which is
 * learned only while no field has focus, so a rotation with the keyboard up can't set a short one.
 * Until an orientation has a baseline, the keyboard stays as it was. Pinch zoom shrinks the visual
 * viewport too, so a zoomed page has no overlaid keyboard.
 */
export function readKeyboard(sample: ViewportSample, memory: KeyboardMemory): KeyboardState {
  const zoomed = Math.abs(sample.scale - 1) > 0.01;
  const covered = zoomed ? 0 : Math.max(0, sample.innerHeight - sample.viewportHeight);
  const top = Math.min(covered, Math.max(0, sample.offsetTop));
  const baseline = memory.baselines.get(sample.orientation);

  if (!sample.editing) {
    memory.baselines.set(sample.orientation, Math.max(baseline ?? 0, sample.innerHeight));
  }

  if (covered >= KEYBOARD_MIN_HEIGHT) {
    memory.open = true;
  } else if (!sample.coarse) {
    // A desktop window made shorter is not a keyboard.
    memory.open = false;
  } else if (baseline === undefined) {
    memory.open = sample.editing && memory.open;
  } else {
    memory.open = baseline - sample.innerHeight >= KEYBOARD_MIN_HEIGHT;
  }

  return { top, bottom: covered - top, open: memory.open };
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

function orientation(): Orientation {
  const type = window.screen.orientation?.type;

  if (type !== undefined) {
    return type.startsWith("landscape") ? "landscape" : "portrait";
  }

  return window.screen.width > window.screen.height ? "landscape" : "portrait";
}

/**
 * The on-screen keyboard, for the whole app; the shell runs it once, batched to a frame. On <html>:
 * `--keyboard-inset`, the layout viewport's strip under an overlaid keyboard (iOS), and
 * `--viewport-top-inset`, its strip above the visible area when iOS pans the page up to a field.
 * The shell sits between the two, so its header stays on screen; a position: fixed surface adds
 * them itself. While any keyboard is up, overlaid or resizing, <html> carries
 * `data-keyboard="open"`.
 */
export function useKeyboardInset(): void {
  useEffect(() => {
    const viewport = window.visualViewport;

    if (viewport === null || viewport === undefined) {
      return;
    }

    const root = document.documentElement;
    const coarse = window.matchMedia(COARSE_QUERY);
    const memory = keyboardMemory();
    let frame = 0;

    const update = () => {
      frame = 0;

      const state = readKeyboard(
        {
          innerHeight: window.innerHeight,
          viewportHeight: viewport.height,
          offsetTop: viewport.offsetTop,
          scale: viewport.scale,
          coarse: coarse.matches,
          orientation: orientation(),
          editing: editing(),
        },
        memory,
      );

      root.style.setProperty("--keyboard-inset", `${Math.round(state.bottom)}px`);
      root.style.setProperty("--viewport-top-inset", `${Math.round(state.top)}px`);

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

    return () => {
      cancelAnimationFrame(frame);
      viewport.removeEventListener("resize", schedule);
      viewport.removeEventListener("scroll", schedule);
      window.removeEventListener("resize", schedule);
      root.style.removeProperty("--keyboard-inset");
      root.style.removeProperty("--viewport-top-inset");
      delete root.dataset.keyboard;
    };
  }, []);
}
