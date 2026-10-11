import { describe, expect, it } from "vitest";
import { meFixture, messageFixture, sidebarRowFixture, userFixture } from "../api/testing.ts";
import { ignoresStale, isIgnored, messageIgnored } from "./ignores.ts";
import {
  applyBlockState,
  applyEvents,
  applyPage,
  invalidateIgnores,
  receiveMessage,
  setBlocks,
} from "./reducers.ts";
import { initialState, type State } from "./state.ts";

/** Whether message 9 shows collapsed, as every view reads it. */
function collapsed(state: State): boolean {
  const message = state.messages[9];

  return message !== undefined && messageIgnored(state.ignores, message);
}

describe("personal ignore sync", () => {
  it("refreshes a missed unignore on reconnect and preserves changes newer than the snapshot", () => {
    const loaded = applyPage(
      initialState,
      4,
      {
        messages: [messageFixture(9, 4, { creatorId: 2 })],
        users: [],
        saved: [],
        before: null,
        after: null,
      },
      "replace",
    );

    const blocked = applyEvents(
      loaded,
      [
        {
          seq: 1,
          topic: "user",
          type: "blocks.updated",
          data: { viewerId: 7, userId: 2, ignoredByMe: true, revision: 3 },
        },
      ],
      0,
    );

    expect(collapsed(setBlocks(blocked, { viewerId: 7, people: [], revision: 2 }))).toBe(true);
    const reconnected = setBlocks(blocked, { viewerId: 7, people: [], revision: 4 });
    expect(collapsed(reconnected)).toBe(false);

    const replayed = applyEvents(
      reconnected,
      [
        {
          seq: 2,
          topic: "user",
          type: "blocks.updated",
          data: { viewerId: 7, userId: 2, ignoredByMe: true, revision: 3 },
        },
      ],
      0,
    );

    expect(collapsed(replayed)).toBe(false);
  });

  it("marks cached messages on other sessions and ignores stale preference events", () => {
    const original = { ...messageFixture(9, 4), creatorId: 2 };

    const state = applyPage(
      initialState,
      4,
      {
        messages: [original],
        users: [],
        saved: [],
        before: null,
        after: null,
      },
      "replace",
    );

    const blocked = applyEvents(
      state,
      [
        {
          seq: 1,
          topic: "user",
          type: "blocks.updated",
          data: { viewerId: 7, userId: 2, ignoredByMe: true, revision: 3 },
        },
      ],
      0,
    );

    expect(collapsed(blocked)).toBe(true);

    const stale = applyEvents(
      blocked,
      [
        {
          seq: 2,
          topic: "user",
          type: "blocks.updated",
          data: { viewerId: 7, userId: 2, ignoredByMe: false, revision: 2 },
        },
      ],
      0,
    );

    expect(collapsed(stale)).toBe(true);

    const unblocked = applyEvents(
      stale,
      [
        {
          seq: 3,
          topic: "user",
          type: "blocks.updated",
          data: { viewerId: 7, userId: 2, ignoredByMe: false, revision: 4 },
        },
      ],
      0,
    );

    expect(collapsed(unblocked)).toBe(false);
    expect(unblocked.messages[9]?.bodyHtml).toBe(original.bodyHtml);
  });

  it("keeps a newer preference when an older page or live update lands", () => {
    const original = { ...messageFixture(9, 4), creatorId: 2 };

    const blocked = applyEvents(
      initialState,
      [
        {
          seq: 1,
          topic: "user",
          type: "blocks.updated",
          data: { viewerId: 7, userId: 2, ignoredByMe: true, revision: 3 },
        },
      ],
      0,
    );

    const loaded = applyPage(
      blocked,
      4,
      {
        messages: [original],
        users: [],
        saved: [],
        before: null,
        after: null,
      },
      "replace",
    );

    expect(collapsed(loaded)).toBe(true);

    const unblocked = applyEvents(
      loaded,
      [
        {
          seq: 2,
          topic: "user",
          type: "blocks.updated",
          data: { viewerId: 7, userId: 2, ignoredByMe: false, revision: 4 },
        },
      ],
      0,
    );

    const arrived = receiveMessage(unblocked, {
      ...original,
      ignored: true,
      updatedAt: "2026-10-11T12:00:00.000Z",
    });

    expect(collapsed(arrived)).toBe(false);
  });

  it("drops an echo or a stale snapshot, and reads missing as not ignored once loaded", () => {
    const loaded = setBlocks(initialState, {
      viewerId: 7,
      people: [{ user: userFixture(2), ignoredByMe: true, createdAt: "2026-10-01T12:00:00Z" }],
      revision: 4,
    });

    expect(loaded.ignores).toMatchObject({
      snapshot: 4,
      people: { 2: { ignored: true, revision: 4 } },
    });

    const stopped = applyEvents(
      loaded,
      [
        {
          seq: 1,
          topic: "user",
          type: "blocks.updated",
          data: { viewerId: 7, userId: 2, ignoredByMe: false, revision: 5 },
        },
      ],
      0,
    );

    const echoed = setBlocks(stopped, { viewerId: 7, people: [], revision: 5 });

    expect(echoed.ignores.people[2]).toBeUndefined();
    expect(
      collapsed(
        applyPage(
          echoed,
          4,
          {
            messages: [messageFixture(9, 4, { creatorId: 2, ignored: true })],
            users: [],
            saved: [],
            before: null,
            after: null,
          },
          "replace",
        ),
      ),
    ).toBe(false);
    expect(setBlocks(echoed, { viewerId: 7, people: [], revision: 3 }).ignores).toBe(
      echoed.ignores,
    );
  });
});

describe("per-person ignore revisions", () => {
  const A = 2;
  const B = 3;

  const listed = (...ids: number[]) =>
    ids.map((id) => ({
      user: userFixture(id),
      ignoredByMe: true,
      createdAt: "2026-10-01T12:00:00Z",
    }));

  const event = (seq: number, userId: number, ignoredByMe: boolean, revision: number) => ({
    seq,
    topic: "user" as const,
    type: "blocks.updated" as const,
    data: { viewerId: 7, userId, ignoredByMe, revision },
  });

  const ignored = (state: State, userId: number) => isIgnored(state.ignores, userId, undefined);

  it("lands another person's change at the revision a local no-op already reported", () => {
    // B ignored as of 10. Elsewhere: unignore B (11), then ignore A (12).
    const loaded = setBlocks(initialState, { viewerId: 7, people: listed(B), revision: 10 });

    // Before those events arrive, "Stop ignoring B" here is a no-op that reports revision 12.
    const stopped = applyBlockState(loaded, {
      viewerId: 7,
      userId: B,
      ignoredByMe: false,
      revision: 12,
    });

    expect(ignored(stopped, B)).toBe(false);

    const arrived = applyEvents(stopped, [event(1, B, false, 11), event(2, A, true, 12)], 0);

    expect(ignored(arrived, A)).toBe(true);
    expect(ignored(arrived, B)).toBe(false);
  });

  it("keeps a change newer than a snapshot that lands after it", () => {
    const loaded = setBlocks(initialState, { viewerId: 7, people: listed(B), revision: 10 });
    const changed = applyEvents(loaded, [event(1, A, true, 12), event(2, B, false, 13)], 0);

    // A snapshot read at 11, before both changes, arrives late.
    const late = setBlocks(changed, { viewerId: 7, people: listed(B), revision: 11 });

    expect(late.ignores.snapshot).toBe(11);
    expect(ignored(late, A)).toBe(true);
    expect(ignored(late, B)).toBe(false);

    // One read after them takes over; one older than the last snapshot is dropped.
    const current = setBlocks(late, { viewerId: 7, people: listed(A), revision: 14 });

    expect(current.ignores.people).toEqual({ [A]: { ignored: true, revision: 14 } });
    expect(setBlocks(current, { viewerId: 7, people: listed(B), revision: 13 })).toBe(current);
  });
});

describe("ignore snapshot freshness", () => {
  it("stays stale when a reply to a request older than the miss lands", () => {
    // A request goes out at generation 1 and is held.
    const asked = invalidateIgnores(initialState);
    const heldAt = asked.ignores.invalidatedAt;

    // A reconnect without replay misses a change; the four refresh tries all fail.
    const missed = invalidateIgnores(invalidateIgnores(asked));

    expect(ignoresStale(missed.ignores)).toBe(true);

    // The held reply lands: its data merges, but it vouches only for generation 1.
    const landed = setBlocks(missed, { viewerId: 7, people: [], revision: 10 }, heldAt);

    expect(landed.ignores.snapshot).toBe(10);
    expect(ignoresStale(landed.ignores)).toBe(true);

    // One requested after the miss clears it; an older one never moves coverage back.
    const current = setBlocks(
      landed,
      { viewerId: 7, people: [], revision: 11 },
      missed.ignores.invalidatedAt,
    );

    expect(ignoresStale(current.ignores)).toBe(false);
    expect(setBlocks(current, { viewerId: 7, people: [], revision: 9 }, heldAt)).toBe(current);
  });
});

describe("whose ignore set it is", () => {
  const A = 7;
  const B = 8;

  const listed = (viewerId: number, revision: number, ...ids: number[]) => ({
    viewerId,
    people: ids.map((id) => ({
      user: userFixture(id),
      ignoredByMe: true,
      createdAt: "2026-10-01T12:00:00Z",
    })),
    revision,
  });

  /** Signed in as A, A's set loaded at 100 with user 3 ignored. */
  function signedInAsA(): State {
    const state = { ...initialState, me: { ...meFixture, user: userFixture(A) } };

    return setBlocks(state, listed(A, 100, 3), state.ignores.invalidatedAt);
  }

  it("never lets B's snapshot vouch for A's set, even when it was asked for A", () => {
    const loaded = signedInAsA();
    const asked = invalidateIgnores(loaded);
    const coveredAt = asked.ignores.invalidatedAt;

    // `/boot` failed, so nothing said B signed in; the request went out for A.
    const answered = setBlocks(asked, listed(B, 2, 4), coveredAt);

    expect(answered).toBe(asked);
    expect(ignoresStale(answered.ignores)).toBe(true);
    expect(isIgnored(answered.ignores, 4, undefined)).toBe(false);
  });

  it("drops B's changes and keeps A's own", () => {
    const loaded = signedInAsA();

    const theirs = applyBlockState(loaded, {
      viewerId: B,
      userId: 3,
      ignoredByMe: false,
      revision: 200,
    });

    expect(theirs).toBe(loaded);

    const own = applyBlockState(loaded, {
      viewerId: A,
      userId: 5,
      ignoredByMe: true,
      revision: 101,
    });

    expect(isIgnored(own.ignores, 5, undefined)).toBe(true);
  });

  it("takes its owner from the first payload when no one is signed in yet", () => {
    const first = setBlocks(initialState, listed(B, 2, 4), 0);

    expect(first.ignores.userId).toBe(B);
    expect(setBlocks(first, listed(A, 3, 3), 0)).toBe(first);
  });
});

describe("unchanged sync publications", () => {
  it("consumes room, thread and activity slots without adding unread state", () => {
    const before = {
      ...initialState,
      sidebar: { ...initialState.sidebar, rows: { 4: sidebarRowFixture(4, "Room") } },
    };

    const state = applyEvents(
      before,
      [
        {
          seq: 1,
          topic: "user",
          type: "room.unread",
          data: { roomId: 4, messageId: null, mentioned: false, unchanged: true },
        },
        {
          seq: 2,
          topic: "user",
          type: "thread.unread",
          data: { roomId: 4, threadId: 9, refreshOnly: true },
        },
        {
          seq: 3,
          topic: "user",
          type: "activity.item",
          data: {
            item: null,
            unreadCount: 0,
            unreadRevision: 0,
            evaluatedAt: "2026-10-11T00:00:00.000000000Z",
          },
        },
      ],
      0,
    );

    expect(state.sidebar.rows).toEqual(before.sidebar.rows);
    expect(state.threads).toEqual(initialState.threads);
    expect(state.activity.items).toEqual(initialState.activity.items);
  });
});
