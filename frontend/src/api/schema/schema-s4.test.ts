import { describe, expect, it } from "@effect/vitest";
import { Schema } from "effect";
import {
  AgentApproval,
  AgentApprovalPage,
  AgentCapability,
  AgentDirectory,
  AgentDirectoryRow,
  AgentKind,
  AgentProfile,
  AgentStatus,
  AgentStatusChanged,
  AgentStepStatus,
  ApprovalDecision,
  DecideApproval,
} from "./agents.ts";
import { MessageDTO } from "./message.ts";
import { ServerFrame } from "./sync.ts";
import { User } from "./user.ts";

// Mirrors the wire JSON in crates/api_types/src/tests_s4.rs.

const directoryRowJson = {
  agentId: 3,
  userId: 40,
  kind: "personal",
  ownerId: 7,
  status: "working",
  statusNote: "Triaging the queue",
  suspended: false,
  createdAt: "2026-09-01T10:00:00.000Z",
  statusChangedAt: "2026-10-06T09:00:00.000Z",
  lastSeenAt: null,
  updatedAt: "2026-10-06T09:00:00.000000Z",
} as const;

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

const stepJson = {
  id: 11,
  messageId: 9001,
  threadId: null,
  name: "Search the docs",
  status: "done",
  inputSummary: "rate limits",
  outputSummary: null,
  durationMs: 1250,
  position: 0,
  createdAt: "2026-10-06T09:15:01.000Z",
  updatedAt: "2026-10-06T09:15:02.250Z",
} as const;

const approvalJson = {
  id: 21,
  agentId: 3,
  agentUserId: 40,
  roomId: 12,
  roomName: "general",
  action: "github.merge_pull_request",
  summary: "Merge #42 into main",
  status: "pending",
  expiresAt: "2026-10-07T09:15:00.000Z",
  createdAt: "2026-10-06T09:15:00.000Z",
  decidedById: null,
  decidedAt: null,
  decisionNote: null,
  githubLogin: "scout-bot",
  fizzyUserName: null,
  adminOnly: true,
  approvable: false,
  deniable: true,
  updatedAt: "2026-10-06T09:00:00.000000Z",
} as const;

const messageJson = {
  id: 9001,
  roomId: 12,
  threadId: null,
  creatorId: 40,
  clientMessageId: "4f1c7a0e-5b0e-4c55-9d0a-6f3b2d1e8c11",
  sound: null,
  bodyHtml: "<p>Done.</p>",
  markdownSource: "Done.",
  systemNote: false,
  action: false,
  streaming: false,
  embedsSuppressed: false,
  replyToMessageId: null,
  forwardedFromMessageId: null,
  forwardedAt: null,
  forwardNote: null,
  editedAt: null,
  attachment: null,
  reactions: [],
  boosts: [],
  pinned: false,
  thread: null,
  poll: null,
  cards: [],
  cardsAsOf: "2026-10-06T09:15:00.200Z",
  steps: [stepJson],
  createdAt: "2026-10-06T09:15:00.123Z",
  updatedAt: "2026-10-06T09:15:00.123Z",
} as const;

/** Decoding then encoding gives back exactly the wire JSON. */
const roundTrips = <S extends Schema.Codec<unknown, unknown>>(schema: S, wire: S["Encoded"]) =>
  expect(Schema.encodeSync(schema)(Schema.decodeUnknownSync(schema)(wire))).toEqual(wire);

const accepts = <S extends Schema.Codec<unknown, unknown>>(
  schema: S,
  values: readonly S["Encoded"][],
) => {
  for (const value of values) roundTrips(schema, value);
};

describe("S4 contract A schemas", () => {
  it("keeps fixed-width revisions as strings on directory rows, approvals and status events", () => {
    const updatedAt = "2026-10-06T09:00:00.123456Z";

    const decoders = [
      (version: string) =>
        Schema.decodeUnknownSync(AgentDirectoryRow)({ ...directoryRowJson, updatedAt: version })
          .updatedAt,
      (version: string) =>
        Schema.decodeUnknownSync(AgentApproval)({ ...approvalJson, updatedAt: version }).updatedAt,
      (version: string) =>
        Schema.decodeUnknownSync(AgentStatusChanged)({
          agentId: 3,
          userId: 40,
          status: "working",
          statusNote: null,
          statusChangedAt: null,
          suspended: false,
          workingPresence: null,
          workingPresenceExpiresAt: null,
          updatedAt: version,
        }).updatedAt,
    ];

    for (const decode of decoders) {
      expect(decode(updatedAt)).toBe(updatedAt);
      expect(() => decode("2026-10-06T09:00:00.123Z")).toThrow();
    }
  });

  it("round-trip the agent enums", () => {
    accepts(AgentStatus, ["idle", "working", "waiting", "failed"]);
    accepts(AgentKind, ["personal", "workspace"]);
    accepts(AgentStepStatus, ["pending", "running", "done", "failed"]);
    accepts(AgentCapability, [
      "read_messages",
      "post_messages",
      "react",
      "manage_threads",
      "external_action",
      "fizzy",
      "dm_anyone",
    ]);
    accepts(ApprovalDecision, ["approved", "denied"]);
    expect(() => Schema.decodeUnknownSync(ApprovalDecision)("maybe")).toThrow();
  });

  it("read an agent status, kind or step status added later as unknown", () => {
    expect(Schema.decodeUnknownSync(AgentStatus)("busy")).toBe("unknown");
    expect(Schema.decodeUnknownSync(AgentKind)("team")).toBe("unknown");
    expect(Schema.decodeUnknownSync(AgentStepStatus)("skipped")).toBe("unknown");
    expect(() => Schema.decodeUnknownSync(AgentStatus)(3)).toThrow();

    // One unknown value doesn't fail the user, the message or the list that carries it.
    const user = Schema.decodeUnknownSync(User)({
      ...agentUserJson,
      agent: { ...agentUserJson.agent, kind: "team", status: "busy" },
    });

    expect(user.agent).toMatchObject({ kind: "unknown", status: "unknown" });

    const message = Schema.decodeUnknownSync(MessageDTO)({
      ...messageJson,
      steps: [{ ...stepJson, status: "skipped" }],
    });

    expect(message.steps.map((step) => step.status)).toEqual(["unknown"]);

    const directory = Schema.decodeUnknownSync(AgentDirectory)({
      agents: [directoryRowJson, { ...directoryRowJson, agentId: 4, status: "sleeping" }],
      users: [agentUserJson],
    });

    expect(directory.agents.map((row) => row.status)).toEqual(["working", "unknown"]);
  });

  it("round-trip an agent user with its badge and icon", () => {
    roundTrips(User, agentUserJson);
    roundTrips(User, { ...agentUserJson, avatarIcon: null, agent: null });
  });

  it("round-trip the directory and profile", () => {
    roundTrips(AgentDirectory, { agents: [directoryRowJson], users: [agentUserJson] });

    const profile = {
      agent: directoryRowJson,
      provider: "Anthropic",
      runtime: null,
      description: "Keeps the queue tidy.",
      rooms: [{ roomId: 12, name: "general" }],
      hiddenRoomCount: 2,
      grants: {
        legacy: false,
        grants: [
          { capability: "post_messages", workspaceWide: false, roomCount: 2 },
          { capability: "read_messages", workspaceWide: true, roomCount: 0 },
        ],
      },
      management: {
        activitySummary: { delivered: 4, acknowledged: 3, posted: 9, suppressed: 0 },
        budgetUsage: [
          { cap: "messages", used: 9, limit: 200 },
          { cap: "board_posts", used: 0, limit: null },
        ],
      },
      users: [],
    } as const;

    roundTrips(AgentProfile, profile);
    roundTrips(AgentProfile, { ...profile, grants: null, management: null });
  });

  it("round-trip a message with steps", () => {
    roundTrips(MessageDTO, messageJson);
  });

  it("round-trip approvals and decisions", () => {
    roundTrips(AgentApprovalPage, {
      approvals: [approvalJson],
      users: [agentUserJson],
      nextCursor: "MjE",
    });
    roundTrips(DecideApproval, { decision: "denied", note: "Not on a Friday" });
    roundTrips(DecideApproval, { decision: "approved" });
    expect(Schema.encodeSync(DecideApproval)({ decision: "approved" })).not.toHaveProperty("note");
  });
});

describe("S4 contract A sync events", () => {
  it("round-trip in a batch", () => {
    const events = [
      {
        seq: 1,
        topic: "user",
        type: "agent.status",
        data: {
          agentId: 3,
          userId: 40,
          status: "working",
          statusNote: null,
          statusChangedAt: "2026-10-06T09:00:00.000Z",
          suspended: false,
          workingPresence: "Reviewing #42",
          updatedAt: "2026-10-06T09:00:00.000000Z",
          workingPresenceExpiresAt: "2026-10-06T09:20:00.000Z",
        },
      },
      {
        seq: 2,
        topic: "room:12",
        type: "agent.steps",
        data: { roomId: 12, messageId: 9001, threadId: null, steps: [stepJson] },
      },
      {
        seq: 3,
        topic: "thread:88",
        type: "agent.steps",
        data: { roomId: 12, messageId: null, threadId: 88, steps: [] },
      },
      {
        seq: 4,
        topic: "user",
        type: "approval.updated",
        data: { approval: approvalJson, users: [agentUserJson] },
      },
    ] as const;

    const wire = { t: "batch", events } as const;
    const frame = Schema.decodeUnknownSync(ServerFrame)(wire);

    expect(frame.t === "batch" && frame.events.map((event) => event.type)).toEqual(
      events.map((event) => event.type),
    );
    expect(Schema.encodeSync(ServerFrame)(frame)).toEqual(wire);
  });
});
