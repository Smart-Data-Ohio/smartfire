import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { meFixture, messageFixture, sidebarRowFixture } from "../api/testing.ts";
import type { Involvement } from "../gen/Involvement.ts";
import type { MessageDTO } from "../gen/MessageDTO.ts";
import type { NotificationSettings } from "../gen/NotificationSettings.ts";
import type { SidebarRow } from "../gen/SidebarRow.ts";
import type { SyncEvent } from "../gen/SyncEvent.ts";
import { initialState, type State } from "../store/state.ts";
import { store } from "../store/store.ts";
import { notificationPreferencesFixture } from "../test/notification-fixtures.ts";
import {
  chimeAllowed,
  chimeGates,
  followMessageChimes,
  notifyingRooms,
  THROTTLE_MS,
} from "./message-chime.ts";
import { emitSyncEvents } from "./signals.ts";

const playMessageChime = vi.fn();

const VIEWER = meFixture.user.id;

const MAYA = 2;

const DM = 9;

const GENERAL = 3;

const NOBODY_ON_SCREEN: ReadonlySet<number> = new Set();

const PREFERENCES: NotificationSettings = {
  ...notificationPreferencesFixture,
  roomNotificationLevels: {},
  roomMuteUntil: {},
};

function row(
  roomId: number,
  change: {
    readonly kind?: SidebarRow["room"]["kind"];
    readonly involvement?: Involvement;
    readonly notificationCount?: number;
    readonly threadNotificationCount?: number;
    readonly lastReadMessageId?: number | null;
  } = {},
): SidebarRow {
  const base = sidebarRowFixture(roomId, `Room ${roomId}`, change.kind ?? "open");

  return {
    ...base,
    membership: {
      ...base.membership,
      involvement: change.involvement ?? "everything",
      lastReadMessageId: change.lastReadMessageId === undefined ? 100 : change.lastReadMessageId,
    },
    notificationCount: change.notificationCount ?? 0,
    threadNotificationCount: change.threadNotificationCount ?? 0,
  };
}

function stateWith(
  rows: readonly SidebarRow[],
  {
    preferences = PREFERENCES,
    messages = [],
  }: {
    readonly preferences?: NotificationSettings;
    readonly messages?: readonly MessageDTO[];
  } = {},
): State {
  return {
    ...initialState,
    me: meFixture,
    messages: Object.fromEntries(messages.map((message) => [message.id, message])),
    sidebar: {
      ...initialState.sidebar,
      rows: Object.fromEntries(rows.map((entry) => [entry.room.id, entry])),
      notificationPreferences: preferences,
    },
  };
}

function upserted(data: SidebarRow, seq = 10): SyncEvent {
  return { seq, topic: "user", type: "sidebar.row.upserted", data };
}

function created(data: MessageDTO, seq = 10): SyncEvent {
  return { seq, topic: `room:${data.roomId}`, type: "message.created", data };
}

/** The rooms a live batch chimes for, the row going from `before` to `after`. */
function risen(before: SidebarRow | null, after: SidebarRow, liveAfter = 0): readonly number[] {
  return notifyingRooms(
    [upserted(after)],
    liveAfter,
    stateWith(before === null ? [] : [before]),
    stateWith([after]),
    NOBODY_ON_SCREEN,
  );
}

describe("notifyingRooms from the server's push count", () => {
  it("chimes for a direct message, including the one that opens a new conversation", () => {
    const dm = { kind: "direct" } as const;

    expect(
      risen(row(DM, { ...dm, notificationCount: 1 }), row(DM, { ...dm, notificationCount: 2 })),
    ).toEqual([DM]);
    expect(risen(null, row(DM, { ...dm, notificationCount: 1 }))).toEqual([DM]);
  });

  it("chimes for a new message in an everything room", () => {
    expect(risen(row(GENERAL), row(GENERAL, { notificationCount: 1 }))).toEqual([GENERAL]);
  });

  it("chimes in a mentions-only room only when the count rises for a mention", () => {
    const mentions = { involvement: "mentions" } as const;

    expect(risen(row(GENERAL, mentions), row(GENERAL, mentions))).toEqual([]);
    expect(
      risen(row(GENERAL, mentions), row(GENERAL, { ...mentions, notificationCount: 1 })),
    ).toEqual([GENERAL]);
  });

  it("chimes for a reply in a followed thread", () => {
    expect(
      risen(row(GENERAL), row(GENERAL, { notificationCount: 1, threadNotificationCount: 1 })),
    ).toEqual([GENERAL]);
  });

  it("stays quiet when the count holds, as it does for the viewer's own message", () => {
    expect(
      risen(row(GENERAL, { notificationCount: 2 }), row(GENERAL, { notificationCount: 2 })),
    ).toEqual([]);
    expect(
      risen(row(GENERAL, { notificationCount: 2 }), row(GENERAL, { notificationCount: 0 })),
    ).toEqual([]);
  });

  it("stays quiet for rows replayed from before the connection's welcome", () => {
    expect(risen(row(GENERAL), row(GENERAL, { notificationCount: 1 }), 10)).toEqual([]);
  });

  it("stays quiet when the rise comes from marking the room unread", () => {
    expect(
      risen(row(GENERAL), row(GENERAL, { notificationCount: 3, lastReadMessageId: 90 })),
    ).toEqual([]);
  });

  it("stays quiet when the rise comes from changing the room's level", () => {
    const before = stateWith([row(GENERAL, { notificationCount: 1 })], {
      preferences: { ...PREFERENCES, roomNotificationLevels: { [GENERAL]: "mentions" } },
    });

    const after = stateWith([row(GENERAL, { notificationCount: 4 })]);

    expect(
      notifyingRooms(
        [upserted(row(GENERAL, { notificationCount: 4 }))],
        0,
        before,
        after,
        NOBODY_ON_SCREEN,
      ),
    ).toEqual([]);
  });

  it("stays quiet for a room that joins the sidebar with history already unread", () => {
    expect(risen(null, row(GENERAL, { notificationCount: 5 }))).toEqual([]);
  });
});

describe("notifyingRooms for a room on screen in an unfocused window", () => {
  const onScreen: ReadonlySet<number> = new Set([GENERAL]);

  function arrives(
    message: MessageDTO,
    {
      involvement = "everything",
      screen = onScreen,
      messages = [],
      liveAfter = 0,
    }: {
      readonly involvement?: Involvement;
      readonly screen?: ReadonlySet<number>;
      readonly messages?: readonly MessageDTO[];
      readonly liveAfter?: number;
    } = {},
  ): readonly number[] {
    const state = stateWith([row(GENERAL, { involvement })], { messages });

    return notifyingRooms([created(message)], liveAfter, state, state, screen);
  }

  const fromMaya = (change: Partial<MessageDTO> = {}) =>
    messageFixture(200, GENERAL, { creatorId: MAYA, ...change });

  it("chimes for someone else's message in an everything room", () => {
    expect(arrives(fromMaya())).toEqual([GENERAL]);
  });

  it("chimes in a mentions-only room for a mention or a reply to the viewer, and nothing else", () => {
    const mentions = { involvement: "mentions" } as const;
    const mine = messageFixture(150, GENERAL, { creatorId: VIEWER });

    expect(arrives(fromMaya(), mentions)).toEqual([]);
    expect(
      arrives(
        fromMaya({ bodyHtml: `<p><span data-user-id="${VIEWER}">@Ada</span></p>` }),
        mentions,
      ),
    ).toEqual([GENERAL]);
    expect(arrives(fromMaya({ replyToMessageId: 150 }), { ...mentions, messages: [mine] })).toEqual(
      [GENERAL],
    );
  });

  it("stays quiet for the viewer's own message, system notes, thread replies and ignored people", () => {
    expect(arrives(messageFixture(200, GENERAL, { creatorId: VIEWER }))).toEqual([]);
    expect(arrives(fromMaya({ systemNote: true }))).toEqual([]);
    expect(arrives(fromMaya({ threadId: 40 }))).toEqual([]);
    expect(arrives(fromMaya({ ignored: true }))).toEqual([]);
  });

  it("stays quiet for a nothing room, a replay, or a room this tab doesn't show", () => {
    expect(arrives(fromMaya(), { involvement: "nothing" })).toEqual([]);
    expect(arrives(fromMaya(), { liveAfter: 10 })).toEqual([]);
    expect(arrives(fromMaya(), { screen: NOBODY_ON_SCREEN })).toEqual([]);
  });
});

describe("chimeAllowed", () => {
  const open = { enabled: true, muted: false, desktop: true };
  const state = stateWith([row(GENERAL), row(DM, { kind: "direct" })]);

  it("chimes on desktop with the setting on", () => {
    expect(chimeAllowed(state, [DM], open, new Set())).toBe(true);
  });

  it("stays quiet with the setting off, while muted, or on a phone", () => {
    expect(chimeAllowed(state, [DM], { ...open, enabled: false }, new Set())).toBe(false);
    expect(chimeAllowed(state, [DM], { ...open, muted: true }, new Set())).toBe(false);
    expect(chimeAllowed(state, [DM], { ...open, desktop: false }, new Set())).toBe(false);
  });

  it("stays quiet for a snoozed room", () => {
    const snoozed = stateWith([row(DM, { kind: "direct" })], {
      preferences: { ...PREFERENCES, roomMuteUntil: { [DM]: null } },
    });

    expect(chimeAllowed(snoozed, [DM], open, new Set())).toBe(false);
  });

  it("stays quiet for the room a focused tab shows, not for another one", () => {
    expect(chimeAllowed(state, [DM], open, new Set([DM]))).toBe(false);
    expect(chimeAllowed(state, [DM], open, new Set([GENERAL]))).toBe(true);
    expect(chimeAllowed(state, [DM, GENERAL], open, new Set([DM]))).toBe(true);
  });
});

describe("chimeGates", () => {
  let coarse = false;

  beforeEach(() => {
    coarse = false;
    vi.stubGlobal("matchMedia", (query: string) => ({
      matches: coarse && query.includes("coarse"),
    }));
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("is open on desktop with the setting on and no do-not-disturb", () => {
    expect(chimeGates(stateWith([]))).toEqual({ enabled: true, muted: false, desktop: true });
  });

  it("follows the setting", () => {
    const off = stateWith([], { preferences: { ...PREFERENCES, messageSound: false } });

    expect(chimeGates(off).enabled).toBe(false);
  });

  it("mutes under do not disturb or out of office, as the server folds them into chatSounds", () => {
    const dnd = {
      ...stateWith([]),
      me: { ...meFixture, chatSounds: { ...meFixture.chatSounds, muted: true } },
    };

    expect(chimeGates(dnd).muted).toBe(true);
  });

  it("treats a touch screen as a phone", () => {
    coarse = true;

    expect(chimeGates(stateWith([])).desktop).toBe(false);
  });
});

describe("followMessageChimes", () => {
  let stop: (() => void) | null = null;
  let now = 1_000_000;

  beforeEach(() => {
    now = 1_000_000;
    vi.spyOn(Date, "now").mockImplementation(() => now);
    vi.spyOn(document, "hasFocus").mockReturnValue(false);
    vi.stubGlobal("matchMedia", () => ({ matches: false }));
    localStorage.clear();
    playMessageChime.mockClear();
    store.setState(stateWith([row(DM, { kind: "direct", notificationCount: 0 })]));
    stop = followMessageChimes(playMessageChime);
  });

  afterEach(() => {
    stop?.();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
    store.setState(initialState);
  });

  /** One DM arriving: the row's count goes up by one in the store, then the batch is emitted. */
  function dmArrives(seq: number): void {
    const before = store.getState();
    const held = before.sidebar.rows[DM];
    const next = row(DM, { kind: "direct", notificationCount: (held?.notificationCount ?? 0) + 1 });

    store.setState({
      sidebar: { ...before.sidebar, rows: { ...before.sidebar.rows, [DM]: next } },
    });
    emitSyncEvents([upserted(next, seq)], 0, before);
  }

  it("plays once for a burst, then again after the throttle", () => {
    dmArrives(1);
    now += 500;
    dmArrives(2);

    expect(playMessageChime).toHaveBeenCalledTimes(1);

    now += THROTTLE_MS;
    dmArrives(3);

    expect(playMessageChime).toHaveBeenCalledTimes(2);
  });

  it("leaves the chime to another tab that just played one", () => {
    localStorage.setItem("smartfire:message-chime:at", String(now - 100));
    dmArrives(1);

    expect(playMessageChime).not.toHaveBeenCalled();
  });

  it("stays quiet while a focused tab shows that room", () => {
    localStorage.setItem(
      "smartfire:message-chime:focused",
      JSON.stringify({ tab: "another-tab", rooms: [DM] }),
    );
    dmArrives(1);

    expect(playMessageChime).not.toHaveBeenCalled();
  });

  it("ignores a batch without the store as it was before it", () => {
    emitSyncEvents([upserted(row(DM, { kind: "direct", notificationCount: 1 }), 1)], 0);

    expect(playMessageChime).not.toHaveBeenCalled();
  });
});
