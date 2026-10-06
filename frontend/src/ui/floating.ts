import { type RefObject, useLayoutEffect } from "react";
import { type Placement, useAnchorFallback } from "../lib/anchor.ts";

/**
 * Ties a floating element (menu, popover, tooltip) to its anchor. With CSS anchor positioning
 * the stylesheet does all the work (`.floating[data-placement]` in floating.css); this only
 * names the anchor and points the floating element at it. One trigger can anchor several
 * floating elements (a tooltip and a menu on the same icon button), so the anchor keeps a list.
 */
const anchorNames = new WeakMap<HTMLElement, Set<string>>();

function writeAnchorNames(element: HTMLElement): void {
  const names = anchorNames.get(element);

  if (names === undefined || names.size === 0) {
    element.style.removeProperty("anchor-name");

    return;
  }

  element.style.setProperty("anchor-name", [...names].join(", "));
}

function addAnchorName(element: HTMLElement, name: string): void {
  const names = anchorNames.get(element) ?? new Set<string>();

  names.add(name);
  anchorNames.set(element, names);
  writeAnchorNames(element);
}

function removeAnchorName(element: HTMLElement, name: string): void {
  anchorNames.get(element)?.delete(name);
  writeAnchorNames(element);
}

/** A dashed-ident from a React useId() value. */
export function anchorNameFor(id: string): string {
  return `--anchor-${id.replace(/[^\w-]/g, "")}`;
}

/** Which corner a floating surface grows from, for the dropdown recipe's `data-origin`. */
export function originFor(placement: Placement): string {
  if (placement.startsWith("top")) {
    return placement.endsWith("end") ? "bottom-right" : "bottom-left";
  }

  if (placement.startsWith("left")) {
    return "top-right";
  }

  return placement.endsWith("end") ? "top-right" : "top-left";
}

/**
 * Anchors `floatingRef` to `anchorRef` while `active`: native anchor positioning where supported,
 * the measured fallback elsewhere.
 */
export function useFloating(
  id: string,
  anchorRef: RefObject<HTMLElement | null>,
  floatingRef: RefObject<HTMLElement | null>,
  placement: Placement,
  active: boolean,
): void {
  useLayoutEffect(() => {
    const anchor = anchorRef.current;
    const floating = floatingRef.current;

    if (!active || anchor === null || floating === null) {
      return;
    }

    const name = anchorNameFor(id);

    addAnchorName(anchor, name);
    floating.style.setProperty("position-anchor", name);

    return () => removeAnchorName(anchor, name);
  }, [id, anchorRef, floatingRef, active]);

  useAnchorFallback(anchorRef, floatingRef, placement, active);
}
