/**
 * The chime for a new message that would notify the viewer, on desktop. Rooms in the background
 * reach the client only as sidebar rows on the `user` topic, so the trigger is the server's own
 * push count (`notificationCount`) going up on a live row. A room on screen stays read on the
 * server while it's shown, so for one in a visible but unfocused window the room's own
 * `message.created` decides, by the same rules. Every open tab hears the same events; a Web Lock
 * around a shared timestamp lets one of them play, at most once per `THROTTLE_MS`.
 */
import { Option, Schema } from "effect";
import type { MessageDTO } from "../gen/MessageDTO.ts";
import type { SidebarRow } from "../gen/SidebarRow.ts";
import type { SyncEvent } from "../gen/SyncEvent.ts";
import { COARSE_QUERY } from "../lib/breakpoints.ts";
import { mentionsViewer } from "../lib/mentions.ts";
import { messageIgnored } from "../store/ignores.ts";
import { roomMuted, roomNotificationLevel } from "../store/notification-preferences.ts";
import { serverNow } from "../store/server-clock.ts";
import type { State } from "../store/state.ts";
import { store } from "../store/store.ts";
import { chatSoundsMuted } from "./chat-sounds.ts";
import { onSyncEvents } from "./signals.ts";

export const THROTTLE_MS = 2000;

const LOCK = "smartfire:message-chime";

const LAST_KEY = "smartfire:message-chime:at";

const FOCUS_KEY = "smartfire:message-chime:focused";

function level(state: State, row: SidebarRow) {
  return roomNotificationLevel(
    state.sidebar.notificationPreferences,
    row.room.id,
    row.membership.involvement,
  );
}

/** A live row whose push count went up for a new message, not for a read or settings change. */
function rowRose(before: State, after: State, roomId: number): boolean {
  const next = after.sidebar.rows[roomId];
  const prev = before.sidebar.rows[roomId];

  if (next === undefined) {
    return false;
  }

  if (prev === undefined) {
    return next.room.kind === "direct" && next.notificationCount > 0;
  }

  return (
    next.notificationCount > prev.notificationCount &&
    next.membership.lastReadMessageId === prev.membership.lastReadMessageId &&
    level(before, prev) === level(after, next)
  );
}

/** The server's push rule for a root message in a room the viewer has on screen. */
function messageNotifies(after: State, message: MessageDTO, viewerId: number): boolean {
  const row = after.sidebar.rows[message.roomId];

  if (
    row === undefined ||
    message.threadId !== null ||
    message.creatorId === viewerId ||
    message.systemNote ||
    messageIgnored(after.ignores, message)
  ) {
    return false;
  }

  const mentioned = mentionsViewer(message.bodyHtml, viewerId);

  const repliesToViewer =
    message.replyToMessageId !== null &&
    after.messages[message.replyToMessageId]?.creatorId === viewerId;

  switch (level(after, row)) {
    case "everything":
      return true;
    case "mentions":
      return mentioned || repliesToViewer;
    case "muted":
      return mentioned;
    default:
      return false;
  }
}

/**
 * The rooms one applied batch brings a notifying message to. `unfocusedOnScreen` holds the
 * rooms this tab shows in a visible window without focus. Replayed events never count.
 */
export function notifyingRooms(
  events: readonly SyncEvent[],
  liveAfter: number,
  before: State,
  after: State,
  unfocusedOnScreen: ReadonlySet<number>,
): readonly number[] {
  const viewerId = after.me?.user.id;

  if (viewerId === undefined) {
    return [];
  }

  const rooms = new Set<number>();

  for (const event of events) {
    if (event.seq <= liveAfter) {
      continue;
    }

    if (event.type === "sidebar.row.upserted" && rowRose(before, after, event.data.room.id)) {
      rooms.add(event.data.room.id);
    }

    if (
      event.type === "message.created" &&
      unfocusedOnScreen.has(event.data.roomId) &&
      messageNotifies(after, event.data, viewerId)
    ) {
      rooms.add(event.data.roomId);
    }
  }

  return [...rooms];
}

export interface ChimeGates {
  /** The viewer's "Play a sound for new messages" setting. */
  readonly enabled: boolean;
  /** Do not disturb, quiet hours, a meeting or out of office. */
  readonly muted: boolean;
  /** Phones and tablets stay silent. */
  readonly desktop: boolean;
}

/** Whether `rooms` earn a chime, given the rooms a focused tab already shows. */
export function chimeAllowed(
  state: State,
  rooms: readonly number[],
  gates: ChimeGates,
  focusedRooms: ReadonlySet<number>,
): boolean {
  if (!gates.enabled || gates.muted || !gates.desktop) {
    return false;
  }

  const now = serverNow(state.sidebar.serverClock);

  return rooms.some(
    (roomId) =>
      !focusedRooms.has(roomId) && !roomMuted(state.sidebar.notificationPreferences, roomId, now),
  );
}

export function chimeGates(state: State): ChimeGates {
  return {
    enabled: state.sidebar.notificationPreferences?.messageSound ?? false,
    muted:
      state.me === null ||
      chatSoundsMuted(state.me.chatSounds, new Date(serverNow(state.sidebar.serverClock))),
    desktop: !window.matchMedia(COARSE_QUERY).matches,
  };
}

const onScreen = new Map<number, number>();

const tabId = crypto.randomUUID();

const decodeFocus = Schema.decodeUnknownOption(
  Schema.fromJsonString(Schema.Struct({ tab: Schema.String, rooms: Schema.Array(Schema.Number) })),
);

function read(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

function write(key: string, value: string | null): void {
  try {
    if (value === null) {
      window.localStorage.removeItem(key);
    } else {
      window.localStorage.setItem(key, value);
    }
  } catch {
    // Blocked or full storage: the tabs only lose each other's view, and may both chime.
  }
}

function focused(): boolean {
  return document.visibilityState === "visible" && document.hasFocus();
}

function sharedFocus() {
  return Option.flatMap(Option.fromNullishOr(read(FOCUS_KEY)), decodeFocus);
}

function forgetFocus(): void {
  if (Option.exists(sharedFocus(), (record) => record.tab === tabId)) {
    write(FOCUS_KEY, null);
  }
}

/** Tells the other tabs which rooms the focused one shows, so none of them chimes for those. */
function publishFocus(): void {
  if (focused()) {
    write(FOCUS_KEY, JSON.stringify({ tab: tabId, rooms: [...onScreen.keys()] }));
  } else {
    forgetFocus();
  }
}

function focusedRooms(): ReadonlySet<number> {
  if (focused()) {
    return new Set(onScreen.keys());
  }

  return new Set(
    Option.match(sharedFocus(), { onNone: () => [], onSome: (record) => record.rooms }),
  );
}

let lastLocal = 0;

/** Inside the lock: the first tab within `THROTTLE_MS` takes the chime. */
function take(state: State, rooms: readonly number[], gates: ChimeGates): boolean {
  const now = Date.now();

  if (
    !chimeAllowed(state, rooms, gates, focusedRooms()) ||
    now - Math.max(lastLocal, Number(read(LAST_KEY) ?? 0)) < THROTTLE_MS
  ) {
    return false;
  }

  write(LAST_KEY, String(now));
  lastLocal = now;

  return true;
}

function claimAndPlay(
  state: State,
  rooms: readonly number[],
  gates: ChimeGates,
  play: () => void,
): void {
  const run = () => {
    if (take(state, rooms, gates)) {
      play();
    }
  };

  if (navigator.locks === undefined) {
    run();

    return;
  }

  void navigator.locks.request(LOCK, run).catch(() => undefined);
}

/** Starts chiming for this tab with `play`; returns the stop. */
export function followMessageChimes(play: () => void): () => void {
  const stopEvents = onSyncEvents((events, liveAfter, before) => {
    if (before === undefined) {
      return;
    }

    const after = store.getState();

    const unfocused =
      document.visibilityState === "visible" && !document.hasFocus()
        ? new Set(onScreen.keys())
        : new Set<number>();

    const rooms = notifyingRooms(events, liveAfter, before, after, unfocused);

    if (rooms.length > 0) {
      claimAndPlay(after, rooms, chimeGates(after), play);
    }
  });

  window.addEventListener("focus", publishFocus);
  window.addEventListener("blur", publishFocus);
  window.addEventListener("pagehide", forgetFocus);
  document.addEventListener("visibilitychange", publishFocus);
  publishFocus();

  return () => {
    stopEvents();
    window.removeEventListener("focus", publishFocus);
    window.removeEventListener("blur", publishFocus);
    window.removeEventListener("pagehide", forgetFocus);
    document.removeEventListener("visibilitychange", publishFocus);
  };
}

/** Marks `roomId` as on screen in this tab until the returned release. */
export function holdChimeView(roomId: number): () => void {
  onScreen.set(roomId, (onScreen.get(roomId) ?? 0) + 1);
  publishFocus();

  return () => {
    const held = (onScreen.get(roomId) ?? 1) - 1;

    if (held === 0) {
      onScreen.delete(roomId);
    } else {
      onScreen.set(roomId, held);
    }

    publishFocus();
  };
}
