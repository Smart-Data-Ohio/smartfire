import { describe, expect, it } from "vitest";
import type { ActivityItem } from "../../gen/ActivityItem.ts";
import type { ActivitySource } from "../../gen/ActivitySource.ts";
import { activityTarget, emptyCopy, statusChip, targetFromPath } from "./activity-format.ts";
import { parseActivitySearch } from "./activity-search.ts";

function source(overrides: Partial<ActivitySource>): ActivitySource {
  return {
    sourceType: "message",
    sourceId: 1,
    roomId: 12,
    threadId: null,
    messageId: 9001,
    eventId: null,
    creatorId: 4,
    title: "general",
    body: "Hello",
    occurredAt: "2026-10-06T12:00:00.000Z",
    approvalStatus: null,
    budgetCap: null,
    path: "/rooms/12/@9001",
    ...overrides,
  };
}

function item(overrides: Partial<ActivitySource>): ActivityItem {
  return {
    id: 1,
    eventType: "mention",
    state: "unread",
    readAt: null,
    handledAt: null,
    createdAt: "2026-10-06T12:00:00.000Z",
    updatedAt: "2026-10-06T12:00:00.000Z",
    source: source(overrides),
  };
}

describe("targetFromPath", () => {
  it("maps classic room, message and thread paths into the SPA", () => {
    expect(targetFromPath("/rooms/12")).toEqual({ kind: "room", roomId: 12 });
    expect(targetFromPath("/rooms/12/@9001")).toEqual({
      kind: "message",
      roomId: 12,
      messageId: 9001,
    });
    expect(targetFromPath("/rooms/12?thread=5&message_id=77")).toEqual({
      kind: "thread",
      roomId: 12,
      threadId: 5,
      messageId: 77,
    });
    expect(targetFromPath("/rooms/12?thread=5")).toEqual({
      kind: "thread",
      roomId: 12,
      threadId: 5,
      messageId: null,
    });
  });

  it("maps the scheduled and saved pages", () => {
    expect(targetFromPath("/scheduled_messages")).toEqual({ kind: "scheduled" });
    expect(targetFromPath("/saved/")).toEqual({ kind: "saved" });
  });

  it("leaves screens the SPA hasn't ported on the classic site", () => {
    expect(targetFromPath("/agents/3/approvals")).toEqual({
      kind: "classic",
      href: "/agents/3/approvals",
    });
    expect(targetFromPath("/rooms/abc")).toEqual({ kind: "classic", href: "/rooms/abc" });
    expect(targetFromPath("")).toEqual({ kind: "none" });
  });
});

describe("activityTarget", () => {
  it("opens a message at its permalink, or in its thread", () => {
    expect(activityTarget(item({}))).toEqual({ kind: "message", roomId: 12, messageId: 9001 });
    expect(activityTarget(item({ threadId: 5 }))).toEqual({
      kind: "thread",
      roomId: 12,
      threadId: 5,
      messageId: 9001,
    });
  });

  it("opens work events in their thread and huddles in their room", () => {
    expect(
      activityTarget(item({ sourceType: "work_thread_event", threadId: 5, messageId: null })),
    ).toEqual({ kind: "thread", roomId: 12, threadId: 5, messageId: null });
    expect(activityTarget(item({ sourceType: "huddle_grant", messageId: null }))).toEqual({
      kind: "room",
      roomId: 12,
    });
  });

  it("follows the classic path for the rest", () => {
    expect(
      activityTarget(
        item({ sourceType: "agent_approval", roomId: null, path: "/agents/3/approvals" }),
      ),
    ).toEqual({ kind: "classic", href: "/agents/3/approvals" });
    expect(activityTarget(item({ sourceType: "scheduled_message" }))).toEqual({
      kind: "scheduled",
    });
  });
});

describe("emptyCopy", () => {
  it("says caught up under Unread and nothing yet otherwise", () => {
    expect(emptyCopy("all", true).title).toBe("You're all caught up");
    expect(emptyCopy("mentions", true).title).toBe("No unread mentions or replies");
    expect(emptyCopy("huddles", false).title).toBe("No huddles yet");
  });
});

describe("parseActivitySearch", () => {
  it("keeps known values and drops defaults and junk", () => {
    expect(parseActivitySearch({ tab: "mentions", status: "handled" })).toEqual({
      tab: "mentions",
      status: "handled",
    });
    expect(parseActivitySearch({ tab: "all", status: "unread" })).toEqual({
      tab: undefined,
      status: undefined,
    });
    expect(parseActivitySearch({ tab: "nope", status: 3 })).toEqual({
      tab: undefined,
      status: undefined,
    });
  });
});

describe("statusChip", () => {
  it("names an approval's state or the budget that ran out", () => {
    expect(statusChip(item({ sourceType: "agent_approval", approvalStatus: "pending" }))).toEqual({
      status: "pending",
      label: "Waiting for you",
    });
    expect(statusChip(item({ sourceType: "agent_budget_notice", budgetCap: "messages" }))).toEqual({
      status: "budget",
      label: "Message cap",
    });
    expect(statusChip(item({}))).toBeNull();
  });
});
