/**
 * The per-frame half of the input-clear-dissolve recipe (recipes/input-clear-dissolve.css, from
 * transitions.dev; see that file's header): the cleared text flies down, blurs and fades while
 * a soft streak lights under each word and the placeholder falls in from above. Timing and
 * distances come from the recipe's tunables, which point at the motion tokens, so reduced motion
 * is already folded in; the glow is skipped outright then.
 */
import { resolvedTheme } from "../lib/appearance.ts";
import { parseTimeMs } from "./durations.ts";
import { prefersReducedMotion } from "./reduced-motion.ts";

/** The recipe's layers inside one `.t-clear` wrap. */
export interface ClearParts {
  readonly wrap: HTMLElement;
  readonly input: HTMLInputElement;
  readonly mirror: HTMLElement;
  readonly placeholder: HTMLElement;
  readonly glow: HTMLElement;
}

/** A sampler for `cubic-bezier(x1, y1, x2, y2)`, so the frames ease like the CSS would. */
export function bezier(value: string): (t: number) => number {
  const match =
    /cubic-bezier\(\s*([-\d.]+)\s*,\s*([-\d.]+)\s*,\s*([-\d.]+)\s*,\s*([-\d.]+)\s*\)/u.exec(value);

  if (match === null) {
    return (t) => t;
  }

  const [x1 = 0, y1 = 0, x2 = 1, y2 = 1] = match.slice(1).map(Number);
  const cx = 3 * x1;
  const bx = 3 * (x2 - x1) - cx;
  const ax = 1 - cx - bx;
  const cy = 3 * y1;
  const by = 3 * (y2 - y1) - cy;
  const ay = 1 - cy - by;

  return (t) => {
    if (t <= 0) return 0;

    if (t >= 1) return 1;

    let s = t;

    for (let index = 0; index < 8; index += 1) {
      const dx = ((ax * s + bx) * s + cx) * s - t;
      const slope = (3 * ax * s + 2 * bx) * s + cx;

      if (Math.abs(dx) < 1e-6 || slope === 0) break;

      s -= dx / slope;
    }

    return ((ay * s + by) * s + cy) * s;
  };
}

/** One radial-gradient streak stack per word, measured in the input's font. */
function streaks(parts: ClearParts, text: string, spread: number): string {
  const context = document.createElement("canvas").getContext("2d");

  if (context === null) {
    return "";
  }

  const style = getComputedStyle(parts.input);
  const rgb = resolvedTheme() === "dark" ? "255,255,255" : "0,0,0";
  const width = parts.wrap.clientWidth || 280;
  const left = parts.input.offsetLeft + (Number.parseFloat(style.paddingLeft) || 0);
  const layers: string[] = [];
  let x = 0;

  context.font = style.font;

  for (const piece of text.split(/(\s+)/u)) {
    const pieceWidth = context.measureText(piece).width;

    if (piece.trim() !== "") {
      const center = left + x + pieceWidth / 2;
      const half = Math.max(pieceWidth * 0.45, 8) * spread;

      const blobs = [
        [0, 0.8, 7, 0.22],
        [half * 0.45, 0.55, 8, 0.18],
        [-half * 0.4, 0.65, 6, 0.16],
        [half * 0.15, 0.9, 5, 0.14],
      ] as const;

      for (const [dx, widthScale, height, alpha] of blobs) {
        const at = (((center + dx) / width) * 100).toFixed(2);
        const radius = Math.max(half * widthScale, 2).toFixed(1);

        layers.push(
          `radial-gradient(ellipse ${radius}px ${height}px at ${at}% 100%, rgba(${rgb},${alpha}), transparent)`,
        );
      }
    }

    x += pieceWidth;
  }

  return layers.join(", ");
}

/**
 * Plays the clear over `text` (the value just removed from the input). Returns a function that
 * stops it early and resets the layers.
 */
export function playClearDissolve(parts: ClearParts, text: string): () => void {
  const { wrap, mirror, placeholder, glow } = parts;
  const style = getComputedStyle(document.documentElement);
  const ms = (name: string) => parseTimeMs(style.getPropertyValue(name));

  const number = (name: string, fallback: number) => {
    const value = Number.parseFloat(style.getPropertyValue(name));

    return Number.isFinite(value) ? value : fallback;
  };

  const total = ms("--clear-dur");
  const outDuration = Math.max(ms("--clear-out-dur"), 1);
  const inDuration = Math.max(ms("--clear-in-dur"), 1);
  const outFly = number("--clear-out-fly", 12);
  const inFly = number("--clear-in-fly", 12);
  const blur = number("--clear-blur", 2);
  const glowDelay = ms("--glow-delay");
  const peakAt = number("--glow-peak-at", 0.15);
  const glowOpacity = prefersReducedMotion() ? 0 : number("--glow-opacity", 0.42);
  const easeOut = bezier(style.getPropertyValue("--clear-out-ease"));
  const easeIn = bezier(style.getPropertyValue("--clear-in-ease"));
  let frame = 0;

  mirror.textContent = text.replace(/ /gu, " ");
  wrap.classList.add("is-clearing");
  glow.style.background =
    glowOpacity === 0 ? "" : streaks(parts, text, number("--glow-spread", 1.5));
  glow.style.opacity = "0";
  placeholder.style.transform = `translateY(${-inFly}px)`;
  placeholder.style.opacity = "0.9";
  placeholder.style.filter = `blur(${blur}px)`;

  const reset = () => {
    window.cancelAnimationFrame(frame);
    wrap.classList.remove("is-clearing");
    mirror.style.cssText = "";
    placeholder.style.cssText = "";
    mirror.textContent = "";
    glow.style.opacity = "0";
    glow.style.background = "";
  };

  const started = performance.now();

  const tick = (now: number) => {
    const elapsed = now - started;
    const out = easeOut(Math.min(1, elapsed / outDuration));
    const fallIn = easeIn(Math.min(1, elapsed / inDuration));

    mirror.style.transform = `translateY(${(out * outFly).toFixed(1)}px)`;
    mirror.style.opacity = (1 - out).toFixed(3);
    mirror.style.filter = `blur(${(out * blur).toFixed(1)}px)`;
    placeholder.style.transform = `translateY(${(-inFly + fallIn * inFly).toFixed(1)}px)`;
    placeholder.style.opacity = (0.9 + fallIn * 0.1).toFixed(3);
    placeholder.style.filter = `blur(${(blur - fallIn * blur).toFixed(1)}px)`;

    let lit = 0;

    if (elapsed > glowDelay) {
      const progress = Math.min(1, (elapsed - glowDelay) / Math.max(1, total - glowDelay));

      lit = progress < peakAt ? progress / peakAt : 1 - (progress - peakAt) / (1 - peakAt);
    }

    glow.style.opacity = (lit * glowOpacity).toFixed(3);

    if (elapsed < total) {
      frame = window.requestAnimationFrame(tick);
    } else {
      reset();
    }
  };

  frame = window.requestAnimationFrame(tick);

  return reset;
}
