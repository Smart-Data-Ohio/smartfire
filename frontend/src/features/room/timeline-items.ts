import { dayKey, formatDay, toMillis } from "../../lib/time.ts";
import type { MessageDTO, PendingMessage, Timeline } from "../../store/model.ts";

/** Consecutive messages from one author within this window share a header. */
export const GROUP_WINDOW_MS = 5 * 60_000;

/** A row in the virtualized timeline. Keys are stable across reconciliation. */
export type TimelineItem =
  | { readonly kind: "intro"; readonly key: "intro" }
  /** Under an intro that stays first (a board post's work): loads the replies before the window. */
  | { readonly kind: "earlier"; readonly key: "earlier" }
  | { readonly kind: "loading"; readonly key: "loading-older" | "loading-newer" }
  | { readonly kind: "day"; readonly key: string; readonly label: string }
  | { readonly kind: "unread"; readonly key: "unread"; readonly count: number }
  | {
      readonly kind: "message";
      /** `c-<clientMessageId>`: the same key while pending and once confirmed, so the row never remounts. */
      readonly key: string;
      readonly message: MessageDTO;
      /** Starts a group: shows the avatar, name and time. */
      readonly groupStart: boolean;
    }
  | {
      readonly kind: "pending";
      readonly key: string;
      readonly pending: PendingMessage;
      readonly groupStart: boolean;
    };

interface Previous {
  readonly creatorId: number;
  readonly millis: number;
  readonly day: string;
  readonly quiet: boolean;
}

/** The key a message or pending row renders under. */
export function messageKey(clientMessageId: string): string {
  return `c-${clientMessageId}`;
}

function continues(previous: Previous | null, creatorId: number, millis: number): boolean {
  return (
    previous !== null &&
    !previous.quiet &&
    previous.creatorId === creatorId &&
    millis - previous.millis < GROUP_WINDOW_MS
  );
}

interface TimelineInput {
  readonly timeline: Timeline;
  readonly messages: Readonly<Record<number, MessageDTO>>;
  readonly pending: readonly PendingMessage[];
  readonly now: number;
  /**
   * The intro leads whatever the window holds, with an "earlier" row after it while older replies
   * remain (a board post: its work belongs above the discussion, however long that is).
   */
  readonly introFirst?: boolean;
}

/**
 * Lays a room's loaded window out as rows: the room intro at the very start, a spinner row at an
 * edge that's still loading, day dividers, the unread divider before the first unread message,
 * and messages grouped by author. Pending sends follow the confirmed messages once the window
 * reaches the present or while its replacement page loads.
 */
export function timelineItems({
  timeline,
  messages,
  pending,
  now,
  introFirst = false,
}: TimelineInput): TimelineItem[] {
  const items: TimelineItem[] = [];
  let previous: Previous | null = null;

  if (timeline.before === null || introFirst) {
    items.push({ kind: "intro", key: "intro" });
  }

  if (timeline.before !== null && timeline.loadingOlder) {
    items.push({ kind: "loading", key: "loading-older" });
  } else if (timeline.before !== null && introFirst) {
    items.push({ kind: "earlier", key: "earlier" });
  }

  const place = (creatorId: number, createdAt: string, quiet: boolean): boolean => {
    const millis = toMillis(createdAt);
    const day = dayKey(millis);
    let breaks = false;

    if (previous?.day !== day) {
      items.push({ kind: "day", key: `day-${day}`, label: formatDay(millis, now) });
      breaks = true;
    }

    const groupStart = breaks || quiet || !continues(previous, creatorId, millis);

    previous = { creatorId, millis, day, quiet };

    return groupStart;
  };

  for (const id of timeline.ids) {
    const message = messages[id];

    if (message === undefined) {
      continue;
    }

    let forced = false;

    if (id === timeline.unreadFromId) {
      items.push({ kind: "unread", key: "unread", count: timeline.unreadCount });
      forced = true;
    }

    const groupStart = place(message.creatorId, message.createdAt, message.systemNote) || forced;

    items.push({ kind: "message", key: messageKey(message.clientMessageId), message, groupStart });
  }

  if (timeline.after !== null) {
    if (timeline.loadingNewer) {
      items.push({ kind: "loading", key: "loading-newer" });
    }

    if (timeline.arrived === null) {
      return items;
    }
  }

  for (const entry of pending) {
    const groupStart = place(entry.creatorId, entry.createdAt, false);

    items.push({
      kind: "pending",
      key: messageKey(entry.clientMessageId),
      pending: entry,
      groupStart,
    });
  }

  return items;
}

/** What a list committed last render, for telling a prepend from an append. */
export interface CommittedEdges {
  readonly first: string | null;
  readonly firstMessage: string | null;
}

/** The key of the first message row, or `null`. */
export function firstMessageKey(items: readonly TimelineItem[]): string | null {
  return items.find((item) => item.kind === "message")?.key ?? null;
}

/**
 * Whether rows were added or removed before what was on top (an older page landing, its spinner
 * coming or going), so the virtualiser keeps the view from the end instead of jumping. Either
 * the first row or the first message moved down: when an older page replaces its spinner, the
 * spinner (the old first row) is gone but the old first message sits further down.
 */
export function prepended(items: readonly TimelineItem[], previous: CommittedEdges): boolean {
  const movedDown = (key: string | null, now: string | null) =>
    key !== null && now !== key && items.findIndex((item) => item.key === key) > 0;

  return (
    movedDown(previous.first, items[0]?.key ?? null) ||
    movedDown(previous.firstMessage, firstMessageKey(items))
  );
}
