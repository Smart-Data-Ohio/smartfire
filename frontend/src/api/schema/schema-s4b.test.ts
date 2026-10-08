import { describe, expect, it } from "@effect/vitest";
import { Schema } from "effect";
import {
  AgentDeliveryOutcome,
  AgentLedgerEventType,
  AgentLedgerPage,
  AgentWebhookStatus,
} from "./agents.ts";
import { SyncEvent } from "./sync.ts";
import { Thread, ThreadDetail } from "./thread.ts";
import {
  CreateWorkHandoff,
  isSafeWorkHref,
  UpdateWork,
  WorkDetail,
  WorkFacts,
  WorkFilter,
  WorkHistoryKind,
  WorkLinkKind,
  WorkList,
  WorkPullRequestState,
  WorkStatusRead,
} from "./work.ts";

// Mirrors the wire JSON in crates/api_types/src/tests_s4b.rs.

const userJson = {
  id: 7,
  name: "Ada Lovelace",
  role: "administrator",
  status: "active",
  bio: null,
  avatarUrl: "/users/7/avatar?v=1700000000",
  hasAvatar: true,
  customStatus: null,
  avatarIcon: null,
  agent: null,
  createdAt: "2026-09-26T12:26:46.848Z",
  updatedAt: "2026-09-26T12:26:46.848000Z",
} as const;

/** The agent owner and a thread's step, as in schema-s4.test.ts. */
const agentUserJson = {
  id: 40,
  name: "Scout",
  role: "bot",
  status: "active",
  bio: null,
  avatarUrl: "/users/40/avatar?v=1700000001",
  hasAvatar: true,
  customStatus: null,
  avatarIcon: {
    name: "github",
    title: "GitHub",
    kind: "brand",
    character: null,
    imageUrl: "/assets/icons/github.svg",
  },
  agent: { agentId: 3, kind: "personal", status: "working", suspended: false },
  createdAt: "2026-09-26T12:26:46.848Z",
  updatedAt: "2026-09-26T12:26:46.848000Z",
} as const;

const threadStepJson = {
  id: 11,
  messageId: null,
  threadId: 88,
  name: "Search the docs",
  status: "done",
  inputSummary: "rate limits",
  outputSummary: null,
  durationMs: 1250,
  position: 0,
  createdAt: "2026-10-06T09:15:01.000Z",
  updatedAt: "2026-10-06T09:15:02.250Z",
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
  tags: [],
  messageCount: 4,
  status: "in_progress",
  owner: agentUserJson,
  ownerActive: true,
  runUrl: "https://ci.example.com/runs/7",
  resultUpdatedAt: null,
  updatedAt: "2026-10-06T09:00:00.000000Z",
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
  steps: [threadStepJson],
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
      toOwner: { userId: 7, name: null },
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
  it("keeps work revisions as fixed-width strings and rejects millisecond versions", () => {
    const updatedAt = "2026-10-06T09:00:00.123456Z";

    const decode = (version: string) =>
      Schema.decodeUnknownSync(WorkFacts)({ ...workFactsJson, updatedAt: version });

    expect(decode(updatedAt).updatedAt).toBe(updatedAt);
    expect(() => decode("2026-10-06T09:00:00.123Z")).toThrow();
  });

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

  it("read every webhook status and pull request state, and later ones as unknown", () => {
    for (const status of ["none", "pending", "delivered", "failed"] as const) {
      roundTrips(AgentWebhookStatus, status);
    }

    for (const state of ["open", "draft", "merged", "closed"] as const) {
      roundTrips(WorkPullRequestState, state);
    }

    expect(Schema.decodeUnknownSync(AgentWebhookStatus)("retrying")).toBe("unknown");
    expect(Schema.decodeUnknownSync(WorkPullRequestState)("queued")).toBe("unknown");

    const page = Schema.decodeUnknownSync(AgentLedgerPage)({
      events: [{ ...ledgerEventJson, webhookStatus: "retrying" }],
      users: [],
      nextCursor: null,
    });

    expect(page.events[0]?.webhookStatus).toBe("unknown");
  });

  it("read a work status added later as unknown on a thread, its list and its events", () => {
    const later = { ...threadJson, work: { ...workFactsJson, status: "in_review" } };

    expect(Schema.decodeUnknownSync(WorkStatusRead)("in_review")).toBe("unknown");
    expect(Schema.decodeUnknownSync(Thread)(later).work?.status).toBe("unknown");

    const list = Schema.decodeUnknownSync(WorkList)({
      threads: [{ thread: later, roomName: "general", board: false, updatedAt: later.createdAt }],
      users: [],
    });

    expect(list.threads[0]?.thread.work?.status).toBe("unknown");

    const event = Schema.decodeUnknownSync(SyncEvent)({
      seq: 4,
      topic: "room:12",
      type: "thread.updated",
      data: later,
    });

    expect(event.type === "thread.updated" && event.data.work?.status).toBe("unknown");

    const detail = Schema.decodeUnknownSync(WorkDetail)({
      ...workDetailJson,
      history: [{ ...workDetailJson.history[1], toStatus: "in_review" }],
    });

    expect(detail.history[0]?.toStatus).toBe("unknown");
    // Writes stay strict: only the four statuses go out.
    expect(() => Schema.decodeUnknownSync(UpdateWork)({ status: "in_review" })).toThrow();
  });

  it("keep only https run URLs and links safe in an href", () => {
    const facts = Schema.decodeUnknownSync(WorkFacts)({
      ...workFactsJson,
      runUrl: "javascript:alert(1)",
      links: [
        workFactsJson.links[0],
        { ...workFactsJson.links[0], id: 32, kind: "drive_file", url: "javascript:alert(1)" },
        { ...workFactsJson.links[0], id: 33, kind: "event", url: "/rooms/12/events/5" },
        { ...workFactsJson.links[0], id: 34, kind: "drive_file", url: "http://docs.example" },
      ],
    });

    expect(facts.runUrl).toBeNull();
    expect(facts.links.map((link) => link.id)).toEqual([31, 33]);
    expect(Schema.decodeUnknownSync(WorkFacts)(workFactsJson).runUrl).toBe(
      "https://ci.example.com/runs/7",
    );

    for (const url of ["https://github.com/x", "HTTPS://drive.google.com/y", "/rooms/1/events/2"]) {
      expect(isSafeWorkHref(url), url).toBe(true);
    }

    for (const url of [
      "javascript:alert(1)",
      "//evil.example",
      "/\\evil.example",
      "http://a.b",
      "data:x",
    ]) {
      expect(isSafeWorkHref(url), url).toBe(false);
    }
  });

  it("read an unassigned thread's facts", () => {
    const facts = Schema.decodeUnknownSync(WorkFacts)({ ...workFactsJson, owner: null });

    expect(facts.owner).toBeNull();
    roundTrips(WorkFacts, { ...workFactsJson, owner: null, links: [] });
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
