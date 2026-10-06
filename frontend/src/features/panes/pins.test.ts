import { describe, expect, it } from "vitest";
import type { PinList } from "../../gen/PinList.ts";
import { messageFixture } from "../threads/test-fixtures.ts";
import { pinnedEntries } from "./pins-pane.tsx";

const list: PinList = {
  pins: [
    { messageId: 1, pinnerId: 2, pinnedAt: "2026-10-06T10:00:00.000Z" },
    { messageId: 2, pinnerId: 2, pinnedAt: "2026-10-06T09:00:00.000Z" },
    { messageId: 3, pinnerId: 2, pinnedAt: "2026-10-06T08:00:00.000Z" },
    { messageId: 4, pinnerId: 2, pinnedAt: "2026-10-06T07:00:00.000Z" },
  ],
  messages: [1, 2, 3].map((id) => messageFixture(id, { pinned: true })),
  users: [],
};

describe("pinnedEntries", () => {
  it("pairs each pin with its message in the list's order, skipping missing messages", () => {
    expect(pinnedEntries(list, new Set(), {}).map((entry) => entry.message.id)).toEqual([1, 2, 3]);
  });

  it("drops pins unpinned here or since, by event", () => {
    const held = {
      3: messageFixture(3, { pinned: false }),
      1: messageFixture(1, { pinned: true }),
    };

    expect(pinnedEntries(list, new Set([2]), held).map((entry) => entry.message.id)).toEqual([1]);
  });
});
