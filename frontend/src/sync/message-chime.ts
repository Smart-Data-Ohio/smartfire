import { Option, Schema } from "effect";
import type { MessageNotified } from "../gen/MessageNotified.ts";
import type { SyncEvent } from "../gen/SyncEvent.ts";
import { COARSE_QUERY } from "../lib/breakpoints.ts";
import { store } from "../store/store.ts";
import { onSyncEvents } from "./signals.ts";

export const THROTTLE_MS = 2000;

/** How long another tab's focus record counts without a heartbeat, so a crashed tab's lapses. */
export const FOCUS_TTL_MS = 10_000;

const HEARTBEAT_MS = 3000;

const LOCK = "smartfire:message-chime";

const LAST_KEY = "smartfire:message-chime:at";

const FOCUS_KEY = "smartfire:message-chime:focused";

const HANDLED_KEY = "smartfire:message-chime:handled";

/** How long a tab remembers a message it chimed for or let pass, so a late tab skips it too. */
const HANDLED_TTL_MS = 5 * 60_000;

const HANDLED_MAX = 200;

type Conversation = `room:${number}` | `thread:${number}`;

function conversation(roomId: number, threadId: number | null): Conversation {
  return threadId === null ? `room:${roomId}` : `thread:${threadId}`;
}

function liveNotifications(
  events: readonly SyncEvent[],
  liveAfter: number,
): readonly MessageNotified[] {
  return events.flatMap((event) =>
    event.type === "message.notified" && event.seq > liveAfter && event.data.ignored !== true
      ? [event.data]
      : [],
  );
}

const onScreen = new Map<Conversation, number>();

const tabId = crypto.randomUUID();

const decodeFocus = Schema.decodeUnknownOption(
  Schema.fromJsonString(
    Schema.Struct({
      tab: Schema.String,
      conversations: Schema.Array(Schema.String),
      at: Schema.Number,
    }),
  ),
);

const decodeHandled = Schema.decodeUnknownOption(
  Schema.fromJsonString(Schema.Array(Schema.Tuple([Schema.Number, Schema.Number]))),
);

function tryRead(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

function tryWrite(key: string, value: string | null): void {
  try {
    if (value === null) {
      window.localStorage.removeItem(key);
    } else {
      window.localStorage.setItem(key, value);
    }
  } catch {}
}

function focused(): boolean {
  return document.visibilityState === "visible" && document.hasFocus();
}

function sharedFocus() {
  return Option.flatMap(Option.fromNullishOr(tryRead(FOCUS_KEY)), decodeFocus);
}

function forgetFocus(): void {
  if (Option.exists(sharedFocus(), (record) => record.tab === tabId)) {
    tryWrite(FOCUS_KEY, null);
  }
}

function shareFocus(): void {
  if (focused()) {
    tryWrite(
      FOCUS_KEY,
      JSON.stringify({ tab: tabId, conversations: [...onScreen.keys()], at: Date.now() }),
    );
  } else {
    forgetFocus();
  }
}

function focusedConversations(): ReadonlySet<string> {
  if (focused()) {
    return new Set(onScreen.keys());
  }

  return Option.match(
    Option.filter(sharedFocus(), (record) => Date.now() - record.at < FOCUS_TTL_MS),
    { onNone: () => new Set(), onSome: (record) => new Set(record.conversations) },
  );
}

/** Browsers play sound only on a page someone has interacted with. */
function audible(): boolean {
  return navigator.userActivation?.hasBeenActive !== false;
}

let lastLocal = 0;

let handledLocal = new Map<number, number>();

/** Message ids any tab has already chimed for or let pass, with when, newest last. */
function handledRecently(now: number): Map<number, number> {
  const shared = Option.getOrElse(
    Option.flatMap(Option.fromNullishOr(tryRead(HANDLED_KEY)), decodeHandled),
    () => [],
  );

  return new Map(
    [...shared, ...handledLocal].filter(([, at]) => now - at < HANDLED_TTL_MS).slice(-HANDLED_MAX),
  );
}

function claimThrottledChime(notified: readonly MessageNotified[]): boolean {
  const now = Date.now();
  const handled = handledRecently(now);
  const fresh = notified.filter((each) => !handled.has(each.messageId));

  for (const each of fresh) {
    handled.set(each.messageId, now);
  }

  handledLocal = handled;
  tryWrite(HANDLED_KEY, JSON.stringify([...handled]));
  const silenced = focusedConversations();

  if (
    fresh.every((each) => silenced.has(conversation(each.roomId, each.threadId))) ||
    now - Math.max(lastLocal, Number(tryRead(LAST_KEY) ?? 0)) < THROTTLE_MS
  ) {
    return false;
  }

  tryWrite(LAST_KEY, String(now));
  lastLocal = now;

  return true;
}

function claimAndPlay(notified: readonly MessageNotified[], play: () => void): void {
  const run = () => {
    if (claimThrottledChime(notified)) {
      play();
    }
  };

  if (navigator.locks === undefined) {
    run();

    return;
  }

  void navigator.locks.request(LOCK, run).catch(() => undefined);
}

/**
 * Plays `play` once for each live batch telling this person the server notified them of a new
 * message, unless the setting is off, the device is touch-first, the conversation is on screen
 * in a focused tab, or another tab played within {@link THROTTLE_MS}. A tab that can't sound
 * yet never takes the turn, so a tab that can still plays.
 */
export function followMessageChimes(play: () => void): () => void {
  const stopEvents = onSyncEvents((events, liveAfter) => {
    const notified = liveNotifications(events, liveAfter);

    if (
      notified.length === 0 ||
      store.getState().sidebar.notificationPreferences?.messageSound !== true ||
      window.matchMedia(COARSE_QUERY).matches ||
      !audible()
    ) {
      return;
    }

    claimAndPlay(notified, play);
  });

  const heartbeat = window.setInterval(shareFocus, HEARTBEAT_MS);

  window.addEventListener("focus", shareFocus);
  window.addEventListener("blur", shareFocus);
  window.addEventListener("pagehide", forgetFocus);
  document.addEventListener("visibilitychange", shareFocus);
  shareFocus();

  return () => {
    stopEvents();
    window.clearInterval(heartbeat);
    window.removeEventListener("focus", shareFocus);
    window.removeEventListener("blur", shareFocus);
    window.removeEventListener("pagehide", forgetFocus);
    document.removeEventListener("visibilitychange", shareFocus);
  };
}

/**
 * Marks a conversation as on screen in this tab until the returned release: the room's own
 * timeline, or `threadId`'s thread in it.
 */
export function holdChimeView(roomId: number, threadId: number | null = null): () => void {
  const held = conversation(roomId, threadId);

  onScreen.set(held, (onScreen.get(held) ?? 0) + 1);
  shareFocus();

  return () => {
    const count = (onScreen.get(held) ?? 1) - 1;

    if (count === 0) {
      onScreen.delete(held);
    } else {
      onScreen.set(held, count);
    }

    shareFocus();
  };
}
