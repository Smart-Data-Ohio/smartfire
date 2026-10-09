/**
 * Where the tour's card goes beside its anchor, as classic's tour places it: a tall anchor (the
 * sidebar) gets the card beside it, right when it fits, else left; anything else gets it below,
 * or above when below would overflow. Always kept MARGIN inside the viewport. Phones don't use
 * this: there the card is a sheet docked to the edge away from the anchor (`dockFor`).
 */
export interface Box {
  readonly top: number;
  readonly left: number;
  readonly width: number;
  readonly height: number;
}

export interface Size {
  readonly width: number;
  readonly height: number;
}

/** Where the card's top-left corner goes, in viewport pixels. */
export interface Placement {
  readonly left: number;
  readonly top: number;
}

export const MARGIN = 12;

export function cardPosition(anchor: Box, card: Size, viewport: Size): Placement {
  const width = Math.min(card.width, viewport.width - MARGIN * 2);
  const right = anchor.left + anchor.width;
  const bottom = anchor.top + anchor.height;
  const clamp = (value: number, max: number) => Math.max(Math.min(value, max), MARGIN);

  if (anchor.height > viewport.height * 0.6) {
    const beside = right + MARGIN;

    const left =
      beside + width <= viewport.width - MARGIN
        ? beside
        : Math.max(anchor.left - width - MARGIN, MARGIN);

    return { left, top: clamp(anchor.top, viewport.height - card.height - MARGIN) };
  }

  const below = bottom + MARGIN;

  const top =
    below + card.height <= viewport.height - MARGIN
      ? below
      : Math.max(anchor.top - card.height - MARGIN, MARGIN);

  return { left: clamp(anchor.left, viewport.width - width - MARGIN), top };
}

/** A phone's sheet: at the bottom, unless the anchor sits in the lower half (the composer, the You tab). */
export function dockFor(anchor: Box, viewportHeight: number): "top" | "bottom" {
  return anchor.top + anchor.height / 2 > viewportHeight / 2 ? "top" : "bottom";
}

export function sameBox(a: Box | null, b: Box | null): boolean {
  return (
    a === b ||
    (a !== null &&
      b !== null &&
      a.top === b.top &&
      a.left === b.left &&
      a.width === b.width &&
      a.height === b.height)
  );
}
