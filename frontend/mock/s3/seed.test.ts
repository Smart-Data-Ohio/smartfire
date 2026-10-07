import { describe, expect, it } from "vitest";
import type { ActivityItem } from "../../src/gen/ActivityItem.ts";
import { locate } from "../s2/model.ts";
import { NOW } from "../s2/testing.ts";
import { BOT_ID, VIEWER_ID } from "../seed.ts";
import { buildWorld, S3_MESSAGE_IDS, S3_ROOM_IDS } from "./seed.ts";

const world = buildWorld(NOW, 1);

const items: readonly ActivityItem[] = [...world.activity.values()];

describe("the S3 seed", () => {
  it("has more than a page of read items, a few dozen unread and some handled", () => {
    const count = (state: string) => items.filter((item) => item.state === state).length;

    expect(count("read")).toBeGreaterThan(100);
    expect(count("unread")).toBeGreaterThanOrEqual(20);
    expect(count("handled")).toBeGreaterThanOrEqual(10);
  });

  it("derives each state from its timestamps, with nothing in the future", () => {
    for (const item of items) {
      const state = item.handledAt !== null ? "handled" : item.readAt !== null ? "read" : "unread";

      expect(item.state).toBe(state);
      expect(Date.parse(item.updatedAt)).toBeLessThanOrEqual(NOW);
      expect(item.updatedAt >= item.createdAt).toBe(true);
    }
  });

  it("points message items at real messages, by their creators, with the classic path", () => {
    const messageItems = items.filter((item) => item.source.sourceType === "message");

    expect(messageItems.length).toBeGreaterThan(40);

    for (const item of messageItems) {
      const location = locate(world, item.source.sourceId);

      expect(location?.message.creatorId).toBe(item.source.creatorId);
      expect(item.source).toMatchObject({
        messageId: item.source.sourceId,
        roomId: location?.message.roomId,
        threadId: location?.message.threadId,
      });
      expect(item.source.path).toContain(`/rooms/${item.source.roomId}`);
    }
  });

  it("sends GitHub review requests from Ember and mentions from people", () => {
    const reviews = items.filter((item) => item.eventType === "pr_review_request");
    const mentions = items.filter((item) => item.eventType === "mention");

    expect(reviews.length).toBeGreaterThanOrEqual(6);
    expect(reviews.every((item) => item.source.creatorId === BOT_ID)).toBe(true);
    expect(reviews.map((item) => item.source.sourceId)).toContain(S3_MESSAGE_IDS.prReviewUnread);
    expect(mentions.every((item) => item.source.creatorId !== VIEWER_ID)).toBe(true);
  });

  it("fills agent and security sources the way the presenter does", () => {
    const approvals = items.filter((item) => item.eventType === "agent_approval_request");
    const budgets = items.filter((item) => item.eventType === "agent_budget_exceeded");
    const security = items.filter((item) => item.source.sourceType === "session");

    expect(new Set(approvals.map((item) => item.source.approvalStatus))).toEqual(
      new Set(["pending", "approved", "denied", "expired", "cancelled"]),
    );
    expect(
      budgets.every((item) => item.source.budgetCap !== null && item.source.roomId === null),
    ).toBe(true);
    expect(security.every((item) => item.source.path === "/users/me/sessions")).toBe(true);
  });

  it("leaves #q3-offsite invisible to the viewer", () => {
    const offsite = world.rooms.get(S3_ROOM_IDS.offsite);

    expect(offsite?.membership.involvement).toBe("invisible");
    expect(offsite?.memberIds).not.toContain(VIEWER_ID);
  });

  it("saves more than a page of messages, in threads and direct messages too", () => {
    const saved = [...world.saved.values()];
    const located = saved.map((item) => locate(world, item.messageId));

    expect(saved.length).toBeGreaterThan(50);
    expect(located.every((location) => location !== null)).toBe(true);
    expect(located.some((location) => location?.thread !== null)).toBe(true);
    expect(located.some((location) => location?.room.room.kind === "direct")).toBe(true);
    expect(saved.some((item) => item.status === "done")).toBe(true);
  });
});
