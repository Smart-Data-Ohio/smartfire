import { Schema } from "effect";
import { describe, expect, it } from "vitest";
import { manualScheduler } from "../../../mock/scheduler.ts";
import { createMockServer, type MockServer } from "../../../mock/server.ts";
import { ActivityItemChanged, ActivityList, ActivityUnreadCount } from "./activity.ts";
import { ScheduledMessageList } from "./composer.ts";
import { SavedItemList } from "./saved.ts";
import { SyncEvent } from "./sync.ts";

/** A quiet mock on a manual clock. */
function server(): MockServer {
  const clock = manualScheduler(Date.UTC(2026, 9, 6, 16, 30, 0));

  return createMockServer({ now: () => clock.now(), seed: 1, scheduler: clock });
}

/** Every page of a list, following `nextCursor`. */
async function pages(mock: MockServer, path: string): Promise<unknown[]> {
  const out: unknown[] = [];
  let before: string | null = null;

  do {
    const separator = path.includes("?") ? "&" : "?";
    const suffix: string = before === null ? "" : `${separator}before=${before}`;
    const response = await mock.handle({ method: "GET", path: `${path}${suffix}` });

    expect(response.status).toBe(200);
    out.push(response.json);

    const page = Schema.decodeUnknownSync(
      Schema.Struct({ nextCursor: Schema.NullOr(Schema.String) }),
    )(response.json);

    before = page.nextCursor;
  } while (before !== null && out.length < 10);

  return out;
}

describe("the S3 mock against the pinned wire schemas", () => {
  it("decodes every activity page in every state, and the badge", async () => {
    const mock = server();

    for (const status of ["unread", "read", "handled"]) {
      for (const page of await pages(mock, `/api/v1/activity?status=${status}`)) {
        expect(() => Schema.decodeUnknownSync(ActivityList)(page)).not.toThrow();
      }
    }

    const badge = await mock.handle({ method: "GET", path: "/api/v1/activity/unread_count" });

    expect(() => Schema.decodeUnknownSync(ActivityUnreadCount)(badge.json)).not.toThrow();
  });

  it("decodes every saved page and both scheduled lists", async () => {
    const mock = server();

    for (const page of await pages(mock, "/api/v1/saved")) {
      expect(() => Schema.decodeUnknownSync(SavedItemList)(page)).not.toThrow();
    }

    for (const status of ["pending", "past"]) {
      for (const page of await pages(mock, `/api/v1/scheduled_messages?status=${status}`)) {
        expect(() => Schema.decodeUnknownSync(ScheduledMessageList)(page)).not.toThrow();
      }
    }
  });

  it("decodes a state change reply and the events the mock publishes", async () => {
    const mock = server();
    const frames: unknown[] = [];

    const connection = mock.connect((frame) => {
      if (frame.t === "batch") frames.push(...frame.events);
    });

    connection.receive({ t: "hello", v: 1, resume: null, topics: [] });

    const list = Schema.decodeUnknownSync(ActivityList)(
      (await mock.handle({ method: "GET", path: "/api/v1/activity?status=unread" })).json,
    );

    const first = list.items[0];

    expect(first).toBeDefined();

    const reply = await mock.handle({
      method: "PATCH",
      path: `/api/v1/activity/${first?.id}`,
      body: { action: "handled" },
      headers: { "X-CSRF-Token": mock.csrfToken() },
    });

    expect(() => Schema.decodeUnknownSync(ActivityItemChanged)(reply.json)).not.toThrow();

    await mock.handle({
      method: "POST",
      path: "/__mock/schedule-due",
      body: { all: true },
      headers: { "X-CSRF-Token": mock.csrfToken() },
    });
    await mock.handle({
      method: "POST",
      path: "/__mock/remind-due",
      body: { all: true },
      headers: { "X-CSRF-Token": mock.csrfToken() },
    });

    expect(frames.length).toBeGreaterThan(3);

    for (const frame of frames) {
      expect(() => Schema.decodeUnknownSync(SyncEvent)(frame)).not.toThrow();
    }

    mock.dispose();
  });
});
