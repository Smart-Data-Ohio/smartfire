import { describe, expect, it } from "vitest";
import type { AgentApproval } from "../../src/gen/AgentApproval.ts";
import type { AgentApprovalPage } from "../../src/gen/AgentApprovalPage.ts";
import type { AgentDirectory } from "../../src/gen/AgentDirectory.ts";
import type { AgentProfile } from "../../src/gen/AgentProfile.ts";
import type { AgentStatusChanged } from "../../src/gen/AgentStatusChanged.ts";
import type { ThreadDetail } from "../../src/gen/ThreadDetail.ts";
import type { WorkList } from "../../src/gen/WorkList.ts";
import { collect, expectStatus, get, harness, NOW } from "../s2/testing.ts";
import { SEED_IDS } from "../server.ts";
import { AGENT_IDS } from "./agents.ts";

const DAY = 24 * 60 * 60_000;

const { users } = SEED_IDS;

describe("S4 mock server revisions", () => {
  it("publishes strictly increasing work revisions for status, owner, result and handoff", async () => {
    const { server } = harness();
    const threadId = SEED_IDS.s4.work.done;
    const events = collect(server, [`thread:${threadId}`]);
    const path = `/api/v1/threads/${threadId}`;
    const before = await get<ThreadDetail>(server, path);
    const revisions = [before.thread.work?.updatedAt];

    expect(revisions[0]).toMatch(/^2026-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3}Z$/);

    for (const change of [
      { status: "planned" },
      { ownerId: users.jonah },
      { resultMarkdown: "Reviewed the release" },
    ]) {
      const reply = await expectStatus<ThreadDetail>(server, "PATCH", `${path}/work`, change, 200);

      revisions.push(reply.thread.work?.updatedAt);
    }

    const handoff = await expectStatus<ThreadDetail>(
      server,
      "POST",
      `${path}/work/handoff`,
      { receiverAgentId: AGENT_IDS.ember, summary: "Ship it", links: [], openQuestions: [] },
      201,
    );

    revisions.push(handoff.thread.work?.updatedAt);
    expect(revisions.slice(1)).toEqual([
      "2026-10-06T16:30:00.000Z",
      "2026-10-06T16:30:00.001Z",
      "2026-10-06T16:30:00.002Z",
      "2026-10-06T16:30:00.003Z",
    ]);

    const unchanged = await expectStatus<ThreadDetail>(
      server,
      "PATCH",
      `${path}/work`,
      { status: "planned" },
      200,
    );

    expect(unchanged.thread.work?.updatedAt).toBe(revisions.at(-1));
    expect(
      events.flatMap((event) =>
        event.type === "thread.updated" ? [event.data.work?.updatedAt] : [],
      ),
    ).toEqual(revisions.slice(1));

    const list = await get<WorkList>(server, "/api/v1/work");

    expect(list.threads.find((row) => row.thread.id === threadId)?.thread.work?.updatedAt).toBe(
      revisions.at(-1),
    );
  });

  it("advances an approval revision on a decision, cancellation and return to pending in one millisecond", async () => {
    const { server } = harness();
    const events = collect(server);

    const requested = await expectStatus<AgentApproval>(
      server,
      "POST",
      "/__mock/approval-request",
      { summary: "Publish the summary" },
      201,
    );

    expect(requested.updatedAt).toBe("2026-10-06T16:30:00.000Z");

    const approved = await expectStatus<AgentApproval>(
      server,
      "PATCH",
      `/api/v1/agent_approvals/${requested.id}`,
      { decision: "approved", note: "Ready" },
      200,
    );

    expect(approved.updatedAt).toBe("2026-10-06T16:30:00.001Z");

    const revisions = [approved.updatedAt];

    for (const status of ["cancelled", "pending", "expired"]) {
      const settled = await expectStatus<AgentApproval>(
        server,
        "POST",
        "/__mock/approval-settle",
        { id: requested.id, status },
        200,
      );

      revisions.push(settled.updatedAt);
    }

    expect(revisions).toEqual([
      "2026-10-06T16:30:00.001Z",
      "2026-10-06T16:30:00.002Z",
      "2026-10-06T16:30:00.003Z",
      "2026-10-06T16:30:00.004Z",
    ]);
    expect(
      events.flatMap((event) =>
        event.type === "approval.updated" ? [event.data.approval.updatedAt] : [],
      ),
    ).toEqual(revisions);
  });

  it("advances overdue approval revisions when listing or deciding settles expiration", async () => {
    const { server, clock } = harness();
    const events = collect(server);
    const requested: AgentApproval[] = [];

    for (const summary of ["Expires on read", "Expires on decision"]) {
      requested.push(
        await expectStatus<AgentApproval>(
          server,
          "POST",
          "/__mock/approval-request",
          { summary },
          201,
        ),
      );
    }

    const [listed, decided] = requested;

    if (listed === undefined || decided === undefined) throw new Error("Missing requests");

    clock.advance(2 * DAY);

    await expectStatus(
      server,
      "PATCH",
      `/api/v1/agent_approvals/${decided.id}`,
      {
        decision: "approved",
      },
      422,
    );

    const page = await get<AgentApprovalPage>(
      server,
      `/api/v1/agents/${AGENT_IDS.ember}/approvals`,
    );

    const expired = page.approvals.find((approval) => approval.id === listed.id);

    expect(expired).toMatchObject({ status: "expired", updatedAt: "2026-10-08T16:30:00.000Z" });
    expect(
      events.flatMap((event) =>
        event.type === "approval.updated" &&
        requested.some((item) => item.id === event.data.approval.id)
          ? [event.data.approval.updatedAt]
          : [],
      ),
    ).toEqual(["2026-10-08T16:30:00.000Z", "2026-10-08T16:30:00.000Z"]);

    const again = await get<AgentApprovalPage>(
      server,
      `/api/v1/agents/${AGENT_IDS.ember}/approvals`,
    );

    expect(again.approvals.find((approval) => approval.id === listed.id)?.updatedAt).toBe(
      expired?.updatedAt,
    );
  });

  it("shares one increasing status revision across events, directory and profiles", async () => {
    const { server } = harness();
    const directory = await get<AgentDirectory>(server, "/api/v1/agents");
    const before = directory.agents.find((agent) => agent.agentId === AGENT_IDS.ember);
    const revisions: string[] = [];

    expect(before?.updatedAt).toMatch(/^2026-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3}Z$/);

    for (const change of [
      { status: "working" },
      { statusNote: "Reading the room" },
      { suspended: true },
      { presence: "Summarising" },
      { presence: "Writing" },
      { presence: "" },
      { status: "idle", suspended: false, statusNote: null },
    ]) {
      const event = await expectStatus<AgentStatusChanged>(
        server,
        "POST",
        "/__mock/agent-status",
        { agentId: AGENT_IDS.ember, ...change },
        200,
      );

      revisions.push(event.updatedAt);

      const profile = await get<AgentProfile>(server, `/api/v1/agents/${AGENT_IDS.ember}`);
      const current = await get<AgentDirectory>(server, "/api/v1/agents");

      expect(profile.agent.updatedAt).toBe(event.updatedAt);
      expect(current.agents.find((agent) => agent.agentId === AGENT_IDS.ember)?.updatedAt).toBe(
        event.updatedAt,
      );
    }

    expect(revisions).toEqual(
      Array.from({ length: 7 }, (_, index) => new Date(NOW + index).toISOString()),
    );

    const unchanged = await expectStatus<AgentStatusChanged>(
      server,
      "POST",
      "/__mock/agent-status",
      { agentId: AGENT_IDS.ember, status: "idle" },
      200,
    );

    expect(unchanged.updatedAt).toBe(revisions.at(-1));
  });
});
