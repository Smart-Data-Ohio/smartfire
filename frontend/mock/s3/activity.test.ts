import { describe, expect, it } from "vitest";
import type { ActivityItem } from "../../src/gen/ActivityItem.ts";
import type { ActivityItemChanged } from "../../src/gen/ActivityItemChanged.ts";
import type { ActivityList } from "../../src/gen/ActivityList.ts";
import type { ActivityTab } from "../../src/gen/ActivityTab.ts";
import type { ActivityUnreadCount } from "../../src/gen/ActivityUnreadCount.ts";
import { collect, errorOf, expectStatus, get, harness, NOW, send } from "../s2/testing.ts";
import type { MockServer } from "../server.ts";
import { ACTIVITY_PAGE_SIZE, TAB_TYPES } from "./activity.ts";
import { encodeCursor } from "./model.ts";

const TABS: readonly ActivityTab[] = [
  "all",
  "mentions",
  "threads",
  "events",
  "agents",
  "github",
  "huddles",
  "reminders",
  "security",
];

/** Every item of one list, page by page. */
async function everyPage(server: MockServer, query: string): Promise<ActivityList[]> {
  const pages: ActivityList[] = [];
  let before: string | null = null;

  do {
    const cursor: string = before === null ? "" : `&before=${before}`;

    const page: ActivityList = await get<ActivityList>(
      server,
      `/api/v1/activity?${query}${cursor}`,
    );

    pages.push(page);
    before = page.nextCursor;
  } while (before !== null && pages.length < 10);

  return pages;
}

function newestFirst(items: readonly ActivityItem[]): boolean {
  return items.every((item, index) => {
    const previous = items[index - 1];

    return (
      previous === undefined ||
      previous.updatedAt > item.updatedAt ||
      (previous.updatedAt === item.updatedAt && previous.id > item.id)
    );
  });
}

async function firstUnread(server: MockServer): Promise<ActivityItem> {
  const page = await get<ActivityList>(server, "/api/v1/activity?status=unread");
  const item = page.items[0];

  if (item === undefined) throw new Error("the seed has no unread item");

  return item;
}

describe("the activity list", () => {
  it("pages read items 100 at a time, newest first, with a cursor only when more follow", async () => {
    const { server } = harness();
    const pages = await everyPage(server, "status=read&type=all");
    const items = pages.flatMap((page) => page.items);

    expect(pages.length).toBeGreaterThanOrEqual(2);
    expect(pages[0]?.items).toHaveLength(ACTIVITY_PAGE_SIZE);
    expect(pages.at(-1)?.nextCursor).toBeNull();
    expect(new Set(items.map((item) => item.id)).size).toBe(items.length);
    expect(newestFirst(items)).toBe(true);
    expect(items.every((item) => item.state === "read")).toBe(true);
  });

  it("defaults to unread in every type, and reads unknown values as the defaults", async () => {
    const { server } = harness();
    const plain = await get<ActivityList>(server, "/api/v1/activity");
    const odd = await get<ActivityList>(server, "/api/v1/activity?status=nope&type=nope");

    expect(plain.items.length).toBeGreaterThan(0);
    expect(plain.items.every((item) => item.state === "unread")).toBe(true);
    expect(odd.items.map((item) => item.id)).toEqual(plain.items.map((item) => item.id));
    expect(plain.unreadCount).toBe(plain.items.length);
  });

  it("fills every tab, keeps each to its types and brings the creators", async () => {
    const { server } = harness();

    for (const tab of TABS) {
      const items: ActivityItem[] = [];

      for (const status of ["unread", "read", "handled"]) {
        const pages = await everyPage(server, `status=${status}&type=${tab}`);

        items.push(...pages.flatMap((page) => page.items));

        for (const page of pages) {
          const userIds = new Set(page.users.map((user) => user.id));

          for (const item of page.items) {
            if (item.source.creatorId !== null) expect(userIds).toContain(item.source.creatorId);
          }
        }
      }

      expect(items.length, tab).toBeGreaterThan(0);

      if (tab !== "all") {
        expect(items.every((item) => TAB_TYPES[tab].includes(item.eventType))).toBe(true);
      }
    }
  });

  it("refuses a cursor that doesn't decode, and resumes from one that does", async () => {
    const { server } = harness();
    const bad = await server.handle({ method: "GET", path: "/api/v1/activity?before=%%%" });
    const all = (await everyPage(server, "status=read")).flatMap((page) => page.items);
    const third = all[2];

    expect(bad.status).toBe(422);
    expect(third).toBeDefined();

    if (third === undefined) return;

    const after = await get<ActivityList>(
      server,
      `/api/v1/activity?status=read&before=${encodeCursor({ at: third.updatedAt, id: third.id })}`,
    );

    expect(after.items[0]?.id).toBe(all[3]?.id);
  });
});

describe("activity state changes", () => {
  it("reads, handles, clears handled and unreads, publishing each change with the count", async () => {
    const { server } = harness();
    const events = collect(server);
    const item = await firstUnread(server);
    const { unreadCount } = await get<ActivityUnreadCount>(server, "/api/v1/activity/unread_count");
    const path = `/api/v1/activity/${item.id}`;

    const read = await expectStatus<ActivityItemChanged>(
      server,
      "PATCH",
      path,
      { action: "read" },
      200,
    );

    expect(read.item).toMatchObject({
      state: "read",
      handledAt: null,
      updatedAt: new Date(NOW).toISOString(),
    });
    expect(read.unreadCount).toBe(unreadCount - 1);
    expect(events.at(-1)).toMatchObject({ type: "activity.item", data: read });

    const handled = await expectStatus<ActivityItemChanged>(
      server,
      "PATCH",
      path,
      { action: "handled" },
      200,
    );

    expect(handled.item.state).toBe("handled");
    expect(handled.item.readAt).toBe(read.item.readAt);

    const stillHandled = await expectStatus<ActivityItemChanged>(
      server,
      "PATCH",
      path,
      { action: "read" },
      200,
    );

    expect(stillHandled.item).toEqual(handled.item);

    const cleared = await expectStatus<ActivityItemChanged>(
      server,
      "PATCH",
      path,
      { action: "unhandled" },
      200,
    );

    expect(cleared.item).toMatchObject({
      state: "read",
      readAt: read.item.readAt,
      handledAt: null,
    });

    const unread = await expectStatus<ActivityItemChanged>(
      server,
      "PATCH",
      path,
      { action: "unread" },
      200,
    );

    expect(unread.item).toMatchObject({ state: "unread", readAt: null, handledAt: null });
    expect(unread.unreadCount).toBe(unreadCount);
  });

  it("opens an item by marking it read, never handled", async () => {
    const { server } = harness();
    const item = await firstUnread(server);

    const opened = await expectStatus<ActivityItemChanged>(
      server,
      "POST",
      `/api/v1/activity/${item.id}/open`,
      null,
      200,
    );

    expect(opened.item.state).toBe("read");
  });

  it("answers 404 for an unknown item and 422 for an unknown action", async () => {
    const { server } = harness();
    const item = await firstUnread(server);
    const missing = await send(server, "PATCH", "/api/v1/activity/999999", { action: "read" });
    const wrong = await send(server, "PATCH", `/api/v1/activity/${item.id}`, { state: "read" });

    expect(missing.status).toBe(404);
    expect(wrong.status).toBe(422);
    expect(errorOf(wrong.json).tag).toBe("Validation");
  });

  it("records live arrivals on the control", async () => {
    const { server } = harness();
    const events = collect(server);
    const before = await get<ActivityUnreadCount>(server, "/api/v1/activity/unread_count");

    await send(server, "POST", "/__mock/activity-arrival");

    const arrived = events.find((event) => event.type === "activity.item");
    const after = await get<ActivityUnreadCount>(server, "/api/v1/activity/unread_count");

    expect(arrived).toBeDefined();
    expect(after.unreadCount).toBe(before.unreadCount + 1);
  });
});
