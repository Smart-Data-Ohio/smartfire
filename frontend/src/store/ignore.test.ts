import { describe, expect, it } from "vitest";
import { messageFixture, sidebarRowFixture, userFixture } from "../api/testing.ts";
import { blocksRequest, ignoresStale, isIgnored, messageIgnored } from "./ignores.ts";
import {
  applyBlockState,
  applyEvents,
  applyPage,
  identifyIgnores,
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
          data: { userId: 2, ignoredByMe: true, revision: 3 },
        },
      ],
      0,
    );

    expect(collapsed(setBlocks(blocked, { people: [], revision: 2 }))).toBe(true);
    const reconnected = setBlocks(blocked, { people: [], revision: 4 });
    expect(collapsed(reconnected)).toBe(false);

    const replayed = applyEvents(
      reconnected,
      [
        {
          seq: 2,
          topic: "user",
          type: "blocks.updated",
          data: { userId: 2, ignoredByMe: true, revision: 3 },
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
          data: { userId: 2, ignoredByMe: true, revision: 3 },
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
          data: { userId: 2, ignoredByMe: false, revision: 2 },
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
          data: { userId: 2, ignoredByMe: false, revision: 4 },
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
          data: { userId: 2, ignoredByMe: true, revision: 3 },
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
          data: { userId: 2, ignoredByMe: false, revision: 4 },
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
          data: { userId: 2, ignoredByMe: false, revision: 5 },
        },
      ],
      0,
    );

    const echoed = setBlocks(stopped, { people: [], revision: 5 });

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
    expect(setBlocks(echoed, { people: [], revision: 3 }).ignores).toBe(echoed.ignores);
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
    data: { userId, ignoredByMe, revision },
  });

  const ignored = (state: State, userId: number) => isIgnored(state.ignores, userId, undefined);

  it("lands another person's change at the revision a local no-op already reported", () => {
    // B ignored as of 10. Elsewhere: unignore B (11), then ignore A (12).
    const loaded = setBlocks(initialState, { people: listed(B), revision: 10 });

    // Before those events arrive, "Stop ignoring B" here is a no-op that reports revision 12.
    const stopped = applyBlockState(loaded, { userId: B, ignoredByMe: false, revision: 12 });

    expect(ignored(stopped, B)).toBe(false);

    const arrived = applyEvents(stopped, [event(1, B, false, 11), event(2, A, true, 12)], 0);

    expect(ignored(arrived, A)).toBe(true);
    expect(ignored(arrived, B)).toBe(false);
  });

  it("keeps a change newer than a snapshot that lands after it", () => {
    const loaded = setBlocks(initialState, { people: listed(B), revision: 10 });
    const changed = applyEvents(loaded, [event(1, A, true, 12), event(2, B, false, 13)], 0);

    // A snapshot read at 11, before both changes, arrives late.
    const late = setBlocks(changed, { people: listed(B), revision: 11 });

    expect(late.ignores.snapshot).toBe(11);
    expect(ignored(late, A)).toBe(true);
    expect(ignored(late, B)).toBe(false);

    // One read after them takes over; one older than the last snapshot is dropped.
    const current = setBlocks(late, { people: listed(A), revision: 14 });

    expect(current.ignores.people).toEqual({ [A]: { ignored: true, revision: 14 } });
    expect(setBlocks(current, { people: listed(B), revision: 13 })).toBe(current);
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
    const landed = setBlocks(
      missed,
      { people: [], revision: 10 },
      { coveredAt: heldAt, userId: null },
    );

    expect(landed.ignores.snapshot).toBe(10);
    expect(ignoresStale(landed.ignores)).toBe(true);

    // One requested after the miss clears it; an older one never moves coverage back.
    const current = setBlocks(
      landed,
      { people: [], revision: 11 },
      { coveredAt: missed.ignores.invalidatedAt, userId: null },
    );

    expect(ignoresStale(current.ignores)).toBe(false);
    expect(
      setBlocks(current, { people: [], revision: 9 }, { coveredAt: heldAt, userId: null }),
    ).toBe(current);
  });
});

describe("whose ignore set it is", () => {
  const listed = (revision: number, ...ids: number[]) => ({
    people: ids.map((id) => ({
      user: userFixture(id),
      ignoredByMe: true,
      createdAt: "2026-10-01T12:00:00Z",
    })),
    revision,
  });

  const A = 1;
  const B = 8;

  it("drops A's set for B's when another sign-in reconnects without replay", () => {
    const ofA = identifyIgnores(initialState, A);
    const loadedA = setBlocks(ofA, listed(100, 3), blocksRequest(ofA.ignores, A));

    expect(isIgnored(loadedA.ignores, 3, undefined)).toBe(true);

    // Another tab signs in as B; this one reconnects without replay and asks again.
    const asked = invalidateIgnores(identifyIgnores(loadedA, B));

    expect(asked.ignores).toMatchObject({ userId: B, snapshot: null, people: {} });
    expect(ignoresStale(asked.ignores)).toBe(true);
    expect(isIgnored(asked.ignores, 3, true)).toBe(true);

    const loadedB = setBlocks(asked, listed(2, 4), blocksRequest(asked.ignores, A));

    expect(loadedB.ignores).toMatchObject({ userId: B, snapshot: 2 });
    expect(ignoresStale(loadedB.ignores)).toBe(false);
    expect(isIgnored(loadedB.ignores, 4, undefined)).toBe(true);
    expect(isIgnored(loadedB.ignores, 3, undefined)).toBe(false);
  });

  it("discards a reply read for A that lands after the switch to B", () => {
    const ofA = identifyIgnores(initialState, A);
    const loadedA = setBlocks(ofA, listed(100, 3), blocksRequest(ofA.ignores, A));
    const forA = blocksRequest(invalidateIgnores(loadedA).ignores, A);
    const switched = invalidateIgnores(identifyIgnores(loadedA, B));

    expect(setBlocks(switched, listed(101, 3, 5), forA)).toBe(switched);
    expect(ignoresStale(switched.ignores)).toBe(true);
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
