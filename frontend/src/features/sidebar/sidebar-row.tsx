import { Link } from "@tanstack/react-router";
import {
  createContext,
  type KeyboardEvent,
  type PointerEvent as ReactPointerEvent,
  use,
  useEffect,
  useRef,
} from "react";
import type { SidebarRow as Row } from "../../store/model.ts";
import { useStore } from "../../store/store.ts";
import { AgentThinking } from "../../ui/agent-thinking.tsx";
import { Badge } from "../../ui/badge.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { isAgent, useUser } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { ROOM_KIND_ICON } from "../room/room-icon.ts";
import { GroupAvatars } from "./group-avatars.tsx";
import { rowPillCount, rowState } from "./sections.ts";

/** How long a finger rests on a row before its menu opens. */
const LONG_PRESS_MS = 500;

/** How far a resting finger may drift (a scroll cancels the long press). */
const LONG_PRESS_SLOP = 8;

/** How long a press may wait for its release before the menu opens anyway. */
const RELEASE_WAIT_MS = 1000;

/**
 * Runs `open` once the pressed button or finger comes up: a menu opened mid-press is in the top
 * layer when the release lands outside it, and the browser would light-dismiss it at once.
 */
function afterRelease(pressed: boolean, open: () => void): void {
  if (!pressed) {
    open();

    return;
  }

  const done = () => {
    window.removeEventListener("pointerup", done, true);
    window.removeEventListener("pointercancel", done, true);
    window.clearTimeout(fallback);
    window.setTimeout(open, 0);
  };

  const fallback = window.setTimeout(done, RELEASE_WAIT_MS);

  window.addEventListener("pointerup", done, true);
  window.addEventListener("pointercancel", done, true);
}

/** What a row can ask of the sidebar: drag it, or open its menu somewhere. */
export interface RowActions {
  /** The id of the hint every row is described by (how to move it, how to open its menu). */
  readonly hintId: string | undefined;
  readonly onPointerDown: (event: ReactPointerEvent<HTMLElement>, row: Row) => void;
  readonly onKeyDown: (event: KeyboardEvent<HTMLElement>, row: Row) => void;
  readonly onBlur: () => void;
  /** Opens the row's menu at a viewport point (a right click, a long press). */
  readonly openMenuAt: (row: Row, x: number, y: number) => void;
  /** Opens it from an element (the "⋯" button, or the row itself for Shift+F10). */
  readonly openMenuFrom: (row: Row, element: HTMLElement, keyboard: boolean) => void;
}

const noop = () => {};

export const RowActionsContext = createContext<RowActions>({
  hintId: undefined,
  onPointerDown: noop,
  onKeyDown: noop,
  onBlur: noop,
  openMenuAt: noop,
  openMenuFrom: noop,
});

/** Whether someone (other than the viewer) is typing in the room right now. */
function useSomeoneTyping(roomId: number): number | null {
  return useStore((state) => {
    const typists = state.typing[`room:${roomId}`];

    if (typists === undefined) {
      return null;
    }

    const first = Object.keys(typists)[0];

    return first === undefined ? null : Number(first);
  });
}

function DirectGlyph({ row }: { readonly row: Row }) {
  const otherId = row.directMemberIds[0];
  const typistId = useSomeoneTyping(row.room.id);
  const typist = useUser(typistId ?? undefined);

  if (typistId !== null && isAgent(typist)) {
    return (
      <span className="sidebar-row-glyph">
        <AgentThinking
          size={20}
          state="composing"
          label={`${typist?.name ?? "Agent"} is replying`}
        />
      </span>
    );
  }

  if (row.directMemberIds.length > 1) {
    return (
      <span className="sidebar-row-glyph">
        <GroupAvatars ids={row.directMemberIds} size={20} />
      </span>
    );
  }

  return (
    <span className="sidebar-row-glyph">
      {otherId === undefined ? null : <UserAvatar userId={otherId} size={20} presence decorative />}
    </span>
  );
}

/** The row's leading glyph: a person (or two) for direct messages, the room kind's icon else. */
export function RowGlyph({ row }: { readonly row: Row }) {
  return row.room.kind === "direct" ? (
    <DirectGlyph row={row} />
  ) : (
    <span className="sidebar-row-glyph">
      <Icon name={ROOM_KIND_ICON[row.room.kind]} size={16} className="sidebar-row-icon" />
    </span>
  );
}

interface LongPress {
  readonly timer: number;
  readonly x: number;
  readonly y: number;
  fired: boolean;
}

/** A long press on touch screens opens the menu (and the tap it ends in doesn't navigate). */
function useLongPress(row: Row) {
  const { openMenuAt } = use(RowActionsContext);
  const press = useRef<LongPress | null>(null);

  useEffect(() => () => window.clearTimeout(press.current?.timer), []);

  const cancel = () => {
    window.clearTimeout(press.current?.timer);
  };

  return {
    start: (event: ReactPointerEvent<HTMLElement>) => {
      cancel();

      if (event.pointerType !== "touch") {
        press.current = null;

        return;
      }

      const { clientX: x, clientY: y } = event;

      const current: LongPress = {
        x,
        y,
        fired: false,
        timer: window.setTimeout(() => {
          current.fired = true;
          afterRelease(true, () => openMenuAt(row, x, y));
        }, LONG_PRESS_MS),
      };

      press.current = current;
    },
    move: (event: ReactPointerEvent<HTMLElement>) => {
      const current = press.current;

      if (
        current !== null &&
        Math.hypot(event.clientX - current.x, event.clientY - current.y) > LONG_PRESS_SLOP
      ) {
        cancel();
      }
    },
    cancel,
    /** Whether the gesture that just ended opened the menu. */
    fired: () => press.current?.fired ?? false,
  };
}

/** Where a drop line shows on this row while something is dragged over its list. */
export type DropEdge = "before" | "after";

interface SidebarRowProps {
  readonly row: Row;
  readonly selected: boolean;
  /** In its section's own list (not the peek under a folded heading): it moves with FLIP. */
  readonly main?: boolean;
  readonly dropEdge?: DropEdge | undefined;
  /** This is the row being dragged. */
  readonly dragging?: boolean;
}

/**
 * One conversation in the sidebar. Unread rows are bold, muted rows faint, mentions (and every
 * unread direct message) carry a count that pops in, and the selected row is tinted. A right
 * click, a long press, the "⋯" button or Shift+F10 opens its menu; it can be dragged, by pointer
 * or with Space and the arrows.
 */
export function SidebarRow({
  row,
  selected,
  main = false,
  dropEdge,
  dragging = false,
}: SidebarRowProps) {
  const actions = use(RowActionsContext);
  const longPress = useLongPress(row);
  const { room, membership } = row;
  const muted = membership.involvement === "muted";
  const pill = rowPillCount(row);
  const state = rowState(row, selected);

  return (
    <li
      className="sidebar-row-item"
      data-room-id={room.id}
      data-flip={main ? `room-${room.id}` : undefined}
      data-drop-edge={dropEdge}
      data-dragging={dragging || undefined}
    >
      <Link
        to="/r/$roomId"
        params={{ roomId: room.id }}
        className="sidebar-row"
        data-state={state ?? undefined}
        data-muted={(muted && state === "unread") || undefined}
        data-drag-handle={`room-${room.id}`}
        aria-current={selected ? "page" : undefined}
        aria-describedby={actions.hintId}
        draggable={false}
        preload={false}
        onPointerDown={(event) => {
          longPress.start(event);
          actions.onPointerDown(event, row);
        }}
        onDragStart={(event) => event.preventDefault()}
        onPointerMove={longPress.move}
        onPointerUp={longPress.cancel}
        onPointerCancel={longPress.cancel}
        onClick={(event) => {
          if (longPress.fired()) {
            event.preventDefault();
          }
        }}
        onContextMenu={(event) => {
          event.preventDefault();

          if (longPress.fired()) {
            return;
          }

          const element = event.currentTarget;
          const { clientX, clientY } = event;

          // A keyboard's menu key fires contextmenu at (0, 0): hang the menu from the row.
          if (clientX === 0 && clientY === 0) {
            actions.openMenuFrom(row, element, true);

            return;
          }

          afterRelease(event.buttons !== 0, () => actions.openMenuAt(row, clientX, clientY));
        }}
        onKeyDown={(event) => {
          if ((event.key === "F10" && event.shiftKey) || event.key === "ContextMenu") {
            event.preventDefault();
            actions.openMenuFrom(row, event.currentTarget, true);

            return;
          }

          actions.onKeyDown(event, row);
        }}
        onBlur={actions.onBlur}
      >
        <RowGlyph row={row} />
        <span className="sidebar-row-name">{row.displayName}</span>
        {muted ? <Icon name="bell-off" size={14} className="sidebar-row-muted" /> : null}
        <Badge
          count={pill}
          tone="danger"
          label={`${pill} ${room.kind === "direct" && !muted ? "unread" : "mentions"}`}
        />
      </Link>
      <IconButton
        icon="more"
        label={`${row.displayName} options`}
        size="sm"
        tabIndex={-1}
        aria-haspopup="menu"
        tooltipPlacement="right"
        className="sidebar-row-more"
        onClick={(event) => actions.openMenuFrom(row, event.currentTarget, event.detail === 0)}
      />
    </li>
  );
}
