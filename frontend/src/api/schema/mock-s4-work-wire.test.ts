import { Schema } from "effect";
import { describe, expect, it } from "vitest";
import { manualScheduler } from "../../../mock/scheduler.ts";
import { createMockServer, type MockServer, SEED_IDS } from "../../../mock/server.ts";
import { SyncEvent } from "./sync.ts";
import { ThreadDetail, ThreadList } from "./thread.ts";
import { WorkList } from "./work.ts";

/** A quiet mock on a manual clock. */
function server(): MockServer {
  const clock = manualScheduler(Date.UTC(2026, 9, 6, 16, 30, 0));

  return createMockServer({ now: () => clock.now(), seed: 1, scheduler: clock });
}

const headers = (mock: MockServer) => ({ "X-CSRF-Token": mock.csrfToken() });

describe("the S4 work mock against the pinned wire schemas", () => {
  it("decodes the work list in every state", async () => {
    const mock = server();

    for (const state of ["open", "done", "all", "agents", "boards"]) {
      const response = await mock.handle({ method: "GET", path: `/api/v1/work?state=${state}` });

      expect(response.status).toBe(200);
      expect(() => Schema.decodeUnknownSync(WorkList)(response.json)).not.toThrow();
    }
  });

  it("decodes every seeded thread's detail and the rooms' thread lists", async () => {
    const mock = server();

    for (const threadId of Object.values(SEED_IDS.s4.work)) {
      const response = await mock.handle({ method: "GET", path: `/api/v1/threads/${threadId}` });

      expect(() => Schema.decodeUnknownSync(ThreadDetail)(response.json)).not.toThrow();
    }

    for (const roomId of [SEED_IDS.rooms.general, SEED_IDS.rooms.design]) {
      const response = await mock.handle({
        method: "GET",
        path: `/api/v1/rooms/${roomId}/threads?state=all`,
      });

      expect(() => Schema.decodeUnknownSync(ThreadList)(response.json)).not.toThrow();
    }
  });

  it("decodes the change and handoff replies and the events they publish", async () => {
    const mock = server();
    const frames: unknown[] = [];
    const { work } = SEED_IDS.s4;

    const connection = mock.connect((frame) => {
      if (frame.t === "batch") frames.push(...frame.events);
    });

    connection.receive({
      t: "hello",
      v: 1,
      resume: null,
      topics: [`room:${SEED_IDS.rooms.general}`, `thread:${work.untracked}`],
    });

    const replies = [
      await mock.handle({
        method: "PATCH",
        path: `/api/v1/threads/${work.untracked}/work`,
        body: { status: "planned", resultMarkdown: "Notes **here**" },
        headers: headers(mock),
      }),
      await mock.handle({
        method: "POST",
        path: `/api/v1/threads/${work.untracked}/work/handoff`,
        body: {
          receiverAgentId: SEED_IDS.bot,
          summary: "Over to you.",
          links: ["https://example.com"],
          openQuestions: [],
        },
        headers: headers(mock),
      }),
      await mock.handle({
        method: "PATCH",
        path: `/api/v1/threads/${work.untracked}/work`,
        body: { status: null, ownerId: null },
        headers: headers(mock),
      }),
    ];

    expect(replies.map((reply) => reply.status)).toEqual([200, 201, 200]);

    for (const reply of replies) {
      expect(() => Schema.decodeUnknownSync(ThreadDetail)(reply.json)).not.toThrow();
    }

    expect(frames.length).toBeGreaterThanOrEqual(6);

    for (const frame of frames) {
      expect(() => Schema.decodeUnknownSync(SyncEvent)(frame)).not.toThrow();
    }
  });
});
