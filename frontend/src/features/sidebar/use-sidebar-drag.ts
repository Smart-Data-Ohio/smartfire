import {
  type KeyboardEvent,
  type PointerEvent as ReactPointerEvent,
  type RefObject,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import type { RoomCategory } from "../../store/model.ts";
import { isInSlot, organizedSidebar } from "../../store/organize.ts";
import { store } from "../../store/store.ts";
import {
  categoryIndex,
  categoryTargets,
  type DragItem,
  type DropTarget,
  insertionIndex,
  type LabelledTarget,
  roomTargets,
  sameTarget,
  slotForSection,
} from "./organize-model.ts";
import type { SidebarSection } from "./sections.ts";

/** How far the pointer travels before a press becomes a drag (a shorter one is a click). */
const DRAG_THRESHOLD = 4;

/** How close to the list's top or bottom edge the pointer scrolls it. */
const SCROLL_EDGE = 40;

/** Where the floating copy sits from the pointer: below and right, clear of the drop line. */
function ghostAt(x: number, y: number): string {
  return `${x + 14}px ${y + 10}px`;
}

export interface DragState {
  readonly item: DragItem;
  /** Where it would land now; `null` over nowhere useful (or where it already is, for a pointer). */
  readonly target: DropTarget | null;
  readonly mode: "pointer" | "keyboard";
}

interface DragOptions {
  /** The scrolling list the sections live in. */
  readonly containerRef: RefObject<HTMLElement | null>;
  /** The sections as shown (the Favourites placeholder included while a room is dragged). */
  readonly sectionsRef: RefObject<readonly SidebarSection[]>;
  readonly categories: readonly RoomCategory[];
  /** A drop on a real change: the item and where it goes. */
  readonly onDrop: (item: DragItem, target: DropTarget) => void;
}

export interface SidebarDrag {
  readonly drag: DragState | null;
  /** The live region's text: what was picked up, where it is, where it went. */
  readonly announcement: string;
  /** The floating copy that follows the pointer; the sidebar renders it while dragging. */
  readonly ghostRef: RefObject<HTMLDivElement | null>;
  readonly onPointerDown: (event: ReactPointerEvent<HTMLElement>, item: DragItem) => void;
  /** Space picks up; then arrows move, Enter or Space drops, Escape cancels. */
  readonly onKeyDown: (event: KeyboardEvent<HTMLElement>, item: DragItem) => void;
  /** Leaving the handle mid keyboard drag puts the item back. */
  readonly onBlur: () => void;
}

function nameOf(item: DragItem): string {
  return item.kind === "room" ? item.row.displayName : item.category.name;
}

function handleKey(item: DragItem): string {
  return item.kind === "room" ? `room-${item.row.room.id}` : `category-${item.category.id}`;
}

/** Whether `target` would leave `item` where it already is. */
function isNoOp(item: DragItem, target: DropTarget, categories: readonly RoomCategory[]): boolean {
  if (item.kind === "category") {
    return (
      target.kind === "category" && categoryIndex(categories, item.category.id) === target.index
    );
  }

  const view = organizedSidebar(store.getState().sidebar);
  const row = view.rows[item.row.room.id];

  return target.kind === "room" && row !== undefined && isInSlot(view, row, target.slot);
}

/** The vertical middles of `elements`, in viewport coordinates. */
function midpoints(elements: readonly HTMLElement[]): number[] {
  return elements.map((element) => {
    const box = element.getBoundingClientRect();

    return box.top + box.height / 2;
  });
}

/** Swallows the click that ends a pointer drag, so a dragged link doesn't also navigate. */
function swallowNextClick(): void {
  const swallow = (event: MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
  };

  window.addEventListener("click", swallow, { capture: true, once: true });
  window.setTimeout(() => window.removeEventListener("click", swallow, { capture: true }), 0);
}

/**
 * The sidebar's drag and drop, in house: a pointer drag (after a few pixels, with a floating
 * copy, edge scrolling and Escape to cancel) and a keyboard drag over the same targets, each move
 * announced. Touch has no drag: on a phone the row's menu does the moving.
 */
export function useSidebarDrag({
  containerRef,
  sectionsRef,
  categories,
  onDrop,
}: DragOptions): SidebarDrag {
  const [drag, setDrag] = useState<DragState | null>(null);
  const [announcement, setAnnouncement] = useState("");
  const ghostRef = useRef<HTMLDivElement | null>(null);
  const pointer = useRef({ x: 0, y: 0 });

  const keyboard = useRef<{ targets: readonly LabelledTarget[]; index: number }>({
    targets: [],
    index: 0,
  });

  const stopPointer = useRef<(() => void) | null>(null);
  const focusAfter = useRef<string | null>(null);
  const latest = useRef({ drag, categories, onDrop });

  useLayoutEffect(() => {
    latest.current = { drag, categories, onDrop };
  });

  useEffect(() => () => stopPointer.current?.(), []);

  // The floating copy starts under the pointer; later moves place it directly.
  useLayoutEffect(() => {
    const ghost = ghostRef.current;

    if (drag?.mode === "pointer" && ghost !== null) {
      ghost.style.translate = ghostAt(pointer.current.x, pointer.current.y);
    }
  }, [drag?.mode]);

  // A keyboard drop can move the row to another section (a new element): focus follows it.
  useLayoutEffect(() => {
    const key = focusAfter.current;

    if (key === null) {
      return;
    }

    // The first live one: a folded section's own list is inert, its peek isn't.
    const handle = [
      ...(containerRef.current?.querySelectorAll<HTMLElement>(`[data-drag-handle="${key}"]`) ?? []),
    ].find((candidate) => candidate.closest("[inert]") === null);

    if (handle !== undefined) {
      focusAfter.current = null;
      handle.focus({ preventScroll: false });
    }
  });

  const targetAt = (x: number, y: number, item: DragItem): DropTarget | null => {
    const container = containerRef.current;
    const hit = document.elementFromPoint(x, y);

    if (container === null || hit === null || !container.contains(hit)) {
      return null;
    }

    if (item.kind === "category") {
      const others = [
        ...container.querySelectorAll<HTMLElement>('[data-drop-section^="category-"]'),
      ].filter((element) => element.dataset.dropSection !== `category-${item.category.id}`);

      return { kind: "category", index: insertionIndex(midpoints(others), y) };
    }

    const element = hit.closest<HTMLElement>("[data-drop-section]");

    const section = sectionsRef.current.find(
      (candidate) => candidate.key === element?.dataset.dropSection,
    );

    if (element === null || section === undefined) {
      return null;
    }

    const others = section.rows.filter((row) => row.room.id !== item.row.room.id);

    // Over an open Favourites list the gap under the pointer; over a folded one, the end.
    const rows = [
      ...element.querySelectorAll<HTMLElement>('[data-list="main"] > [data-room-id]'),
    ].filter((row) => row.dataset.roomId !== String(item.row.room.id));

    const index =
      element.dataset.open === "true" ? insertionIndex(midpoints(rows), y) : others.length;

    const slot = slotForSection(section, item.row, index);

    return slot === null ? null : { kind: "room", slot };
  };

  const scrollNearEdge = (y: number) => {
    const container = containerRef.current;

    if (container === null) {
      return;
    }

    const box = container.getBoundingClientRect();

    if (y < box.top + SCROLL_EDGE) {
      container.scrollTop -= 8;
    } else if (y > box.bottom - SCROLL_EDGE) {
      container.scrollTop += 8;
    }
  };

  const finish = () => {
    // Settled now, not at the next render: a blur from the drop's own re-render mustn't cancel.
    latest.current = { ...latest.current, drag: null };
    setDrag(null);
    delete document.documentElement.dataset.sidebarDragging;
  };

  const land = (item: DragItem, target: DropTarget | null): boolean => {
    if (target === null || isNoOp(item, target, latest.current.categories)) {
      return false;
    }

    latest.current.onDrop(item, target);

    return true;
  };

  const onPointerDown = (event: ReactPointerEvent<HTMLElement>, item: DragItem) => {
    if (event.button !== 0 || event.pointerType === "touch" || latest.current.drag !== null) {
      return;
    }

    const start = { x: event.clientX, y: event.clientY };
    let started = false;
    let target: DropTarget | null = null;

    const move = (moved: PointerEvent) => {
      pointer.current = { x: moved.clientX, y: moved.clientY };

      if (!started) {
        if (Math.hypot(moved.clientX - start.x, moved.clientY - start.y) < DRAG_THRESHOLD) {
          return;
        }

        started = true;
        document.documentElement.dataset.sidebarDragging = "true";
        setDrag({ item, target: null, mode: "pointer" });
      }

      const ghost = ghostRef.current;

      if (ghost !== null) {
        ghost.style.translate = ghostAt(moved.clientX, moved.clientY);
      }

      scrollNearEdge(moved.clientY);

      const next = targetAt(moved.clientX, moved.clientY, item);
      const shown = next !== null && isNoOp(item, next, latest.current.categories) ? null : next;

      target = shown;
      setDrag((current) =>
        current === null || sameTarget(current.target, shown)
          ? current
          : { ...current, target: shown },
      );
    };

    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", cancel);
      window.removeEventListener("keydown", onEscape, { capture: true });
      stopPointer.current = null;
    };

    const up = () => {
      stop();

      if (started) {
        swallowNextClick();
        land(item, target);
        finish();
      }
    };

    const cancel = () => {
      stop();

      if (started) {
        finish();
      }
    };

    const onEscape = (pressed: globalThis.KeyboardEvent) => {
      if (pressed.key === "Escape" && started) {
        pressed.preventDefault();
        pressed.stopPropagation();
        cancel();
      }
    };

    stopPointer.current?.();
    stopPointer.current = cancel;
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", cancel);
    window.addEventListener("keydown", onEscape, { capture: true });
  };

  const pickUp = (item: DragItem) => {
    const shown = sectionsRef.current;
    const held = latest.current.categories;
    const view = organizedSidebar(store.getState().sidebar);

    const targets =
      item.kind === "room" ? roomTargets(shown, item.row) : categoryTargets(held, item.category);

    const here = targets.findIndex(({ target }) => {
      if (item.kind === "category") {
        return target.kind === "category" && target.index === categoryIndex(held, item.category.id);
      }

      const row = view.rows[item.row.room.id];

      return target.kind === "room" && row !== undefined && isInSlot(view, row, target.slot);
    });

    const index = Math.max(here, 0);
    const first = targets[index];

    if (first === undefined) {
      return;
    }

    keyboard.current = { targets, index };
    setDrag({ item, target: first.target, mode: "keyboard" });
    setAnnouncement(
      `Picked up ${nameOf(item)}, at ${first.label}. Up and down arrows move it, Enter drops it, Escape cancels.`,
    );
  };

  const step = (by: number) => {
    const { targets, index } = keyboard.current;
    const next = Math.min(Math.max(index + by, 0), targets.length - 1);
    const chosen = targets[next];

    if (chosen === undefined) {
      return;
    }

    keyboard.current = { targets, index: next };
    setDrag((current) => (current === null ? current : { ...current, target: chosen.target }));
    setAnnouncement(chosen.label);
  };

  const dropFromKeyboard = (item: DragItem) => {
    const chosen = keyboard.current.targets[keyboard.current.index];
    const moved = land(item, chosen?.target ?? null);

    focusAfter.current = handleKey(item);
    finish();
    setAnnouncement(
      moved && chosen !== undefined
        ? `Dropped ${nameOf(item)} at ${chosen.label}.`
        : `${nameOf(item)} stays where it was.`,
    );
  };

  const cancelKeyboard = (item: DragItem) => {
    finish();
    setAnnouncement(`Cancelled. ${nameOf(item)} stays where it was.`);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLElement>, item: DragItem) => {
    const current = latest.current.drag;
    const plain = !event.altKey && !event.ctrlKey && !event.metaKey && !event.shiftKey;

    if (current === null) {
      if (event.key === " " && plain) {
        event.preventDefault();
        pickUp(item);
      }

      return;
    }

    if (current.mode !== "keyboard") {
      return;
    }

    switch (event.key) {
      case "ArrowUp":
      case "ArrowDown":
        event.preventDefault();
        event.stopPropagation();
        step(event.key === "ArrowUp" ? -1 : 1);
        break;
      case "Enter":
      case " ":
        event.preventDefault();
        event.stopPropagation();
        dropFromKeyboard(current.item);
        break;
      case "Escape":
        event.preventDefault();
        event.stopPropagation();
        cancelKeyboard(current.item);
        break;
      case "Tab":
        cancelKeyboard(current.item);
        break;
      default:
        break;
    }
  };

  const onBlur = () => {
    const current = latest.current.drag;

    if (current?.mode === "keyboard") {
      cancelKeyboard(current.item);
    }
  };

  return { drag, announcement, ghostRef, onPointerDown, onKeyDown, onBlur };
}
