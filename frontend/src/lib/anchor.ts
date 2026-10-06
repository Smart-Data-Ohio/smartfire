import { type RefObject, useLayoutEffect } from "react";

/**
 * Where a floating element sits relative to its anchor. "bottom-start" hangs below the anchor,
 * aligned to its start edge; "right-start" opens beside it (submenus).
 */
export type Placement =
  | "bottom-start"
  | "bottom-end"
  | "bottom"
  | "top-start"
  | "top-end"
  | "top"
  | "right-start"
  | "left-start"
  | "right"
  | "left";

/**
 * CSS anchor positioning reached Baseline in January 2026 (Chrome 125, Safari 26, Firefox 147).
 * Where it is supported, the stylesheet places floating elements (`position-anchor` plus
 * `position-area`, flipping with `position-try-fallbacks`) and nothing runs here. Elsewhere this
 * module measures and places them with fixed positioning, flipping to the other side when the
 * preferred one would overflow the viewport.
 */
export function supportsAnchorPositioning(): boolean {
  return (
    "CSS" in window &&
    "supports" in CSS &&
    CSS.supports("anchor-name: --a") &&
    CSS.supports("position-area: top")
  );
}

const GAP = 6;

interface Point {
  readonly x: number;
  readonly y: number;
}

/** The start coordinate for "start", "end" or centred alignment along one axis. */
function alignStart(
  align: string,
  start: number,
  end: number,
  anchorSize: number,
  floatingSize: number,
): number {
  if (align === "start") {
    return start;
  }

  if (align === "end") {
    return end - floatingSize;
  }

  return start + (anchorSize - floatingSize) / 2;
}

function placeOnSide(placement: Placement, anchor: DOMRect, floating: DOMRect): Point {
  const [side, align = "center"] = placement.split("-");
  const vertical = side === "top" || side === "bottom";

  if (vertical) {
    const y = side === "bottom" ? anchor.bottom + GAP : anchor.top - floating.height - GAP;

    return { x: alignStart(align, anchor.left, anchor.right, anchor.width, floating.width), y };
  }

  const x = side === "right" ? anchor.right + GAP / 2 : anchor.left - floating.width - GAP / 2;
  const y = align === "start" ? anchor.top - 4 : anchor.top + (anchor.height - floating.height) / 2;

  return { x, y };
}

const FLIPPED = new Map([
  ["top", "bottom"],
  ["bottom", "top"],
  ["left", "right"],
  ["right", "left"],
]);

function flip(placement: Placement): Placement {
  const [side = "bottom", align] = placement.split("-");
  const flipped = FLIPPED.get(side) ?? side;
  const candidate = align === undefined ? flipped : `${flipped}-${align}`;

  // SAFETY: `candidate` swaps the side of a valid Placement for its opposite and keeps the
  // alignment, which is again a member of the Placement union.
  return candidate as Placement;
}

function overflows(point: Point, floating: DOMRect): boolean {
  return (
    point.y < 0 ||
    point.x < 0 ||
    point.y + floating.height > window.innerHeight ||
    point.x + floating.width > window.innerWidth
  );
}

/** Fallback placement: measures both boxes and writes fixed coordinates onto the floating one. */
export function placeFloating(anchor: Element, floating: HTMLElement, placement: Placement): void {
  const anchorBox = anchor.getBoundingClientRect();
  const floatingBox = floating.getBoundingClientRect();
  let point = placeOnSide(placement, anchorBox, floatingBox);

  if (overflows(point, floatingBox)) {
    const flipped = placeOnSide(flip(placement), anchorBox, floatingBox);

    if (!overflows(flipped, floatingBox)) {
      point = flipped;
    }
  }

  const x = Math.min(Math.max(point.x, GAP), window.innerWidth - floatingBox.width - GAP);
  const y = Math.min(Math.max(point.y, GAP), window.innerHeight - floatingBox.height - GAP);

  floating.style.position = "fixed";
  floating.style.inset = "auto";
  floating.style.margin = "0";
  floating.style.left = `${Math.round(x)}px`;
  floating.style.top = `${Math.round(y)}px`;
}

/**
 * Keeps a floating element beside its anchor while it is shown, using the JavaScript fallback
 * only when CSS anchor positioning is unavailable.
 */
export function useAnchorFallback(
  anchorRef: RefObject<Element | null>,
  floatingRef: RefObject<HTMLElement | null>,
  placement: Placement,
  active: boolean,
): void {
  useLayoutEffect(() => {
    const anchor = anchorRef.current;
    const floating = floatingRef.current;

    if (!active || anchor === null || floating === null || supportsAnchorPositioning()) {
      return;
    }

    const update = () => placeFloating(anchor, floating, placement);

    update();
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, true);

    return () => {
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", update, true);
    };
  }, [anchorRef, floatingRef, placement, active]);
}
