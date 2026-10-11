import { describe, expect, it } from "vitest";
import { messageFixture } from "../api/testing.ts";
import { applyEvents, applyPage, receiveMessage, setBlocks } from "./reducers.ts";
import { initialState } from "./state.ts";

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

    expect(setBlocks(blocked, { people: [], revision: 2 }).messages[9]?.ignored).toBe(true);
    const reconnected = setBlocks(blocked, { people: [], revision: 4 });
    expect(reconnected.messages[9]?.ignored).toBe(false);

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

    expect(replayed.messages[9]?.ignored).toBe(false);
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

    expect(blocked.messages[9]?.ignored).toBe(true);

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

    expect(stale.messages[9]?.ignored).toBe(true);

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

    expect(unblocked.messages[9]?.ignored).toBe(false);
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

    expect(loaded.messages[9]?.ignored).toBe(true);

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

    expect(arrived.messages[9]?.ignored).toBe(false);
  });
});
