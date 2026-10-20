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

const LEADER_LOCK = "smartfire:message-chime:leader";

const LAST_KEY = "smartfire:message-chime:at";

const FOCUS_KEY = "smartfire:message-chime:focused";

const HANDLED_KEY = "smartfire:message-chime:handled";

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

/** Two accounts signed in on one origin keep separate records. */
function scoped(key: string): string {
  return `${key}:${store.getState().me?.user.id ?? "signed-out"}`;
}

/** None when storage is out of reach (blocked, or private mode in some browsers). */
function readStorage(key: string): Option.Option<string | null> {
  try {
    return Option.some(window.localStorage.getItem(scoped(key)));
  } catch {
    return Option.none();
  }
}

function tryRead(key: string): string | null {
  return Option.getOrNull(readStorage(key));
}

function tryWrite(key: string, value: string | null): boolean {
  try {
    if (value === null) {
      window.localStorage.removeItem(scoped(key));
    } else {
      window.localStorage.setItem(scoped(key), value);
    }

    return true;
  } catch {
    return false;
  }
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

/**
 * Whether this tab holds the leader lock, which decides the one tab that plays while the shared
 * record can't be read or written.
 */
let leading = false;

function handledAtByMessageId(
  shared: readonly (readonly [number, number])[],
  now: number,
): Map<number, number> {
  const handled = new Map<number, number>();

  for (const [messageId, at] of [...shared, ...handledLocal]) {
    if (now - at < HANDLED_TTL_MS) {
      handled.set(messageId, Math.max(at, handled.get(messageId) ?? at));
    }
  }

  return handled;
}

function newest(handled: Map<number, number>): Map<number, number> {
  return new Map([...handled].sort(([, a], [, b]) => a - b).slice(-HANDLED_MAX));
}

function claimThrottledChime(notified: readonly MessageNotified[]): boolean {
  const now = Date.now();
  const stored = readStorage(HANDLED_KEY);

  const handled = handledAtByMessageId(
    Option.getOrElse(
      Option.flatMap(Option.flatMap(stored, Option.fromNullishOr), decodeHandled),
      () => [],
    ),
    now,
  );

  const fresh = notified.filter((each) => !handled.has(each.messageId));

  for (const each of fresh) {
    handled.set(each.messageId, now);
  }

  handledLocal = newest(handled);

  const shared = Option.isSome(stored) && tryWrite(HANDLED_KEY, JSON.stringify([...handledLocal]));

  const silenced = focusedConversations();

  if (
    (!shared && !leading && navigator.locks !== undefined) ||
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

/** Queues for the leader lock until the returned resignation, held for the tab's lifetime. */
function lead(): () => void {
  if (navigator.locks === undefined) {
    return () => undefined;
  }

  const controller = new AbortController();
  let release = () => {};

  void navigator.locks
    .request(scoped(LEADER_LOCK), { signal: controller.signal }, () => {
      leading = true;

      return new Promise<void>((resolve) => {
        release = resolve;
      });
    })
    .catch(() => undefined);

  return () => {
    controller.abort();
    leading = false;
    release();
  };
}

/**
 * Plays `play` once for each live batch telling this person the server notified them of a new
 * message, unless the setting is off, the device is touch-first, the conversation is on screen
 * in a focused tab, or another tab played within {@link THROTTLE_MS}. A tab that can't sound
 * yet never takes the turn, so a tab that can still plays. While the shared record is out of
 * reach, only the tab holding the leader lock plays.
 */
export function followMessageChimes(play: () => void): () => void {
  let resign = audible() ? lead() : undefined;

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

    resign ??= lead();
    claimAndPlay(notified, play);
  });

  const heartbeat = window.setInterval(shareFocus, HEARTBEAT_MS);

  window.addEventListener("focus", shareFocus);
  window.addEventListener("blur", shareFocus);
  window.addEventListener("pagehide", forgetFocus);
  document.addEventListener("visibilitychange", shareFocus);
  shareFocus();

  return () => {
    resign?.();
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
