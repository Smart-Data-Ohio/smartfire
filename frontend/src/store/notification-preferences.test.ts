import { describe, expect, it } from "vitest";
import { sidebarRowFixture } from "../api/testing.ts";
import { notificationPreferencesFixture } from "../test/notification-fixtures.ts";
import { notificationRow, roomMuted, roomNotificationLevel } from "./notification-preferences.ts";

const now = Date.parse("2026-10-10T12:00:00Z");

const end = now + 15 * 60_000;

describe("room notification preferences", () => {
  it("inherits changes to the account default and keeps explicit and legacy room choices", () => {
    expect(roomNotificationLevel(notificationPreferencesFixture, 4, "everything")).toBe("mentions");
    expect(
      roomNotificationLevel(
        { ...notificationPreferencesFixture, defaultNotificationLevel: "nothing" },
        4,
        "everything",
      ),
    ).toBe("nothing");
    expect(roomNotificationLevel(notificationPreferencesFixture, 5, "everything")).toBe("nothing");
    expect(roomNotificationLevel(notificationPreferencesFixture, 6, "everything")).toBe(
      "everything",
    );
  });

  it("expires exactly at the deadline and keeps indefinite mutes", () => {
    expect(roomMuted(notificationPreferencesFixture, 4, end - 1)).toBe(true);
    expect(roomMuted(notificationPreferencesFixture, 4, end)).toBe(false);
    expect(roomMuted(notificationPreferencesFixture, 5, end)).toBe(true);
    expect(roomMuted(notificationPreferencesFixture, 6, now)).toBe(false);
  });

  it("suppresses every room badge without destroying the unread state it restores", () => {
    const row = {
      ...sidebarRowFixture(4, "general"),
      notificationCount: 8,
      mentionCount: 2,
      threadNotificationCount: 3,
    };

    expect(notificationRow(row, notificationPreferencesFixture, now)).toMatchObject({
      membership: { involvement: "muted", unreadAt: null },
      notificationCount: 0,
      mentionCount: 0,
      threadNotificationCount: 0,
    });
    expect(notificationRow(row, notificationPreferencesFixture, end)).toMatchObject({
      membership: { involvement: "mentions", unreadAt: row.membership.unreadAt },
      notificationCount: 8,
      mentionCount: 2,
      threadNotificationCount: 3,
    });
  });
});
