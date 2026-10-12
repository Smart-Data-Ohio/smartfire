import { describe, expect, it } from "vitest";
import type { ActivityItem } from "../../gen/ActivityItem.ts";
import type { ActivitySource } from "../../gen/ActivitySource.ts";
import {
  activityTarget,
  emptyCopy,
  isSitePath,
  statusChip,
  targetFromPath,
} from "./activity-format.ts";
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
  it.each([
    ["/rooms/12", "/app/r/12"],
    ["/rooms/12/@9001?source=mail#message_9001", "/app/r/12/m/9001?source=mail#message_9001"],
    ["/rooms/12?thread=5&message_id=77&source=mail", "/app/r/12/t/5?m=77&source=mail"],
    ["/rooms/12?thread=5", "/app/r/12/t/5"],
    ["/scheduled_messages", "/app/scheduled"],
    ["/saved/", "/app/saved"],
    ["/agents/3/approvals", "/app/agents/3/approvals"],
    ["/users/me/profile?classic=1#security", "/app/settings#security"],
    ["/rooms/abc?source=old#top", "/app/rooms/abc?source=old#top"],
    ["/app/activity?tab=agents#top", "/app/activity?tab=agents#top"],
  ])("opens %s inside the SPA with its link context", (path, href) => {
    expect(targetFromPath(path)).toEqual({ kind: "page", href });
  });

  it.each([
    "",
    "https://example.com/rooms/12",
    "//example.com/rooms/12",
    "/\\example.com",
    "javascript:alert(1)",
  ])("refuses an unsafe activity destination %s", (path) =>
    expect(targetFromPath(path)).toEqual({ kind: "none" }),
  );
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

  it("maps remaining paths into the SPA", () => {
    expect(
      activityTarget(
        item({ sourceType: "agent_approval", roomId: null, path: "/agents/3/approvals" }),
      ),
    ).toEqual({ kind: "page", href: "/app/agents/3/approvals" });
    expect(activityTarget(item({ sourceType: "scheduled_message" }))).toEqual({
      kind: "scheduled",
    });
  });
});

describe("emptyCopy", () => {
  it("says caught up under Unread, nothing handled under Handled, nothing yet under Read", () => {
    expect(emptyCopy("all", "unread").title).toBe("You're all caught up");
    expect(emptyCopy("mentions", "unread").title).toBe("No unread mentions or replies");
    expect(emptyCopy("huddles", "read").title).toBe("No huddles yet");
    expect(emptyCopy("all", "handled").title).toBe("Nothing handled yet");
    expect(emptyCopy("github", "handled").title).toBe("No handled review requests");
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

describe("isSitePath", () => {
  it("follows a path on this site", () => {
    expect(isSitePath("/rooms/12/events/4")).toBe(true);
    expect(isSitePath("/account/sessions?x=1")).toBe(true);
  });

  it("refuses another site's URL, however it's spelled", () => {
    expect(isSitePath("https://example.com/rooms/12")).toBe(false);
    expect(isSitePath("javascript:alert(1)")).toBe(false);
    expect(isSitePath("//example.com/rooms/12")).toBe(false);
    expect(isSitePath("/\\example.com")).toBe(false);
    expect(isSitePath("/\t/example.com")).toBe(false);
    expect(isSitePath("rooms/12")).toBe(false);
    expect(isSitePath("")).toBe(false);
  });
});
