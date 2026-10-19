import {
  type KeyboardEvent,
  type ReactNode,
  type PointerEvent as ReactPointerEvent,
  type RefObject,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  useSyncExternalStore,
} from "react";
import type { WorkStatus } from "../../gen/WorkStatus.ts";
import { readDurationMs } from "../../motion/durations.ts";
import { prefersReducedMotion } from "../../motion/reduced-motion.ts";
import { store, useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { toast } from "../../ui/toast-store.ts";
import { useAnnouncer } from "../destinations/live-region.tsx";
import { threadTitle } from "../threads/thread-format.ts";
import { canMoveWork, type Landing, landingFor, statusBeside } from "./board-drag.ts";
import { WORK_STATUS_LABEL, WORK_STATUSES } from "./board-format.ts";
import { cssValue } from "./use-live-flip.ts";
import "./board-drag.css";

const DRAG_THRESHOLD_PX = 5;

const SCROLL_EDGE_PX = 56;

const SCROLL_SPEED_PX_PER_FRAME = 18;

const FINE_POINTER = "(any-pointer: fine)";

/**
 * A card being moved: lifted (following the pointer, or the arrow keys), or flying back to its
 * slot after a pointer drag was cancelled. `over` is the column it would go to, `null` outside
 * every column.
 */
export type BoardDragState =
  | {
      readonly phase: "lifted";
      readonly threadId: number;
      readonly mode: "pointer" | "keyboard";
      readonly over: WorkStatus | null;
      readonly slotHeight: number;
    }
  | { readonly phase: "returning"; readonly threadId: number };

interface Arrival {
  readonly threadId: number;
  readonly status: WorkStatus;
  readonly from: DOMRect;
  ghost: HTMLElement | null;
  readonly focus: boolean;
}

export interface CardDrag {
  readonly held: { readonly threadId: number; readonly look: string } | null;
  readonly hintId: string;
  readonly onPointerDown: (event: ReactPointerEvent<HTMLElement>, threadId: number) => void;
  readonly onKeyDown: (event: KeyboardEvent<HTMLElement>, threadId: number) => void;
  readonly onBlur: () => void;
}

/** jsdom (the unit tests) has a `matchMedia` property but no function behind it. */
function canMatchMedia(): boolean {
  return "matchMedia" in window && window.matchMedia instanceof Function;
}

function subscribeFinePointer(onChange: () => void): () => void {
  if (!canMatchMedia()) {
    return () => {};
  }

  const media = window.matchMedia(FINE_POINTER);

  media.addEventListener("change", onChange);

  return () => media.removeEventListener("change", onChange);
}

function hasFinePointer(): boolean {
  return canMatchMedia() && window.matchMedia(FINE_POINTER).matches;
}

function isStatus(value: string | undefined): value is WorkStatus {
  return WORK_STATUSES.some((status) => status === value);
}

function statusAt(board: HTMLElement | null, x: number, y: number): WorkStatus | null {
  const column = document.elementFromPoint(x, y)?.closest<HTMLElement>(".board-column");
  const status = column?.dataset.status;

  return board !== null && column != null && board.contains(column) && isStatus(status)
    ? status
    : null;
}

function liftGhost(card: HTMLElement, rect: DOMRect): HTMLElement {
  const ghost = document.createElement("div");

  ghost.className = "board-drag-ghost";
  ghost.setAttribute("aria-hidden", "true");
  ghost.inert = true;
  ghost.style.width = `${rect.width}px`;
  ghost.append(card.cloneNode(true));

  for (const name of ["id", "href", "aria-current", "aria-describedby"]) {
    ghost.firstElementChild?.removeAttribute(name);
  }

  document.body.append(ghost);

  return ghost;
}

function reveal(element: Element, container: HTMLElement | null, axis: "x" | "y"): void {
  if (container === null) {
    return;
  }

  const box = element.getBoundingClientRect();
  const view = container.getBoundingClientRect();

  const [start, end, viewStart, viewEnd] =
    axis === "x"
      ? [box.left, box.right, view.left, view.right]
      : [box.top, box.bottom, view.top, view.bottom];

  const by = start < viewStart ? start - viewStart - 8 : end > viewEnd ? end - viewEnd + 8 : 0;

  if (axis === "x") {
    container.scrollLeft += by;
  } else {
    container.scrollTop += by;
  }
}

/** Swallows the click that ends a pointer drag, so the dragged card doesn't also open. */
function swallowNextClick(): void {
  const swallow = (event: MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
  };

  window.addEventListener("click", swallow, { capture: true, once: true });
  window.setTimeout(() => window.removeEventListener("click", swallow, { capture: true }), 0);
}

function titleOf(threadId: number): string {
  const thread = store.getState().threads[threadId];

  return thread === undefined ? "The post" : threadTitle(thread);
}

function statusOf(threadId: number): WorkStatus | undefined {
  return store.getState().threads[threadId]?.work?.status;
}

function settle(item: HTMLElement, { from, ghost, focus }: Arrival): void {
  ghost?.remove();

  for (const liveSettle of item.getAnimations()) {
    liveSettle.cancel();
  }

  reveal(item, item.closest<HTMLElement>(".board-column-list"), "y");

  if (focus && (document.activeElement === null || document.activeElement === document.body)) {
    item.querySelector<HTMLElement>("a")?.focus({ preventScroll: true });
  }

  if (prefersReducedMotion()) {
    return;
  }

  const to = item.getBoundingClientRect();
  const lifted = ghost === null ? {} : { rotate: "2deg", scale: "1.02" };

  item.animate(
    [
      { translate: `${from.left - to.left}px ${from.top - to.top}px`, ...lifted },
      { translate: "0 0", rotate: "0deg", scale: "1" },
    ],
    { duration: readDurationMs("--duration-medium"), easing: cssValue("--ease-move", "ease-out") },
  );
}

export interface BoardDrag {
  readonly drag: BoardDragState | null;
  readonly landing: Landing | null;
  /** The cards' part, `undefined` without a fine pointer. */
  readonly card: CardDrag | undefined;
  /** The live region and the keyboard hint the cards point to. */
  readonly region: ReactNode;
}

/**
 * Moves a board's cards between its status columns. With a mouse or pen, a press that travels a
 * few pixels lifts a copy of the card, and the copy follows the pointer. The column under it
 * lights up and shows the slot the card would take. A drop there changes the post's status at
 * once, and a refused change rolls back with a toast. Escape, or a drop outside the columns,
 * sends the card back. On the keyboard, Space picks up the focused card, the arrows choose a
 * column, Enter or Space drops it, and Escape cancels. A live region announces each step. Only
 * cards the viewer may move (`canMoveWork`) take part, and only with a fine pointer: a phone
 * changes status from the post.
 */
export function useBoardDrag(roomId: number, boardRef: RefObject<HTMLElement | null>): BoardDrag {
  const [drag, setDrag] = useState<BoardDragState | null>(null);
  const fine = useSyncExternalStore(subscribeFinePointer, hasFinePointer, () => false);
  const { announce, region } = useAnnouncer("assertive");
  const hintId = useId();
  const latest = useRef<BoardDragState | null>(null);
  const stopPointer = useRef<(() => void) | null>(null);
  const arrival = useRef<Arrival | null>(null);

  const heldMovable = useStore(
    (state) => drag !== null && canMoveWork(state, roomId, drag.threadId),
  );

  const index = useStore((state) =>
    drag?.phase === "lifted"
      ? (landingFor(state, roomId, drag.threadId, drag.over)?.index ?? -1)
      : -1,
  );

  const landing =
    drag?.phase === "lifted" && drag.over !== null && index !== -1
      ? { status: drag.over, index }
      : null;

  useEffect(
    () => () => {
      stopPointer.current?.();
      arrival.current?.ghost?.remove();
      delete document.documentElement.dataset.boardDragging;
    },
    [],
  );

  useLayoutEffect(() => {
    const pending = arrival.current;

    if (pending === null) {
      return;
    }

    const item = boardRef.current?.querySelector<HTMLElement>(
      `.board-column[data-status="${pending.status}"] [data-flip="${pending.threadId}"]`,
    );

    if (item == null) {
      pending.ghost?.remove();
      pending.ghost = null;

      return;
    }

    arrival.current = null;
    settle(item, pending);
  });

  const slotAt = landing === null ? "" : `${landing.status}:${landing.index}`;

  useEffect(() => {
    const slot = slotAt === "" ? null : boardRef.current?.querySelector(".board-drop-slot");

    if (slot != null) {
      reveal(slot, slot.closest<HTMLElement>(".board-column-list"), "y");
    }
  }, [boardRef, slotAt]);

  const update = (next: BoardDragState | null) => {
    latest.current = next;
    setDrag(next);

    if (next?.phase !== "lifted" || next.mode !== "pointer") {
      delete document.documentElement.dataset.boardDragging;
    }
  };

  const cardItem = (threadId: number) =>
    boardRef.current?.querySelector<HTMLElement>(`[data-flip="${threadId}"]`) ?? null;

  const tryLand = (threadId: number, to: WorkStatus | null, ghost: HTMLElement | null): boolean => {
    const from = statusOf(threadId);
    const item = cardItem(threadId);

    if (
      to === null ||
      from === undefined ||
      to === from ||
      item === null ||
      !canMoveWork(store.getState(), roomId, threadId)
    ) {
      return false;
    }

    const title = titleOf(threadId);

    const pending: Arrival = {
      threadId,
      status: to,
      from: (ghost ?? item).getBoundingClientRect(),
      ghost,
      focus: ghost === null,
    };

    arrival.current = pending;
    update(null);
    announce(`Moved ${title} to ${WORK_STATUS_LABEL[to]}.`);
    actions.work
      .setStatus(threadId, to)
      .catch((error: Error) => {
        toast({ title: `Couldn't move ${title}`, description: error.message, tone: "danger" });
      })
      .finally(() => {
        if (arrival.current === pending) {
          arrival.current = null;
        }
      });

    return true;
  };

  const sendBack = (threadId: number, ghost: HTMLElement) => {
    const home = cardItem(threadId);

    if (home === null || prefersReducedMotion()) {
      ghost.remove();
      update(null);

      return;
    }

    const to = home.getBoundingClientRect();

    update({ phase: "returning", threadId });
    ghost.dataset.returning = "true";
    ghost
      .animate([{ translate: ghost.style.translate }, { translate: `${to.left}px ${to.top}px` }], {
        duration: readDurationMs("--duration-small"),
        easing: cssValue("--ease-move", "ease-out"),
        fill: "forwards",
      })
      .finished.catch(() => undefined)
      .finally(() => {
        ghost.remove();

        if (latest.current?.phase === "returning" && latest.current.threadId === threadId) {
          update(null);
        }
      });
  };

  const onPointerDown = (event: ReactPointerEvent<HTMLElement>, threadId: number) => {
    if (
      event.button !== 0 ||
      event.pointerType === "touch" ||
      latest.current !== null ||
      !canMoveWork(store.getState(), roomId, threadId)
    ) {
      return;
    }

    const card = event.currentTarget;
    const start = { x: event.clientX, y: event.clientY };
    let point = start;
    let grab = { x: 0, y: 0 };
    let ghost: HTMLElement | null = null;
    let frame = 0;
    let cancelled = false;

    const place = () => {
      if (ghost !== null) {
        ghost.style.translate = `${point.x - grab.x}px ${point.y - grab.y}px`;
      }
    };

    const retarget = () => {
      const over = statusAt(boardRef.current, point.x, point.y);
      const current = latest.current;

      if (current?.phase === "lifted" && current.over !== over) {
        update({ ...current, over });
      }
    };

    const edgeScroll = () => {
      const board = boardRef.current;

      if (board !== null && board.scrollWidth > board.clientWidth) {
        const box = board.getBoundingClientRect();
        const left = point.x - box.left;
        const right = box.right - point.x;
        const inside = point.y >= box.top && point.y <= box.bottom;

        const speed = !inside
          ? 0
          : left < SCROLL_EDGE_PX
            ? -Math.ceil(
                ((SCROLL_EDGE_PX - Math.max(left, 0)) / SCROLL_EDGE_PX) * SCROLL_SPEED_PX_PER_FRAME,
              )
            : right < SCROLL_EDGE_PX
              ? Math.ceil(
                  ((SCROLL_EDGE_PX - Math.max(right, 0)) / SCROLL_EDGE_PX) *
                    SCROLL_SPEED_PX_PER_FRAME,
                )
              : 0;

        if (speed !== 0) {
          board.scrollLeft += speed;
          retarget();
        }
      }

      frame = requestAnimationFrame(edgeScroll);
    };

    const lift = () => {
      const rect = card.getBoundingClientRect();

      grab = { x: start.x - rect.left, y: start.y - rect.top };
      ghost = liftGhost(card, rect);
      place();
      window.getSelection()?.removeAllRanges();
      document.documentElement.dataset.boardDragging = "true";
      update({
        phase: "lifted",
        threadId,
        mode: "pointer",
        over: statusAt(boardRef.current, point.x, point.y),
        slotHeight: rect.height,
      });
      frame = requestAnimationFrame(edgeScroll);
    };

    const move = (moved: PointerEvent) => {
      point = { x: moved.clientX, y: moved.clientY };

      if (ghost === null) {
        if (Math.hypot(point.x - start.x, point.y - start.y) >= DRAG_THRESHOLD_PX) {
          lift();
        }

        return;
      }

      place();
      retarget();
    };

    const stop = () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", cancel);
      window.removeEventListener("blur", cancel);
      window.removeEventListener("keydown", onEscape, { capture: true });
      stopPointer.current = null;
    };

    const up = () => {
      stop();

      if (ghost === null) {
        return;
      }

      swallowNextClick();

      if (cancelled) {
        return;
      }

      const current = latest.current;
      const over = current?.phase === "lifted" ? current.over : null;

      if (!tryLand(threadId, over, ghost)) {
        sendBack(threadId, ghost);
      }
    };

    const cancel = () => {
      stop();

      if (ghost !== null && !cancelled) {
        sendBack(threadId, ghost);
      }
    };

    const onEscape = (pressed: globalThis.KeyboardEvent) => {
      if (pressed.key !== "Escape" || ghost === null || cancelled) {
        return;
      }

      pressed.preventDefault();
      pressed.stopPropagation();
      cancelled = true;
      cancelAnimationFrame(frame);
      window.removeEventListener("pointermove", move);
      sendBack(threadId, ghost);
    };

    stopPointer.current = () => {
      stop();
      ghost?.remove();
    };

    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", cancel);
    window.addEventListener("blur", cancel);
    window.addEventListener("keydown", onEscape, { capture: true });
  };

  const choose = (over: WorkStatus) => {
    const current = latest.current;

    if (current?.phase !== "lifted") {
      return;
    }

    update({ ...current, over });

    const column = boardRef.current?.querySelector(`.board-column[data-status="${over}"]`);

    if (column != null) {
      reveal(column, boardRef.current, "x");
    }

    announce(
      over === statusOf(current.threadId)
        ? `${WORK_STATUS_LABEL[over]}, where it is now.`
        : WORK_STATUS_LABEL[over],
    );
  };

  const putBack = (threadId: number) => {
    const status = statusOf(threadId);

    update(null);
    announce(
      `Cancelled. ${titleOf(threadId)} stays in ${status === undefined ? "its column" : WORK_STATUS_LABEL[status]}.`,
    );
  };

  const onKeyDown = (event: KeyboardEvent<HTMLElement>, threadId: number) => {
    const current = latest.current;
    const plain = !event.altKey && !event.ctrlKey && !event.metaKey && !event.shiftKey;

    if (current === null) {
      const status = statusOf(threadId);
      const item = cardItem(threadId);

      if (
        event.key !== " " ||
        !plain ||
        status === undefined ||
        item === null ||
        !canMoveWork(store.getState(), roomId, threadId)
      ) {
        return;
      }

      event.preventDefault();
      update({
        phase: "lifted",
        threadId,
        mode: "keyboard",
        over: status,
        slotHeight: item.getBoundingClientRect().height,
      });
      announce(
        `Picked up ${titleOf(threadId)}, in ${WORK_STATUS_LABEL[status]}. Left and right arrows choose a column, Space or Enter drops it, Escape cancels.`,
      );

      return;
    }

    if (current.phase !== "lifted" || current.mode !== "keyboard") {
      return;
    }

    if (current.threadId !== threadId) {
      putBack(current.threadId);

      return;
    }

    const over = current.over ?? statusOf(threadId) ?? "planned";

    switch (event.key) {
      case "ArrowLeft":
      case "ArrowRight":
        event.preventDefault();
        choose(statusBeside(over, event.key === "ArrowLeft" ? -1 : 1));
        break;
      case "Home":
      case "End":
        event.preventDefault();
        choose(
          statusBeside(over, event.key === "Home" ? -WORK_STATUSES.length : WORK_STATUSES.length),
        );
        break;
      case "Enter":
      case " ":
        event.preventDefault();

        if (!tryLand(current.threadId, current.over, null)) {
          putBack(current.threadId);
        }

        break;
      case "Escape":
        event.preventDefault();
        event.stopPropagation();
        putBack(current.threadId);
        break;
      case "Tab":
        putBack(current.threadId);
        break;
      default:
        break;
    }
  };

  useLayoutEffect(() => {
    const current = latest.current;

    if (current?.phase !== "lifted" || current.mode !== "keyboard") {
      return;
    }

    const link = cardItem(current.threadId)?.querySelector<HTMLElement>("a");

    if (link == null || !heldMovable) {
      putBack(current.threadId);
    } else if (document.activeElement !== link) {
      link.focus({ preventScroll: true });
    }
  });

  const onBlur = () => {
    const current = latest.current;

    if (current?.phase === "lifted" && current.mode === "keyboard") {
      putBack(current.threadId);
    }
  };

  const held =
    drag === null
      ? null
      : { threadId: drag.threadId, look: drag.phase === "lifted" ? drag.mode : drag.phase };

  return {
    drag,
    landing,
    card: fine ? { held, hintId, onPointerDown, onKeyDown, onBlur } : undefined,
    region: (
      <>
        {region}
        <p id={hintId} className="visually-hidden">
          Press Space to pick this post up, then use the left and right arrows to move it to another
          column.
        </p>
      </>
    ),
  };
}

/**
 * One card's part in the drag: how its item looks while held, and its link's handlers. A card
 * the viewer can't move gets none of them, so it stays a plain link.
 */
export function useCardDrag(drag: CardDrag | undefined, roomId: number, threadId: number) {
  const movable = useStore((state) => canMoveWork(state, roomId, threadId));
  const look = drag?.held?.threadId === threadId ? drag.held.look : undefined;

  if (drag === undefined || !movable) {
    return { look, handle: {} };
  }

  return {
    look,
    handle: {
      "data-movable": true,
      "aria-describedby": drag.hintId,
      draggable: false,
      onDragStart: (event: { preventDefault: () => void }) => event.preventDefault(),
      onPointerDown: (event: ReactPointerEvent<HTMLElement>) => drag.onPointerDown(event, threadId),
      onKeyDown: (event: KeyboardEvent<HTMLElement>) => drag.onKeyDown(event, threadId),
      onBlur: drag.onBlur,
    },
  };
}

/** A column's cards with the held card's slot among them, when that's the column it would go to. */
export function withSlot(
  ids: readonly number[],
  status: WorkStatus,
  landing: Landing | null,
): readonly (number | "slot")[] {
  return landing?.status === status
    ? [...ids.slice(0, landing.index), "slot", ...ids.slice(landing.index)]
    : ids;
}

/** Where the held card would land, as tall as the card. */
export function DropSlot({ drag }: { readonly drag: BoardDragState | null }) {
  return (
    <li
      className="board-drop-slot"
      style={{ height: drag?.phase === "lifted" ? drag.slotHeight : undefined }}
      aria-hidden="true"
    >
      Moves here
      <span className="board-drop-slot-note">Sorted by latest activity</span>
    </li>
  );
}
