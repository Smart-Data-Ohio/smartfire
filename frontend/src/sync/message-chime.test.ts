import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { meFixture, messageFixture, sidebarRowFixture } from "../api/testing.ts";
import type { Involvement } from "../gen/Involvement.ts";
import type { MessageDTO } from "../gen/MessageDTO.ts";
import type { NotificationSettings } from "../gen/NotificationSettings.ts";
import type { SidebarRow } from "../gen/SidebarRow.ts";
import type { SyncEvent } from "../gen/SyncEvent.ts";
import { initialState, type State } from "../store/state.ts";
import { notificationPreferencesFixture } from "../test/notification-fixtures.ts";
import { THROTTLE_MS } from "./message-chime.ts";

const MAYA = 2;

const DM = 9;

const GENERAL = 3;

const THREAD = 40;

/** Longer than any focus record may go without its tab's heartbeat. */
const LONG_AFTER_A_CRASH_MS = 60_000;

const PREFERENCES: NotificationSettings = {
  ...notificationPreferencesFixture,
  roomNotificationLevels: {},
  roomMuteUntil: {},
  messageSound: true,
};

function row(
  roomId: number,
  change: {
    readonly kind?: SidebarRow["room"]["kind"];
    readonly involvement?: Involvement;
    readonly notificationCount?: number;
    readonly threadNotificationCount?: number;
  } = {},
): SidebarRow {
  const base = sidebarRowFixture(roomId, `Room ${roomId}`, change.kind ?? "open");

  return {
    ...base,
    membership: {
      ...base.membership,
      involvement: change.involvement ?? "everything",
      lastReadMessageId: 100,
    },
    notificationCount: change.notificationCount ?? 0,
    threadNotificationCount: change.threadNotificationCount ?? 0,
  };
}

function stateWith(
  rows: readonly SidebarRow[],
  preferences: NotificationSettings = PREFERENCES,
): State {
  return {
    ...initialState,
    me: meFixture,
    sidebar: {
      ...initialState.sidebar,
      rows: Object.fromEntries(rows.map((entry) => [entry.room.id, entry])),
      notificationPreferences: preferences,
    },
  };
}

function upserted(data: SidebarRow, seq: number): SyncEvent {
  return { seq, topic: "user", type: "sidebar.row.upserted", data };
}

function created(data: MessageDTO, seq: number): SyncEvent {
  const topic = data.threadId === null ? `room:${data.roomId}` : `thread:${data.threadId}`;

  return { seq, topic, type: "message.created", data };
}

function updated(data: MessageDTO, seq: number): SyncEvent {
  const topic = data.threadId === null ? `room:${data.roomId}` : `thread:${data.threadId}`;

  return { seq, topic, type: "message.updated", data };
}

function notified(message: MessageDTO, seq: number): SyncEvent {
  return {
    seq,
    topic: "user",
    type: "message.notified",
    data: { messageId: message.id, roomId: message.roomId, threadId: message.threadId },
  };
}

const fromMaya = (id: number, roomId: number, change: Partial<MessageDTO> = {}) =>
  messageFixture(id, roomId, { creatorId: MAYA, ...change });

/** A batch as the engine hands it over: the rows it carries are in the store by then. */
interface Batch {
  readonly events: readonly SyncEvent[];
  readonly rows?: readonly SidebarRow[];
  readonly liveAfter?: number;
}

interface Tab {
  readonly played: () => number;
  readonly deliver: (batch: Batch) => void;
  readonly view: (roomId: number, threadId?: number | null) => () => void;
  readonly close: () => void;
}

const open: Tab[] = [];

let focused = false;

/**
 * Loads the module afresh, as another tab of the app would: tabs share only `localStorage`.
 * A tab left open without `close` is one that crashed.
 */
async function openTab(state: State): Promise<Tab> {
  vi.resetModules();
  const chime = await import("./message-chime.ts");
  const { emitSyncEvents } = await import("./signals.ts");
  const { store } = await import("../store/store.ts");
  const play = vi.fn();

  store.setState(state);
  const stop = chime.followMessageChimes(play);

  const tab: Tab = {
    played: () => play.mock.calls.length,
    deliver({ events, rows = [], liveAfter = 0 }) {
      const before = store.getState();

      store.setState({
        sidebar: {
          ...before.sidebar,
          rows: { ...before.sidebar.rows, ...Object.fromEntries(rows.map((r) => [r.room.id, r])) },
        },
      });
      emitSyncEvents(events, liveAfter);
    },
    view: (roomId, threadId) => chime.holdChimeView(roomId, threadId),
    close: stop,
  };

  open.push(tab);

  return tab;
}

function canSound(active: boolean): void {
  Object.defineProperty(navigator, "userActivation", {
    configurable: true,
    value: { hasBeenActive: active, isActive: false },
  });
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date", "setInterval", "clearInterval"] });
  vi.setSystemTime(1_000_000);
  focused = false;
  vi.spyOn(document, "hasFocus").mockImplementation(() => focused);
  vi.stubGlobal("matchMedia", () => ({ matches: false }));
  canSound(true);
  localStorage.clear();
});

afterEach(() => {
  for (const tab of open.splice(0)) {
    tab.close();
  }

  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  Reflect.deleteProperty(navigator, "userActivation");
});

function directMessage(id: number, count: number, seq: number): Batch {
  const message = fromMaya(id, DM);
  const next = row(DM, { kind: "direct", notificationCount: count });

  return {
    events: [created(message, seq), upserted(next, seq + 1), notified(message, seq + 2)],
    rows: [next],
  };
}

describe("followMessageChimes plays when the server notified this person", () => {
  it("chimes for a second reply in a followed thread that is already unread", async () => {
    const unread = row(GENERAL, { notificationCount: 1, threadNotificationCount: 1 });
    const tab = await openTab(stateWith([unread]));
    const reply = fromMaya(201, GENERAL, { threadId: THREAD });

    tab.deliver({
      events: [created(reply, 1), upserted(unread, 2), notified(reply, 3)],
      rows: [unread],
    });

    expect(tab.played()).toBe(1);
  });

  it("stays quiet when an old activity item is marked unread", async () => {
    const tab = await openTab(stateWith([row(GENERAL)]));
    const marked = row(GENERAL, { notificationCount: 1 });

    tab.deliver({ events: [upserted(marked, 1)], rows: [marked] });

    expect(tab.played()).toBe(0);
  });

  it("stays quiet when an unread message is edited to mention this person", async () => {
    const mentions = { involvement: "mentions" } as const;
    const tab = await openTab(stateWith([row(GENERAL, mentions)]));
    const edited = fromMaya(200, GENERAL, {
      bodyHtml: `<p><span data-user-id="${meFixture.user.id}">@Ada</span></p>`,
    });

    const raised = row(GENERAL, { ...mentions, notificationCount: 1 });

    tab.deliver({ events: [updated(edited, 1), upserted(raised, 2)], rows: [raised] });

    expect(tab.played()).toBe(0);
  });

  it("stays quiet for a user id inside a code block in a mentions-only room on screen", async () => {
    const mentions = row(GENERAL, { involvement: "mentions" });
    const tab = await openTab(stateWith([mentions]));
    const code = fromMaya(200, GENERAL, {
      bodyHtml: `<pre><code>data-user-id="${meFixture.user.id}"</code></pre>`,
    });

    tab.view(GENERAL);
    tab.deliver({ events: [created(code, 1), upserted(mentions, 2)], rows: [mentions] });

    expect(tab.played()).toBe(0);
  });

  it("chimes once for a streaming agent message, when it's finalised", async () => {
    const tab = await openTab(stateWith([row(GENERAL)]));
    const placeholder = fromMaya(200, GENERAL, { streaming: true, bodyHtml: "" });

    tab.view(GENERAL);
    tab.deliver({ events: [created(placeholder, 1)] });

    expect(tab.played()).toBe(0);

    const final = { ...placeholder, streaming: false, bodyHtml: "<p>Done</p>" };

    tab.deliver({ events: [updated(final, 2), notified(final, 3)] });

    expect(tab.played()).toBe(1);
  });

  it("stays quiet for notifications replayed from before the welcome", async () => {
    const tab = await openTab(stateWith([row(DM, { kind: "direct" })]));

    tab.deliver({ ...directMessage(200, 1, 1), liveAfter: 10 });

    expect(tab.played()).toBe(0);
  });

  it("plays once for a burst, then again after the throttle", async () => {
    const tab = await openTab(stateWith([row(DM, { kind: "direct" })]));

    tab.deliver(directMessage(200, 1, 1));
    vi.advanceTimersByTime(500);
    tab.deliver(directMessage(201, 2, 4));

    expect(tab.played()).toBe(1);

    vi.advanceTimersByTime(THROTTLE_MS);
    tab.deliver(directMessage(202, 3, 7));

    expect(tab.played()).toBe(2);
  });

  it("stays quiet with the setting off or on a touch screen", async () => {
    const off = await openTab(
      stateWith([row(DM, { kind: "direct" })], { ...PREFERENCES, messageSound: false }),
    );

    off.deliver(directMessage(200, 1, 1));

    expect(off.played()).toBe(0);

    vi.stubGlobal("matchMedia", (query: string) => ({ matches: query.includes("coarse") }));
    const phone = await openTab(stateWith([row(DM, { kind: "direct" })]));

    phone.deliver(directMessage(201, 1, 1));

    expect(phone.played()).toBe(0);
  });
});

describe("followMessageChimes and the conversation on screen", () => {
  it("chimes for a thread whose pane is closed while its room is on screen in a focused tab", async () => {
    focused = true;
    const raised = row(GENERAL, { notificationCount: 1, threadNotificationCount: 1 });
    const tab = await openTab(stateWith([row(GENERAL)]));
    const reply = fromMaya(201, GENERAL, { threadId: THREAD });

    tab.view(GENERAL);
    tab.deliver({
      events: [created(reply, 1), upserted(raised, 2), notified(reply, 3)],
      rows: [raised],
    });

    expect(tab.played()).toBe(1);
  });

  it("stays quiet for the room or the thread a focused tab shows", async () => {
    focused = true;
    const tab = await openTab(stateWith([row(GENERAL)]));
    const inRoom = fromMaya(200, GENERAL);
    const inThread = fromMaya(201, GENERAL, { threadId: THREAD });

    tab.view(GENERAL);
    tab.view(GENERAL, THREAD);
    tab.deliver({ events: [created(inRoom, 1), notified(inRoom, 2)] });
    vi.advanceTimersByTime(THROTTLE_MS);
    tab.deliver({ events: [created(inThread, 3), notified(inThread, 4)] });

    expect(tab.played()).toBe(0);
  });

  it("chimes for the conversation on screen while the window is out of focus", async () => {
    const tab = await openTab(stateWith([row(GENERAL)]));
    const message = fromMaya(200, GENERAL);

    tab.view(GENERAL);
    tab.deliver({ events: [created(message, 1), notified(message, 2)] });

    expect(tab.played()).toBe(1);
  });
});

describe("followMessageChimes across tabs", () => {
  it("lets a tab that can sound play when a tab that can't saw the message first", async () => {
    const state = stateWith([row(DM, { kind: "direct" })]);
    const silent = await openTab(state);
    const audible = await openTab(state);
    const batch = directMessage(200, 1, 1);

    canSound(false);
    silent.deliver(batch);
    canSound(true);
    audible.deliver(batch);

    expect([silent.played(), audible.played()]).toEqual([0, 1]);
  });

  it("plays once between two tabs that both can", async () => {
    const state = stateWith([row(DM, { kind: "direct" })]);
    const first = await openTab(state);
    const second = await openTab(state);
    const batch = directMessage(200, 1, 1);

    first.deliver(batch);
    second.deliver(batch);

    expect(first.played() + second.played()).toBe(1);
  });

  it("stays quiet while another tab shows that conversation focused, beat after beat", async () => {
    const state = stateWith([row(DM, { kind: "direct" })]);
    const background = await openTab(state);

    vi.clearAllTimers();
    focused = true;
    const reading = await openTab(state);

    reading.view(DM);
    vi.advanceTimersByTime(LONG_AFTER_A_CRASH_MS);
    focused = false;
    background.deliver(directMessage(200, 1, 1));

    expect(background.played()).toBe(0);
  });

  it("forgets a focused tab that crashed without cleaning up", async () => {
    const state = stateWith([row(DM, { kind: "direct" })]);

    focused = true;
    const crashed = await openTab(state);

    crashed.view(DM);
    vi.clearAllTimers();
    focused = false;
    const survivor = await openTab(state);

    vi.setSystemTime(Date.now() + LONG_AFTER_A_CRASH_MS);
    survivor.deliver(directMessage(200, 1, 1));

    expect(survivor.played()).toBe(1);
  });
});
