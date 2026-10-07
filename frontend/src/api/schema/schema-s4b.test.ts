import { describe, expect, it } from "@effect/vitest";
import { Schema } from "effect";
import { AgentDeliveryOutcome, AgentLedgerEventType, AgentLedgerPage } from "./agents.ts";
import { ThreadDetail } from "./thread.ts";
import {
  CreateWorkHandoff,
  UpdateWork,
  WorkDetail,
  WorkFilter,
  WorkHistoryKind,
  WorkLinkKind,
  WorkList,
} from "./work.ts";

// Mirrors the wire JSON in crates/api_types/src/tests_s4b.rs.

const userJson = {
  id: 7,
  name: "Ada Lovelace",
  role: "administrator",
  status: "active",
  bio: null,
  avatarUrl: "/users/7/avatar?v=1700000000",
  customStatus: null,
  avatarIcon: null,
  agent: null,
  createdAt: "2026-09-26T12:26:46.848Z",
} as const;

const ledgerEventJson = {
  id: 501,
  eventType: "github_action_completed",
  outcome: "delivered",
  createdAt: "2026-10-06T09:30:00.000Z",
  roomId: 12,
  roomName: "general",
  actorId: 7,
  messageId: 9001,
  hop: 1,
  detail: null,
  webhookStatus: "failed",
  webhookAttempts: 2,
  webhookLastError: "HTTP 500",
  external: { action: "merge_pull_request", status: "succeeded", message: null },
  handoffSummary: null,
  content: "Merging #42 now",
} as const;

const workFactsJson = {
  status: "in_progress",
  ownerId: 40,
  ownerActive: true,
  runUrl: "https://ci.example.com/runs/7",
  resultUpdatedAt: null,
  links: [
    {
      id: 31,
      kind: "pull_request",
      label: "basecamp/once-campfire#42",
      url: "https://github.com/basecamp/once-campfire/pull/42",
      pullRequestState: "merged",
      title: "Speed up search",
      eventStartsAt: null,
      eventTimeZone: null,
      eventCancelled: false,
    },
  ],
} as const;

const threadJson = {
  id: 88,
  roomId: 12,
  parentMessageId: 9001,
  creatorId: 7,
  name: "Speed up search",
  status: "active",
  replyCount: 3,
  lastActivityAt: "2026-10-06T10:00:00.000Z",
  autoArchiveAfterMinutes: 4320,
  createdAt: "2026-10-06T09:20:00.000Z",
  work: workFactsJson,
} as const;

const workDetailJson = {
  resultMarkdown: "Shipped in **v2.1**",
  resultHtml: "<p>Shipped in <strong>v2.1</strong></p>",
  resultUpdatedById: 7,
  steps: [],
  history: [
    {
      id: 71,
      kind: "handoff",
      createdAt: "2026-10-06T09:40:00.000Z",
      actorId: 7,
      fromStatus: "in_progress",
      toStatus: "in_progress",
      fromOwner: { userId: 7, name: "Ada Lovelace" },
      toOwner: { userId: 40, name: "Scout" },
      note: null,
      handoff: { summary: "Finish the index rebuild", linkCount: 1, questionCount: 0 },
    },
    {
      id: 70,
      kind: "update",
      createdAt: "2026-10-06T09:20:00.000Z",
      actorId: null,
      fromStatus: null,
      toStatus: "planned",
      fromOwner: null,
      toOwner: null,
      note: "Picked up from triage",
      handoff: null,
    },
  ],
  ownerCandidates: [
    { userId: 7, provider: null, description: null },
    { userId: 40, provider: "Anthropic", description: "Triage and fixes" },
  ],
  handoffReceivers: [{ agentId: 3, userId: 40 }],
} as const;

/** Decoding then encoding gives back exactly the wire JSON. */
const roundTrips = <S extends Schema.Codec<unknown, unknown>>(schema: S, wire: S["Encoded"]) =>
  expect(Schema.encodeSync(schema)(Schema.decodeUnknownSync(schema)(wire))).toEqual(wire);

describe("S4 contract B schemas", () => {
  it("round-trip the ledger", () => {
    roundTrips(AgentLedgerPage, {
      events: [
        ledgerEventJson,
        {
          ...ledgerEventJson,
          id: 502,
          eventType: "posted",
          outcome: null,
          webhookStatus: "none",
          external: null,
          content: null,
        },
      ],
      users: [userJson],
      nextCursor: "NTAx",
    });
  });

  it("read a ledger type or outcome added later as unknown", () => {
    expect(Schema.decodeUnknownSync(AgentLedgerEventType)("reaction")).toBe("unknown");
    expect(Schema.decodeUnknownSync(AgentDeliveryOutcome)("expired")).toBe("unknown");

    const page = Schema.decodeUnknownSync(AgentLedgerPage)({
      events: [ledgerEventJson, { ...ledgerEventJson, id: 503, eventType: "reaction" }],
      users: [],
      nextCursor: null,
    });

    expect(page.events.map((event) => event.eventType)).toEqual([
      "github_action_completed",
      "unknown",
    ]);
  });

  it("round-trip a tracked thread's detail and the work list", () => {
    const permissions = {
      canRename: true,
      canClose: true,
      canReopen: false,
      canLock: true,
      canUnlock: false,
      canDelete: true,
      canConvertWork: false,
      canManageWork: true,
      canUpdateWorkStatus: true,
      canAssignWork: true,
      canRemoveWork: true,
    } as const;

    roundTrips(WorkDetail, workDetailJson);
    roundTrips(ThreadDetail, {
      thread: threadJson,
      membership: null,
      parentMessage: null,
      permissions,
      work: workDetailJson,
      users: [userJson],
    });
    roundTrips(WorkList, {
      threads: [
        {
          thread: threadJson,
          roomName: "general",
          board: false,
          updatedAt: "2026-10-06T10:00:00.000Z",
        },
      ],
      users: [userJson],
    });
  });

  it("read a link or history kind added later as unknown", () => {
    expect(Schema.decodeUnknownSync(WorkLinkKind)("figma_file")).toBe("unknown");
    expect(Schema.decodeUnknownSync(WorkHistoryKind)("tag")).toBe("unknown");

    const detail = Schema.decodeUnknownSync(WorkDetail)({
      ...workDetailJson,
      history: [{ ...workDetailJson.history[1], kind: "tag" }],
    });

    expect(detail.history.map((entry) => entry.kind)).toEqual(["unknown"]);
  });

  it("round-trip the work filter and requests", () => {
    for (const filter of ["open", "done", "all", "agents", "boards"] as const) {
      roundTrips(WorkFilter, filter);
    }

    roundTrips(UpdateWork, {});
    roundTrips(UpdateWork, { status: null, ownerId: null });
    roundTrips(UpdateWork, { status: "blocked", ownerId: 40, resultMarkdown: "Waiting on review" });
    expect(() => Schema.decodeUnknownSync(UpdateWork)({ status: "started" })).toThrow();
    roundTrips(CreateWorkHandoff, {
      receiverAgentId: 3,
      summary: "Finish the index rebuild",
      links: ["https://github.com/basecamp/once-campfire/pull/42"],
      openQuestions: [],
    });
  });
});
