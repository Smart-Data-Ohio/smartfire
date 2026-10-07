import { afterEach, describe, expect, it } from "vitest";
import { messageFixture, threadFixture } from "../features/threads/test-fixtures.ts";
import {
  agentFixture,
  factsFixture,
  threadDetailFixture,
  workDetailFixture,
} from "../features/work/test-fixtures.ts";
import type { AgentApproval } from "../gen/AgentApproval.ts";
import type { AgentDirectoryRow } from "../gen/AgentDirectoryRow.ts";
import type { AgentProfile } from "../gen/AgentProfile.ts";
import type { AgentStatusChanged } from "../gen/AgentStatusChanged.ts";
import type { AgentStep } from "../gen/AgentStep.ts";
import type { WorkFacts } from "../gen/WorkFacts.ts";
import { approvalListKey, approvalListOf } from "./approvals.ts";
import { mutations, store } from "./store.ts";
import { workListOf } from "./work.ts";

const THREAD = 7;

const AGENT = 9;

const NOW = Date.UTC(2026, 9, 6, 9, 15);

function revision(offset: number): string {
  return new Date(NOW + offset).toISOString();
}

function facts(offset: number, change: Partial<WorkFacts> = {}): WorkFacts {
  return factsFixture({ updatedAt: revision(offset), ...change });
}

function approval(offset: number, change: Partial<AgentApproval> = {}): AgentApproval {
  return {
    id: 3,
    agentId: AGENT,
    agentUserId: AGENT,
    roomId: 4,
    roomName: "engineering",
    action: "messages.post",
    summary: "Post the result",
    status: "pending",
    expiresAt: revision(3_600_000),
    createdAt: revision(-1000),
    decidedById: null,
    decidedAt: null,
    decisionNote: null,
    githubLogin: null,
    fizzyUserName: null,
    adminOnly: false,
    approvable: true,
    deniable: true,
    updatedAt: revision(offset),
    ...change,
  };
}

function agentRow(offset: number, change: Partial<AgentDirectoryRow> = {}): AgentDirectoryRow {
  return {
    agentId: AGENT,
    userId: AGENT,
    kind: "workspace",
    ownerId: 1,
    status: "idle",
    statusNote: null,
    statusChangedAt: null,
    suspended: false,
    createdAt: revision(-1000),
    lastSeenAt: null,
    updatedAt: revision(offset),
    ...change,
  };
}

function agentStatus(offset: number, change: Partial<AgentStatusChanged> = {}): AgentStatusChanged {
  return {
    agentId: AGENT,
    userId: AGENT,
    status: "idle",
    statusNote: null,
    statusChangedAt: null,
    suspended: false,
    workingPresence: null,
    workingPresenceExpiresAt: null,
    updatedAt: revision(offset),
    ...change,
  };
}

function profile(agent: AgentDirectoryRow): AgentProfile {
  return {
    agent,
    provider: null,
    runtime: null,
    description: null,
    rooms: [],
    hiddenRoomCount: 0,
    grants: null,
    management: null,
    users: [agentFixture()],
  };
}

function step(offset: number, change: Partial<AgentStep> = {}): AgentStep {
  return {
    id: 12,
    messageId: null,
    threadId: THREAD,
    name: "Read the logs",
    status: "running",
    inputSummary: null,
    outputSummary: null,
    durationMs: null,
    position: 1,
    createdAt: revision(-1000),
    updatedAt: revision(offset),
    ...change,
  };
}

function landFacts(incoming: WorkFacts): void {
  mutations.loadThreadDetail(threadDetailFixture(THREAD, incoming, workDetailFixture()));
}

function publishFacts(incoming: WorkFacts): void {
  mutations.applyEvents(
    [
      {
        seq: 1,
        topic: `thread:${THREAD}`,
        type: "thread.updated",
        data: threadFixture(THREAD, { work: incoming }),
      },
    ],
    NOW,
  );
}

function landApproval(incoming: AgentApproval): void {
  mutations.landApprovalPage(
    approvalListKey(AGENT, "all"),
    { approvals: [incoming], users: [], nextCursor: null },
    "replace",
  );
}

function publishApproval(incoming: AgentApproval): void {
  mutations.applyEvents(
    [
      {
        seq: 1,
        topic: "user:1",
        type: "approval.updated",
        data: { approval: incoming, users: [] },
      },
    ],
    NOW,
  );
}

function landAgent(incoming: AgentDirectoryRow): void {
  mutations.landAgentDirectory({ agents: [incoming], users: [agentFixture()] }, 0);
}

function publishAgent(incoming: AgentStatusChanged): void {
  mutations.applyEvents([{ seq: 1, topic: "user:1", type: "agent.status", data: incoming }], NOW);
}

describe("server revisions on S4 records", () => {
  afterEach(() => mutations.reset());

  it("keeps work's equal resync echo through an ABA GET", () => {
    landFacts(facts(0, { status: "planned" }));

    const heldGet = facts(1, { status: "blocked" });

    publishFacts(facts(2, { status: "planned" }));
    landFacts(heldGet);

    expect(store.getState().threads[THREAD]?.work).toMatchObject({
      status: "planned",
      updatedAt: revision(2),
    });
  });

  it("keeps an approval's equal resync echo through an ABA GET", () => {
    landApproval(approval(0));

    const heldGet = approval(1, { summary: "Changed in another session" });

    publishApproval(approval(2));
    landApproval(heldGet);

    expect(store.getState().approvals.items[3]).toMatchObject({
      summary: "Post the result",
      updatedAt: revision(2),
    });
  });

  it("keeps an agent's equal resync echo through an ABA GET", () => {
    landAgent(agentRow(0));
    mutations.landAgentProfile(profile(agentRow(0)), 0);

    const heldGet = agentRow(1, { status: "working" });

    publishAgent(agentStatus(2));
    landAgent(heldGet);

    const state = store.getState();

    expect(state.agents.rows[AGENT]).toMatchObject({ status: "idle", updatedAt: revision(2) });
    expect(state.agents.profiles[AGENT]?.profile?.agent.status).toBe("idle");
    expect(state.agents.statuses[AGENT]?.status).toBe("idle");
    expect(state.users[AGENT]?.agent?.status).toBe("idle");
  });

  it("keeps the newest work read when overlapping reads arrive newest first", () => {
    const earlierGet = facts(1, { status: "blocked" });
    const laterGet = facts(2, { status: "done" });

    landFacts(laterGet);
    landFacts(earlierGet);

    expect(store.getState().threads[THREAD]?.work).toEqual(laterGet);
  });

  it("keeps the newest approval read when overlapping reads arrive newest first", () => {
    const earlierGet = approval(1, { summary: "Earlier" });
    const laterGet = approval(2, { summary: "Later" });

    landApproval(laterGet);
    landApproval(earlierGet);

    expect(store.getState().approvals.items[3]).toEqual(laterGet);
  });

  it("keeps the newest agent read when profile and directory arrive newest first", () => {
    const earlierGet = agentRow(1, { suspended: false });
    const laterGet = agentRow(2, { suspended: true });

    mutations.landAgentProfile(profile(laterGet), 0);
    landAgent(earlierGet);

    const state = store.getState();

    expect(state.agents.rows[AGENT]).toMatchObject({ suspended: true, updatedAt: revision(2) });
    expect(state.agents.profiles[AGENT]?.profile?.agent.suspended).toBe(true);
    expect(state.users[AGENT]?.agent?.suspended).toBe(true);
  });

  it("keeps a work event when a held GET lands with older work and newer unrelated data", () => {
    landFacts(facts(0));
    publishFacts(facts(2, { status: "done" }));

    const heldGet = threadDetailFixture(
      THREAD,
      facts(1, { status: "blocked" }),
      workDetailFixture(),
    );

    mutations.loadThreadDetail({ ...heldGet, thread: { ...heldGet.thread, name: "Renamed" } });

    expect(store.getState().threads[THREAD]).toMatchObject({
      name: "Renamed",
      work: { status: "done", updatedAt: revision(2) },
    });
  });

  it("keeps newer work on the work list and normalized thread when an older page lands", () => {
    publishFacts(facts(2, { status: "done" }));
    mutations.landWorkList(
      "all",
      {
        threads: [
          {
            thread: threadFixture(THREAD, { work: facts(1, { status: "planned" }) }),
            roomName: "general",
            board: false,
            updatedAt: revision(1),
          },
        ],
        users: [],
      },
      0,
    );

    expect(workListOf(store.getState(), "all").rows[0]?.thread.work).toMatchObject({
      status: "done",
      updatedAt: revision(2),
    });
    expect(store.getState().threads[THREAD]?.work?.status).toBe("done");
  });

  it("keeps newer work when an older room thread list lands", () => {
    publishFacts(facts(2, { status: "done" }));
    mutations.loadThreadList(4, "active", {
      threads: [
        {
          thread: threadFixture(THREAD, { work: facts(1, { status: "planned" }) }),
          membership: null,
        },
      ],
      users: [],
    });

    expect(store.getState().threads[THREAD]?.work).toMatchObject({
      status: "done",
      updatedAt: revision(2),
    });
  });

  it("merges ownerActive from an older copy without replacing revisioned work facts", () => {
    landFacts(facts(2, { status: "done", ownerActive: true }));
    landFacts(facts(1, { status: "planned", ownerActive: false }));

    expect(store.getState().threads[THREAD]?.work).toMatchObject({
      status: "done",
      ownerActive: false,
      updatedAt: revision(2),
    });
  });

  it("keeps newer suspension and presence when an older agent event follows", () => {
    landAgent(agentRow(0));
    publishAgent(
      agentStatus(2, {
        suspended: true,
        workingPresence: "Publishing",
        workingPresenceExpiresAt: revision(300_000),
      }),
    );
    publishAgent(agentStatus(1));

    expect(store.getState().agents.rows[AGENT]).toMatchObject({
      suspended: true,
      updatedAt: revision(2),
    });
    expect(store.getState().agents.working[AGENT]?.text).toBe("Publishing");
    expect(store.getState().users[AGENT]?.agent?.suspended).toBe(true);
  });

  it("accepts an incoming work copy on a revision tie to fill its missing fields", () => {
    landFacts(facts(2));
    landFacts(facts(2, { runUrl: "https://example.test/run/4" }));

    expect(store.getState().threads[THREAD]?.work?.runUrl).toBe("https://example.test/run/4");
  });

  it("accepts an incoming approval on a revision tie to fill its missing fields", () => {
    landApproval(approval(2));
    landApproval(approval(2, { githubLogin: "ember" }));

    expect(store.getState().approvals.items[3]?.githubLogin).toBe("ember");
  });

  it("accepts a newer server approval even when its decision display time went backwards", () => {
    landApproval(
      approval(1, { status: "approved", decidedAt: revision(500), decisionNote: "First" }),
    );
    landApproval(
      approval(2, { status: "approved", decidedAt: revision(400), decisionNote: "Revised" }),
    );

    expect(store.getState().approvals.items[3]).toMatchObject({
      decisionNote: "Revised",
      updatedAt: revision(2),
    });
  });

  it("uses an agent's updatedAt even when statusChangedAt did not move", () => {
    landAgent(agentRow(1, { statusChangedAt: revision(500) }));
    publishAgent(agentStatus(2, { suspended: true, statusChangedAt: revision(400) }));

    expect(store.getState().agents.rows[AGENT]).toMatchObject({
      suspended: true,
      updatedAt: revision(2),
    });
  });

  it("merges a step's newer revision independently of older work facts", () => {
    landFacts(facts(2, { status: "done" }));
    mutations.loadThreadDetail(
      threadDetailFixture(
        THREAD,
        facts(1),
        workDetailFixture({
          steps: [step(3, { status: "done", outputSummary: "Found the cause" })],
        }),
      ),
    );

    const state = store.getState();

    expect(state.threads[THREAD]?.work?.status).toBe("done");
    expect(state.work.details[THREAD]?.steps[0]).toMatchObject({
      status: "done",
      outputSummary: "Found the cause",
      updatedAt: revision(3),
    });
  });

  it("keeps a newer step event when a work detail with an older step lands", () => {
    mutations.loadThreadDetail(
      threadDetailFixture(THREAD, facts(0), workDetailFixture({ steps: [step(0)] })),
    );
    mutations.applyEvents(
      [
        {
          seq: 1,
          topic: `thread:${THREAD}`,
          type: "agent.steps",
          data: {
            roomId: 4,
            messageId: null,
            threadId: THREAD,
            steps: [step(2, { status: "done" })],
          },
        },
      ],
      NOW,
    );
    mutations.loadThreadDetail(
      threadDetailFixture(THREAD, facts(1), workDetailFixture({ steps: [step(1)] })),
    );

    expect(store.getState().work.details[THREAD]?.steps[0]).toMatchObject({
      status: "done",
      updatedAt: revision(2),
    });
  });

  it("merges a newer approval from a restarted request while protecting the current list", () => {
    const key = approvalListKey(AGENT, "all");

    mutations.setApprovalListLoading(key, false);
    landApproval(approval(0));
    mutations.setApprovalListLoading(key, false);

    const earlierGeneration = approvalListOf(store.getState(), key).generation;

    mutations.setApprovalListLoading(key, false);
    landApproval(approval(1, { id: 4 }));

    const currentList = approvalListOf(store.getState(), key);

    mutations.landApprovalPage(
      key,
      {
        approvals: [approval(2, { summary: "Updated while the request was held" })],
        users: [],
        nextCursor: null,
      },
      "replace",
      earlierGeneration,
    );

    expect(store.getState().approvals.items[3]).toMatchObject({
      summary: "Updated while the request was held",
      updatedAt: revision(2),
    });
    expect(approvalListOf(store.getState(), key)).toBe(currentList);
    expect(currentList.ids).toEqual([4]);
  });

  it("keeps an agent's confirmed badge when an unrelated page carries an older user copy", () => {
    landAgent(agentRow(0));
    publishAgent(agentStatus(2, { status: "working", suspended: true }));
    mutations.landWorkList(
      "all",
      { threads: [], users: [agentFixture(AGENT, "Refreshed name")] },
      0,
    );

    expect(store.getState().users[AGENT]).toMatchObject({
      name: "Refreshed name",
      agent: { status: "working", suspended: true },
    });
    expect(store.getState().agents.rows[AGENT]).toMatchObject({
      status: "working",
      suspended: true,
      updatedAt: revision(2),
    });
  });

  it("keeps newer work when an idempotent thread creation reply arrives late", () => {
    publishFacts(facts(2, { status: "done" }));
    mutations.threadCreated({
      detail: threadDetailFixture(THREAD, facts(0, { status: "planned" }), workDetailFixture()),
      message: messageFixture(100, { threadId: THREAD }),
    });

    expect(store.getState().threads[THREAD]?.work).toMatchObject({
      status: "done",
      updatedAt: revision(2),
    });
  });
});
