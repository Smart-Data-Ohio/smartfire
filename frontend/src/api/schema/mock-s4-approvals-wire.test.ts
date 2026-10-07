import { Schema } from "effect";
import { describe, expect, it } from "vitest";
import type { Json, JsonRecord } from "../../../mock/json.ts";
import { AGENT_IDS, HIDDEN_ROOM } from "../../../mock/s4/agents.ts";
import { EMBER_LEDGER_SIZE, UNKNOWN_LEDGER_TYPE } from "../../../mock/s4/ledger.ts";
import { manualScheduler } from "../../../mock/scheduler.ts";
import { createMockServer, type MockServer } from "../../../mock/server.ts";
import { AgentApproval, AgentApprovalPage, AgentLedgerPage } from "./agents.ts";
import { SyncEvent } from "./sync.ts";

/** A quiet mock on a manual clock. */
function server(): MockServer {
  const clock = manualScheduler(Date.UTC(2026, 9, 6, 16, 30, 0));

  return createMockServer({ now: () => clock.now(), seed: 1, scheduler: clock });
}

async function send(mock: MockServer, method: string, path: string, body: JsonRecord = {}) {
  return mock.handle({ method, path, body, headers: { "X-CSRF-Token": mock.csrfToken() } });
}

async function approvalsPage(mock: MockServer, agentId: number, query = "") {
  const response = await send(mock, "GET", `/api/v1/agents/${agentId}/approvals${query}`);

  expect(response.status).toBe(200);

  return Schema.decodeUnknownSync(AgentApprovalPage)(response.json);
}

async function ledgerPage(mock: MockServer, agentId: number, query = "") {
  const response = await send(mock, "GET", `/api/v1/agents/${agentId}/events${query}`);

  expect(response.status).toBe(200);

  return Schema.decodeUnknownSync(AgentLedgerPage)(response.json);
}

/** The raw `eventType`s of a ledger reply, before decoding. */
function rawTypes(json: Json | undefined): string[] {
  const raw = Schema.decodeUnknownSync(
    Schema.Struct({ events: Schema.Array(Schema.Struct({ eventType: Schema.String })) }),
  )(json);

  return raw.events.map((event) => event.eventType);
}

describe("the S4 approvals mock against the pinned wire schemas", () => {
  it("pages an agent's requests newest first, by status", async () => {
    const mock = server();
    const first = await approvalsPage(mock, AGENT_IDS.ember);

    expect(first.approvals).toHaveLength(50);
    expect(first.nextCursor).not.toBeNull();
    expect(first.approvals[0]?.id).toBe(100);
    expect(first.approvals[0]?.adminOnly).toBe(false);

    const second = await approvalsPage(
      mock,
      AGENT_IDS.ember,
      `?before=${encodeURIComponent(first.nextCursor ?? "")}`,
    );

    const ids = [...first.approvals, ...second.approvals].map((approval) => approval.id);

    expect(new Set(ids).size).toBe(ids.length);
    expect(ids).toEqual([...ids].sort((left, right) => right - left));
    expect(second.nextCursor).toBeNull();

    const pending = await approvalsPage(mock, AGENT_IDS.ember, "?status=pending");

    expect(pending.approvals.map((approval) => approval.status)).toEqual(
      pending.approvals.map(() => "pending"),
    );

    expect(pending.approvals.map((approval) => approval.id)).toEqual([100, 99, 93]);

    const bad = await send(mock, "GET", `/api/v1/agents/${AGENT_IDS.ember}/approvals?before=%%%`);

    expect(bad.status).toBe(422);
    mock.dispose();
  });

  it("decides a request, publishing approval.updated and the item's new status", async () => {
    const mock = server();
    const frames: unknown[] = [];

    const connection = mock.connect((frame) => {
      if (frame.t === "batch") frames.push(...frame.events);
    });

    connection.receive({ t: "hello", v: 1, resume: null, topics: [] });

    const approved = await send(mock, "PATCH", "/api/v1/agent_approvals/100", {
      decision: "approved",
      note: "Ship it",
    });

    expect(approved.status).toBe(200);

    const decided = Schema.decodeUnknownSync(AgentApproval)(approved.json);

    expect(decided).toMatchObject({ status: "approved", decidedById: 1, decisionNote: "Ship it" });

    const again = await send(mock, "PATCH", "/api/v1/agent_approvals/100", { decision: "denied" });

    expect(again.status).toBe(422);

    const events = frames.map((frame) => Schema.decodeUnknownSync(SyncEvent)(frame));

    expect(events.map((event) => event.type)).toEqual(
      expect.arrayContaining(["approval.updated", "activity.item"]),
    );

    mock.dispose();
  });

  it("gates by role: admin-only approvals, hidden rooms and other people's agents", async () => {
    const mock = server();

    expect((await send(mock, "POST", "/__mock/viewer-role", { role: "member" })).status).toBe(200);

    const page = await approvalsPage(mock, AGENT_IDS.ember, "?status=pending");
    const merge = page.approvals.find((approval) => approval.id === 99);
    const hidden = page.approvals.find((approval) => approval.roomId === HIDDEN_ROOM.id);

    expect(merge).toMatchObject({ adminOnly: true, approvable: false, deniable: true });
    expect(merge?.githubLogin).not.toBeNull();
    expect(hidden?.roomName).toBeNull();

    const refused = await send(mock, "PATCH", "/api/v1/agent_approvals/99", {
      decision: "approved",
    });

    expect(refused.status).toBe(403);

    const denied = await send(mock, "PATCH", "/api/v1/agent_approvals/99", { decision: "denied" });

    expect(denied.status).toBe(200);

    const others = await send(mock, "GET", `/api/v1/agents/${AGENT_IDS.scout}/approvals`);

    expect(others.status).toBe(404);
    mock.dispose();
  });

  it("publishes approval.updated when someone else settles a request", async () => {
    const mock = server();
    const frames: unknown[] = [];

    const connection = mock.connect((frame) => {
      if (frame.t === "batch") frames.push(...frame.events);
    });

    connection.receive({ t: "hello", v: 1, resume: null, topics: [] });

    const asked = await send(mock, "POST", "/__mock/approval-request", { summary: "Merge #400" });

    expect(asked.status).toBe(201);

    const { id } = Schema.decodeUnknownSync(AgentApproval)(asked.json);
    const settled = await send(mock, "POST", "/__mock/approval-settle", { id, status: "denied" });

    expect(settled.status).toBe(200);

    const updates = frames.flatMap((frame) => {
      const event = Schema.decodeUnknownSync(SyncEvent)(frame);

      return event.type === "approval.updated" ? [event] : [];
    });

    expect(updates).toHaveLength(1);
    mock.dispose();
  });
});

describe("the S4 ledger mock against the pinned wire schemas", () => {
  it("pages every entry once, newest first, ties included", async () => {
    const mock = server();
    const first = await ledgerPage(mock, AGENT_IDS.ember);

    expect(first.events).toHaveLength(50);
    expect(first.events.some((event) => event.eventType === "unknown")).toBe(true);

    const raw = await send(mock, "GET", `/api/v1/agents/${AGENT_IDS.ember}/events`);

    expect(rawTypes(raw.json)).toContain(UNKNOWN_LEDGER_TYPE);

    const second = await ledgerPage(
      mock,
      AGENT_IDS.ember,
      `?before=${encodeURIComponent(first.nextCursor ?? "")}`,
    );

    const ids = [...first.events, ...second.events].map((event) => event.id);

    expect(new Set(ids).size).toBe(EMBER_LEDGER_SIZE);
    expect(second.nextCursor).toBeNull();

    const suppressed = await ledgerPage(mock, AGENT_IDS.ember, "?outcome=suppressed");

    expect(suppressed.events.length).toBeGreaterThan(0);
    expect(suppressed.events.every((event) => event.outcome === "suppressed")).toBe(true);

    const withContent = first.events.filter((event) => event.content !== null);

    expect(withContent.length).toBeGreaterThan(0);
    expect(withContent.every((event) => [...(event.content ?? "")].length <= 140)).toBe(true);
    mock.dispose();
  });

  it("gates rooms the viewer isn't in, and refuses other people's agents", async () => {
    const mock = server();
    const asAdmin = await ledgerPage(mock, AGENT_IDS.ember);
    const named = asAdmin.events.find((event) => event.roomId === HIDDEN_ROOM.id);

    expect(named?.roomName).toBe(HIDDEN_ROOM.name);
    expect(named?.detail).not.toBeNull();

    await send(mock, "POST", "/__mock/viewer-role", { role: "member" });

    const asOwner = await ledgerPage(mock, AGENT_IDS.ember);
    const gated = asOwner.events.find((event) => event.roomId === HIDDEN_ROOM.id);

    expect(gated).toMatchObject({ roomName: null, detail: null, content: null });

    const roomless = asOwner.events.find((event) => event.roomId === null);

    expect(roomless?.detail).not.toBeNull();
    expect((await send(mock, "GET", `/api/v1/agents/${AGENT_IDS.scout}/events`)).status).toBe(403);
    expect((await send(mock, "GET", "/api/v1/agents/999/events")).status).toBe(404);
    mock.dispose();
  });
});
