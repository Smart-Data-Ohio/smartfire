import { Schema } from "effect";
import { describe, expect, it } from "vitest";
import type { JsonRecord } from "../../../mock/json.ts";
import { AGENT_IDS } from "../../../mock/s4/agents.ts";
import { manualScheduler } from "../../../mock/scheduler.ts";
import { ROOM_IDS } from "../../../mock/seed.ts";
import { createMockServer, type MockServer } from "../../../mock/server.ts";
import { AgentDirectory, AgentProfile } from "./agents.ts";
import { MessagePage } from "./message.ts";
import { SyncEvent } from "./sync.ts";

/** A quiet mock on a manual clock. */
function server(): MockServer {
  const clock = manualScheduler(Date.UTC(2026, 9, 6, 16, 30, 0));

  return createMockServer({ now: () => clock.now(), seed: 1, scheduler: clock });
}

async function control(mock: MockServer, action: string, body: JsonRecord) {
  const response = await mock.handle({
    method: "POST",
    path: `/__mock/${action}`,
    body,
    headers: { "X-CSRF-Token": mock.csrfToken() },
  });

  expect(response.status).toBe(200);
}

describe("the S4 agent mock against the pinned wire schemas", () => {
  it("decodes the directory and every agent's profile", async () => {
    const mock = server();
    const directory = await mock.handle({ method: "GET", path: "/api/v1/agents" });

    expect(directory.status).toBe(200);
    expect(() => Schema.decodeUnknownSync(AgentDirectory)(directory.json)).not.toThrow();

    for (const agentId of Object.values(AGENT_IDS)) {
      const profile = await mock.handle({ method: "GET", path: `/api/v1/agents/${agentId}` });

      expect(profile.status).toBe(200);
      expect(() => Schema.decodeUnknownSync(AgentProfile)(profile.json)).not.toThrow();
    }

    const missing = await mock.handle({ method: "GET", path: "/api/v1/agents/999" });

    expect(missing.status).toBe(404);
    mock.dispose();
  });

  it("decodes the seeded agent messages with their steps", async () => {
    const mock = server();

    const page = Schema.decodeUnknownSync(MessagePage)(
      (await mock.handle({ method: "GET", path: `/api/v1/rooms/${ROOM_IDS.dmEmber}/messages` }))
        .json,
    );

    expect(page.messages.some((message) => message.steps.length > 0)).toBe(true);
    mock.dispose();
  });

  it("decodes the agent.status and agent.steps events the controls publish", async () => {
    const mock = server();
    const frames: unknown[] = [];

    const connection = mock.connect((frame) => {
      if (frame.t === "batch") frames.push(...frame.events);
    });

    connection.receive({ t: "hello", v: 1, resume: null, topics: [`room:${ROOM_IDS.dmEmber}`] });

    await control(mock, "agent-status", {
      agentId: AGENT_IDS.ember,
      status: "working",
      presence: "Reading the deploy log",
    });
    await control(mock, "agent-status", { agentId: AGENT_IDS.quill, suspended: false });

    for (const stage of [0, 1, 2, 3]) {
      await control(mock, "agent-steps", { roomId: ROOM_IDS.dmEmber, stage });
    }

    const types = frames.map(
      (frame) => Schema.decodeUnknownSync(Schema.Struct({ type: Schema.String }))(frame).type,
    );

    expect(types.filter((type) => type === "agent.status")).toHaveLength(2);
    expect(types.filter((type) => type === "agent.steps")).toHaveLength(4);

    for (const frame of frames) {
      expect(() => Schema.decodeUnknownSync(SyncEvent)(frame)).not.toThrow();
    }

    mock.dispose();
  });
});
