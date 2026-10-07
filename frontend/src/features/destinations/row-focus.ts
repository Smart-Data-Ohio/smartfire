/**
 * Keeps keyboard focus in a list when the row holding it leaves: handled, removed or cancelled
 * from its keys, its menu, a context menu or a confirmation dialog. A leaving row turns inert (or
 * goes), which would drop focus on <body>; instead it lands on the next row's open button, else
 * the previous row's, else the list itself. Self-contained: a list only has to mark its parts
 * (`RowParts`), so any list of rows with a main control can use it.
 */
import { type RefObject, useEffect, useRef } from "react";

/** How a list marks its parts, as selectors. */
export interface RowParts {
  /** The list element: focused (made focusable with tabIndex -1) when no row is left. */
  readonly list: string;
  readonly row: string;
  /** A row's main control, which takes focus. */
  readonly open: string;
  /** A row on its way out (it stays in place while its exit plays). */
  readonly leaving: string;
}

/** The destinations' lists (features/destinations/list-row.tsx and paged-list.tsx). */
export const LIST_ROW_PARTS: RowParts = {
  list: "[data-list-root]",
  row: ".list-row",
  open: ".list-row-open",
  leaving: "[data-motion='leave']",
};

/**
 * Where focus waits while a dialog or menu opened from a row is up (including the invisible
 * anchor a context menu hangs from): focus there isn't lost yet, it comes back when they close.
 */
const OVERLAY = "dialog, [role='menu'], [popover], .point-menu-anchor";

/** How often focus is checked while a dialog or menu holds it for a row that left. */
const WAIT_POLL_MS = 50;

/** A row and the rows around it, nearest first, as they were last seen. */
export interface RowPlace {
  readonly row: Element;
  readonly before: readonly Element[];
  readonly after: readonly Element[];
}

/** Whether `row` is still a row focus can stay on: in the document, not leaving, not inert. */
function isLive(row: Element, parts: RowParts): boolean {
  return row.isConnected && !row.matches(parts.leaving) && row.closest("[inert]") === null;
}

/** `row`'s open button, if it can take focus. */
function openOf(row: Element, parts: RowParts): HTMLElement | null {
  const open = row.querySelector<HTMLElement>(parts.open);

  return open === null || open.matches(":disabled") ? null : open;
}

/** `row` with its neighbours inside `root`, nearest first. */
export function placeOf(root: Element, row: Element, parts: RowParts = LIST_ROW_PARTS): RowPlace {
  const rows = [...root.querySelectorAll(parts.row)];
  const index = rows.indexOf(row);

  if (index === -1) {
    return { row, before: [], after: [] };
  }

  return { row, before: rows.slice(0, index).reverse(), after: rows.slice(index + 1) };
}

/**
 * Where focus goes once `place.row` leaves: the next live row's open button, else the previous
 * one's, else the list (made focusable with tabIndex -1). `null` when even the list is gone.
 */
export function landingFor(
  root: Element,
  place: RowPlace,
  parts: RowParts = LIST_ROW_PARTS,
): HTMLElement | null {
  for (const row of [...place.after, ...place.before]) {
    const open = isLive(row, parts) && root.contains(row) ? openOf(row, parts) : null;

    if (open !== null) {
      return open;
    }
  }

  const list = root.matches(parts.list) ? root : root.querySelector<HTMLElement>(parts.list);

  if (!(list instanceof HTMLElement)) {
    return null;
  }

  if (!list.hasAttribute("tabindex")) {
    list.tabIndex = -1;
  }

  return list;
}

/** The open button of the first or last row inside `root` that can take focus, if any. */
export function edgeRowOpen(
  root: Element,
  edge: "first" | "last",
  parts: RowParts = LIST_ROW_PARTS,
): HTMLElement | null {
  const rows = [...root.querySelectorAll(parts.row)];

  for (const row of edge === "first" ? rows : rows.reverse()) {
    const open = isLive(row, parts) ? openOf(row, parts) : null;

    if (open !== null) {
      return open;
    }
  }

  return null;
}

/** Focus fell on <body> (or is stuck inside something inert, before the browser moves it). */
function focusDropped(): boolean {
  const active = document.activeElement;

  return active === null || active === document.body || active.closest("[inert]") !== null;
}

function inOverlay(element: Element): boolean {
  return element.closest(OVERLAY) !== null;
}

function focusQuietly(element: HTMLElement): void {
  element.focus({ preventScroll: true });

  if ("scrollIntoView" in element) {
    element.scrollIntoView({ block: "nearest" });
  }
}

/**
 * Keeps focus in the list inside `rootRef` (a container that outlives the list, so it can also
 * hold focus when the list gives way to its empty state) as rows leave. It tracks the row focus
 * was last in, through any dialog or menu opened from it, and after each render, if that row
 * left while focus was in it (or focus came back to it from a dialog or menu once it had gone),
 * moves focus to the row that took its place. Call it in the component that renders the rows.
 */
export function useKeepRowFocus(
  rootRef: RefObject<HTMLElement | null>,
  parts: RowParts = LIST_ROW_PARTS,
): void {
  /** The row focus was last in, with its neighbours. */
  const place = useRef<RowPlace | null>(null);
  /** The list, holding focus for a row that left: if it goes too, the root takes over. */
  const held = useRef<HTMLElement | null>(null);
  const waiting = useRef<number | undefined>(undefined);

  const stopWaiting = () => {
    window.clearInterval(waiting.current);
    waiting.current = undefined;
  };

  const land = (root: HTMLElement, from: RowPlace) => {
    const fresh =
      from.row.isConnected && root.contains(from.row) ? placeOf(root, from.row, parts) : from;

    const target = landingFor(root, fresh, parts);

    place.current = null;

    if (target === null) {
      return;
    }

    held.current = target.matches(parts.list) ? target : null;
    focusQuietly(target);
  };

  /** Called after each render and while a dialog or menu holds focus for a row that left. */
  const settle = () => {
    const root = rootRef.current;

    if (root === null) {
      return;
    }

    const list = held.current;

    // The list gave way (to its empty or loading state): the root holds focus instead.
    if (list !== null && !list.isConnected && focusDropped()) {
      held.current = null;

      if (!root.hasAttribute("tabindex")) {
        root.tabIndex = -1;
      }

      focusQuietly(root);

      return;
    }

    const current = place.current;

    if (current === null) {
      return;
    }

    if (isLive(current.row, parts) && root.contains(current.row)) {
      place.current = placeOf(root, current.row, parts);

      return;
    }

    const active = document.activeElement;

    if (focusDropped() || (active !== null && current.row.contains(active))) {
      stopWaiting();
      land(root, current);
    } else if (active !== null && inOverlay(active)) {
      // A dialog or menu from the row is still up; focus comes back when it closes.
      waiting.current ??= window.setInterval(() => {
        const now = document.activeElement;

        if (now === null || !now.isConnected || !inOverlay(now)) {
          stopWaiting();
          settle();
        }
      }, WAIT_POLL_MS);
    } else {
      place.current = null;
    }
  };

  useEffect(() => {
    const onFocusIn = (event: FocusEvent) => {
      const root = rootRef.current;
      const target = event.target;

      if (root === null || !(target instanceof Element)) {
        return;
      }

      if (root.contains(target)) {
        const row = target.closest(parts.row);

        // A row that is already leaving counts too: the next render moves focus off it.
        place.current = row === null ? null : placeOf(root, row, parts);

        if (row !== null) {
          held.current = null;
        }
      } else if (!inOverlay(target)) {
        place.current = null;
        held.current = null;
      }
    };

    // A click elsewhere on the page (even on nothing focusable) means focus moved on.
    const onPointerDown = (event: PointerEvent) => {
      const root = rootRef.current;
      const target = event.target;

      if (
        root !== null &&
        target instanceof Element &&
        !root.contains(target) &&
        !inOverlay(target)
      ) {
        place.current = null;
        held.current = null;
      }
    };

    document.addEventListener("focusin", onFocusIn);
    document.addEventListener("pointerdown", onPointerDown, true);

    return () => {
      document.removeEventListener("focusin", onFocusIn);
      document.removeEventListener("pointerdown", onPointerDown, true);
      window.clearInterval(waiting.current);
      waiting.current = undefined;
    };
  }, [rootRef, parts]);

  useEffect(settle);
}
