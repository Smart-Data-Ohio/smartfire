import { describe, expect, it } from "vitest";
import { userFixture } from "../api/testing.ts";
import type { ActivityItem } from "../gen/ActivityItem.ts";
import type { AgentApproval } from "../gen/AgentApproval.ts";
import type { AgentApprovalStatus } from "../gen/AgentApprovalStatus.ts";
import type { AgentLedgerEvent } from "../gen/AgentLedgerEvent.ts";
import {
  applyApproval,
  approvalListKey,
  approvalListOf,
  decidedLocally,
  landApprovalPage,
  markApprovalsStale,
  setApprovalListLoading,
} from "./approvals.ts";
import {
  isLedgerForbidden,
  landLedgerPage,
  ledgerListKey,
  ledgerListOf,
  setLedgerListFailed,
  setLedgerListLoading,
} from "./ledger.ts";
import { applyEvents } from "./reducers.ts";
import { initialState, type State } from "./state.ts";

const NOW = Date.UTC(2026, 9, 6, 16, 30, 0);

const AGENT = 9;

function at(minute: number): string {
  return new Date(NOW + minute * 60_000).toISOString();
}

function approval(id: number, status: AgentApprovalStatus = "pending"): AgentApproval {
  return {
    id,
    agentId: AGENT,
    agentUserId: AGENT,
    roomId: 3,
    roomName: "engineering",
    action: "messages.post",
    summary: `Request ${id}`,
    status,
    expiresAt: at(60 * 48),
    createdAt: at(-id),
    decidedById: null,
    decidedAt: null,
    decisionNote: null,
    githubLogin: null,
    fizzyUserName: null,
    adminOnly: false,
    approvable: true,
    deniable: true,
  };
}

/** Both of the agent's lists loaded: every request (3, 2, 1) and the pending ones (3, 2). */
function loaded(): State {
  const all = approvalListKey(AGENT, "all");
  const pending = approvalListKey(AGENT, "pending");

  let state = setApprovalListLoading(
    setApprovalListLoading(initialState, all, false),
    pending,
    false,
  );

  state = landApprovalPage(
    state,
    all,
    { approvals: [approval(3), approval(2), approval(1, "approved")], users: [], nextCursor: null },
    "replace",
  );

  return landApprovalPage(
    state,
    pending,
    { approvals: [approval(3), approval(2)], users: [userFixture(AGENT)], nextCursor: null },
    "replace",
  );
}

function ids(state: State, filter: "all" | AgentApprovalStatus): readonly number[] {
  return approvalListOf(state, approvalListKey(AGENT, filter)).ids;
}

function requestItem(approvalStatus: AgentApprovalStatus): ActivityItem {
  return {
    id: 70,
    eventType: "agent_approval_request",
    state: "unread",
    readAt: null,
    handledAt: null,
    createdAt: at(0),
    updatedAt: at(0),
    source: {
      sourceType: "agent_approval",
      sourceId: 4,
      roomId: 3,
      threadId: null,
      messageId: null,
      eventId: null,
      creatorId: AGENT,
      title: "engineering · Ember",
      body: "Request 4",
      occurredAt: at(0),
      approvalStatus,
      budgetCap: null,
      path: `/agents/${AGENT}/approvals`,
    },
  };
}

describe("approval requests in the store", () => {
  it("land a page's requests and users in their list", () => {
    const state = loaded();

    expect(ids(state, "all")).toEqual([3, 2, 1]);
    expect(ids(state, "pending")).toEqual([3, 2]);
    expect(state.users[AGENT]?.name).toBe("User 9");
  });

  it("move a decided request out of Pending and into a loaded Approved list", () => {
    const approvedKey = approvalListKey(AGENT, "approved");
    let state = setApprovalListLoading(loaded(), approvedKey, false);

    state = landApprovalPage(
      state,
      approvedKey,
      { approvals: [approval(1, "approved")], users: [], nextCursor: null },
      "replace",
    );

    const decided = applyApproval(state, decidedLocally(approval(2), "approved", 1, "Ok", NOW));

    expect(ids(decided, "pending")).toEqual([3]);
    expect(ids(decided, "approved")).toEqual([2, 1]);
    expect(ids(decided, "all")).toEqual([3, 2, 1]);
    expect(decided.approvals.items[2]).toMatchObject({
      status: "approved",
      decidedById: 1,
      decisionNote: "Ok",
      approvable: false,
    });

    // Rolled back: pending again, in its place.
    const back = applyApproval(decided, approval(2));

    expect(ids(back, "pending")).toEqual([3, 2]);
    expect(ids(back, "approved")).toEqual([1]);
  });

  it("apply approval.updated with its users", () => {
    const state = applyEvents(
      loaded(),
      [
        {
          seq: 1,
          topic: "user:1",
          type: "approval.updated",
          data: {
            approval: { ...approval(3, "denied"), decidedById: 4, decidedAt: at(1) },
            users: [userFixture(4, "Priya")],
          },
        },
      ],
      NOW,
    );

    expect(ids(state, "pending")).toEqual([2]);
    expect(state.approvals.items[3]?.status).toBe("denied");
    expect(state.users[4]?.name).toBe("Priya");
  });

  it("go stale when a new request's activity item arrives, not for a decided one", () => {
    const event = (approvalStatus: AgentApprovalStatus) =>
      applyEvents(
        loaded(),
        [
          {
            seq: 1,
            topic: "user:1",
            type: "activity.item",
            data: { item: requestItem(approvalStatus), unreadCount: 1 },
          },
        ],
        NOW,
      );

    const key = approvalListKey(AGENT, "pending");

    expect(approvalListOf(event("pending"), key).stale).toBe(true);
    expect(approvalListOf(event("approved"), key).stale).toBe(false);
    expect(approvalListOf(markApprovalsStale(loaded()), approvalListKey(AGENT, "all")).stale).toBe(
      true,
    );
  });

  it("drop a page from a load that restarted", () => {
    const key = approvalListKey(AGENT, "all");
    const first = setApprovalListLoading(initialState, key, false);
    const { generation } = approvalListOf(first, key);
    const restarted = setApprovalListLoading(first, key, false);

    const landed = landApprovalPage(
      restarted,
      key,
      { approvals: [approval(1)], users: [], nextCursor: null },
      "replace",
      generation,
    );

    expect(landed).toBe(restarted);
  });
});

function entry(id: number): AgentLedgerEvent {
  return {
    id,
    eventType: "mention",
    outcome: "delivered",
    createdAt: at(-id),
    roomId: 3,
    roomName: "engineering",
    actorId: 2,
    messageId: 30_000 + id,
    hop: 0,
    detail: null,
    webhookStatus: "delivered",
    webhookAttempts: 1,
    webhookLastError: null,
    external: null,
    handoffSummary: null,
    content: "Hello",
  };
}

describe("the ledger in the store", () => {
  it("lands pages in order, and marks a refused agent forbidden until a page lands", () => {
    const key = ledgerListKey(AGENT, "all");
    let state = setLedgerListLoading(initialState, key, false);

    state = setLedgerListFailed(state, key, AGENT, "Forbidden", true);

    expect(isLedgerForbidden(state, AGENT)).toBe(true);
    expect(ledgerListOf(state, key).status).toBe("error");

    state = setLedgerListLoading(state, key, false);
    state = landLedgerPage(
      state,
      key,
      AGENT,
      { events: [entry(1), entry(2)], users: [userFixture(2)], nextCursor: "next" },
      "replace",
    );
    state = setLedgerListLoading(state, key, true);
    state = landLedgerPage(
      state,
      key,
      AGENT,
      { events: [entry(3)], users: [], nextCursor: null },
      "more",
    );

    expect(isLedgerForbidden(state, AGENT)).toBe(false);
    expect(ledgerListOf(state, key)).toMatchObject({ ids: [1, 2, 3], nextCursor: null });
    expect(state.ledger.items[3]?.content).toBe("Hello");
  });

  it("keeps a plain failure apart from a refusal", () => {
    const key = ledgerListKey(AGENT, "suppressed");
    const state = setLedgerListFailed(initialState, key, AGENT, "Server error", false);

    expect(isLedgerForbidden(state, AGENT)).toBe(false);
    expect(ledgerListOf(state, key).error).toBe("Server error");
  });
});
