import { describe, expect, it } from "@effect/vitest";
import { Schema } from "effect";
import {
  SaveSlackCredentials,
  SlackConnectionState,
  SlackPersonal,
  SlackPlan,
  SlackRunChange,
  SlackRunList,
  SlackRunPage,
  SlackSetup,
  StartPersonalSlackImport,
  StartSlackDryRun,
  StartSlackImport,
} from "./slack.ts";

// The wire JSON below mirrors crates/api_types/src/tests_s7_slack.rs.
const run = {
  id: 12,
  kind: "workspace",
  mode: "dry_run",
  status: "running",
  title: "Workspace dry run #12",
  startedBy: "Grace",
  createdAt: "2026-10-06T12:00:00.000Z",
  startedAt: "2026-10-06T12:00:05.000Z",
  finishedAt: null,
  phase: "conversations",
  current: "#general",
  queuedBehind: false,
  people: { total: 9, matched: 6, placeholders: 2, deactivated: 1, bots: 0 },
  counts: null,
  apiCalls: 41,
  issuesCount: 2,
  error: null,
  active: true,
  cancellable: true,
  undoable: false,
  undoBlockedReason: null,
  planReady: false,
  catchUp: false,
  conversations: [],
} as const;

const conversation = {
  id: "C111",
  name: "general",
  kind: "Public channel",
  archived: false,
  members: 9,
  messages: 120,
  threads: 7,
};

const row = {
  id: 12,
  kind: "personal",
  mode: "import",
  status: "undone",
  startedBy: "Grace",
  createdAt: "2026-10-06T12:00:00.000Z",
} as const;

const roundTrips = <S extends Schema.Codec<unknown, unknown>>(schema: S, wire: S["Encoded"]) =>
  expect(Schema.encodeSync(schema)(Schema.decodeUnknownSync(schema)(wire))).toEqual(wire);

describe("S7 Slack schemas", () => {
  it("round-trip the setup page and its writes", () => {
    roundTrips(SlackSetup, {
      clientId: "123.456",
      configured: true,
      configuredBy: "Grace",
      teamName: "Acme",
      teamKnown: true,
      connection: { state: "rejected", reason: "token_revoked" },
      activeRun: { id: 12, kind: "workspace", mode: "dry_run", status: "queued" },
      manifest: "display_information: …",
      connectPath: "/slack/oauth/start?return_to=%2Faccount%2Fslack_import",
    });
    roundTrips(SlackConnectionState, { state: "none" });
    roundTrips(SaveSlackCredentials, { clientId: "123.456", clientSecret: null });
  });

  it("round-trip the run lists", () => {
    roundTrips(SlackRunList, { runs: [row] });
    roundTrips(SlackPersonal, {
      teamKnown: true,
      connection: { state: "connected" },
      connectPath: "/slack/oauth/start?return_to=%2Fslack%2Fimports",
      runs: [row],
    });
  });

  it("round-trip a run and its issues", () => {
    roundTrips(SlackRunPage, {
      run: {
        ...run,
        counts: {
          roomsCreated: 2,
          roomsMerged: 1,
          messages: 300,
          replies: 40,
          threads: 8,
          reactions: 12,
          pins: 1,
          filesLinked: 3,
          skipped: 0,
        },
        conversations: [conversation],
      },
      issues: [{ level: "warning", slackRef: "C111", message: "No access" }],
      nextPage: 2,
    });
    roundTrips(SlackRunChange, { run, notice: "Dry run started." });
  });

  it("round-trip the plan and the starts", () => {
    roundTrips(SlackPlan, {
      runId: 12,
      conversations: [{ conversation, target: "4" }],
      rooms: [{ id: 4, name: "General" }],
      samples: [{ conversation: "general", slackText: "*hi*", html: "<p><strong>hi</strong></p>" }],
      defaultOldest: "2026-09-22",
    });
    roundTrips(StartSlackDryRun, { includePrivate: true, oldest: "2026-01-01", latest: null });
    roundTrips(StartSlackImport, {
      conversationIds: ["C111"],
      roomTargets: { C111: "new" },
      preset: "test",
      oldest: null,
      latest: null,
    });
    roundTrips(StartPersonalSlackImport, {
      mode: "import",
      dryRunId: 12,
      conversationIds: ["D111"],
    });
  });

  it("refuse an unknown run status", () => {
    expect(() =>
      Schema.decodeUnknownSync(SlackRunList)({ runs: [{ ...row, status: "paused" }] }),
    ).toThrow();
  });
});
