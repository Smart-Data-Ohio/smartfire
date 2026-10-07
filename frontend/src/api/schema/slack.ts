import { Schema } from "effect";
import type { SaveSlackCredentials as GeneratedSaveSlackCredentials } from "../../gen/SaveSlackCredentials.ts";
import type { SlackConnectionState as GeneratedSlackConnectionState } from "../../gen/SlackConnectionState.ts";
import type { SlackConversation as GeneratedSlackConversation } from "../../gen/SlackConversation.ts";
import type { SlackCounts as GeneratedSlackCounts } from "../../gen/SlackCounts.ts";
import type { SlackDisconnected as GeneratedSlackDisconnected } from "../../gen/SlackDisconnected.ts";
import type { SlackIssue as GeneratedSlackIssue } from "../../gen/SlackIssue.ts";
import type { SlackPeople as GeneratedSlackPeople } from "../../gen/SlackPeople.ts";
import type { SlackPersonal as GeneratedSlackPersonal } from "../../gen/SlackPersonal.ts";
import type { SlackPlan as GeneratedSlackPlan } from "../../gen/SlackPlan.ts";
import type { SlackPlanConversation as GeneratedSlackPlanConversation } from "../../gen/SlackPlanConversation.ts";
import type { SlackPreset as GeneratedSlackPreset } from "../../gen/SlackPreset.ts";
import type { SlackRoomTarget as GeneratedSlackRoomTarget } from "../../gen/SlackRoomTarget.ts";
import type { SlackRun as GeneratedSlackRun } from "../../gen/SlackRun.ts";
import type { SlackRunChange as GeneratedSlackRunChange } from "../../gen/SlackRunChange.ts";
import type { SlackRunKind as GeneratedSlackRunKind } from "../../gen/SlackRunKind.ts";
import type { SlackRunList as GeneratedSlackRunList } from "../../gen/SlackRunList.ts";
import type { SlackRunMode as GeneratedSlackRunMode } from "../../gen/SlackRunMode.ts";
import type { SlackRunPage as GeneratedSlackRunPage } from "../../gen/SlackRunPage.ts";
import type { SlackRunRow as GeneratedSlackRunRow } from "../../gen/SlackRunRow.ts";
import type { SlackRunStatus as GeneratedSlackRunStatus } from "../../gen/SlackRunStatus.ts";
import type { SlackRunSummary as GeneratedSlackRunSummary } from "../../gen/SlackRunSummary.ts";
import type { SlackSample as GeneratedSlackSample } from "../../gen/SlackSample.ts";
import type { SlackSetup as GeneratedSlackSetup } from "../../gen/SlackSetup.ts";
import type { SlackSetupChange as GeneratedSlackSetupChange } from "../../gen/SlackSetupChange.ts";
import type { StartPersonalSlackImport as GeneratedStartPersonalSlackImport } from "../../gen/StartPersonalSlackImport.ts";
import type { StartSlackDryRun as GeneratedStartSlackDryRun } from "../../gen/StartSlackDryRun.ts";
import type { StartSlackImport as GeneratedStartSlackImport } from "../../gen/StartSlackImport.ts";
import { RoomId, SlackRunId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";

/** A Slack account's connection: none, working, or rejected by Slack (with its reason). */
export const SlackConnectionState = Schema.Union([
  Schema.Struct({ state: Schema.Literal("none") }),
  Schema.Struct({ state: Schema.Literal("connected") }),
  Schema.Struct({ state: Schema.Literal("rejected"), reason: Schema.NullOr(Schema.String) }),
]);

export type SlackConnectionStatePin = Assert<
  Pinned<typeof SlackConnectionState, GeneratedSlackConnectionState>
>;

/** A whole-workspace run or a person's own. */
export const SlackRunKind = Schema.Literals(["workspace", "personal"]);

export type SlackRunKindPin = Assert<Pinned<typeof SlackRunKind, GeneratedSlackRunKind>>;

/** A dry run (a preview, for a person) reads Slack only; an import writes. */
export const SlackRunMode = Schema.Literals(["dry_run", "import"]);

export type SlackRunModePin = Assert<Pinned<typeof SlackRunMode, GeneratedSlackRunMode>>;

/** Where a run stands. */
export const SlackRunStatus = Schema.Literals([
  "queued",
  "running",
  "undoing",
  "completed",
  "failed",
  "cancelled",
  "undone",
]);

export type SlackRunStatusPin = Assert<Pinned<typeof SlackRunStatus, GeneratedSlackRunStatus>>;

/** A test import (recent days, for a look) or the full history. */
export const SlackPreset = Schema.Literals(["test", "full"]);

export type SlackPresetPin = Assert<Pinned<typeof SlackPreset, GeneratedSlackPreset>>;

/** The active run, as the setup page names it. */
export const SlackRunSummary = Schema.Struct({
  id: SlackRunId,
  kind: SlackRunKind,
  mode: SlackRunMode,
  status: SlackRunStatus,
});

export type SlackRunSummaryPin = Assert<Pinned<typeof SlackRunSummary, GeneratedSlackRunSummary>>;

/** `GET /admin/slack`: the setup page. */
export const SlackSetup = Schema.Struct({
  clientId: Schema.NullOr(Schema.String),
  configured: Schema.Boolean,
  configuredBy: Schema.NullOr(Schema.String),
  teamName: Schema.NullOr(Schema.String),
  teamKnown: Schema.Boolean,
  connection: SlackConnectionState,
  activeRun: Schema.NullOr(SlackRunSummary),
  manifest: Schema.String,
  connectPath: Schema.String,
});

export type SlackSetupPin = Assert<Pinned<typeof SlackSetup, GeneratedSlackSetup>>;

/** `PUT /admin/slack`: a blank or `null` secret keeps the saved one. */
export const SaveSlackCredentials = Schema.Struct({
  clientId: Schema.String,
  clientSecret: Schema.NullOr(Schema.String),
});

export type SaveSlackCredentialsPin = Assert<
  Pinned<typeof SaveSlackCredentials, GeneratedSaveSlackCredentials>
>;

/** A setup write's answer: the page afterwards and the classic notice. */
export const SlackSetupChange = Schema.Struct({ setup: SlackSetup, notice: Schema.String });

export type SlackSetupChangePin = Assert<
  Pinned<typeof SlackSetupChange, GeneratedSlackSetupChange>
>;

/** `DELETE /slack/connection`. */
export const SlackDisconnected = Schema.Struct({ notice: Schema.String });

export type SlackDisconnectedPin = Assert<
  Pinned<typeof SlackDisconnected, GeneratedSlackDisconnected>
>;

/** One run on a list. */
export const SlackRunRow = Schema.Struct({
  id: SlackRunId,
  kind: SlackRunKind,
  mode: SlackRunMode,
  status: SlackRunStatus,
  startedBy: Schema.String,
  createdAt: Timestamp,
});

export type SlackRunRowPin = Assert<Pinned<typeof SlackRunRow, GeneratedSlackRunRow>>;

/** `GET /admin/slack/runs`, newest first. */
export const SlackRunList = Schema.Struct({ runs: Schema.Array(SlackRunRow) });

export type SlackRunListPin = Assert<Pinned<typeof SlackRunList, GeneratedSlackRunList>>;

/** `GET /slack/imports`: the person's connection and their own runs. */
export const SlackPersonal = Schema.Struct({
  teamKnown: Schema.Boolean,
  connection: SlackConnectionState,
  connectPath: Schema.String,
  runs: Schema.Array(SlackRunRow),
});

export type SlackPersonalPin = Assert<Pinned<typeof SlackPersonal, GeneratedSlackPersonal>>;

/** How the Slack people matched. */
export const SlackPeople = Schema.Struct({
  total: Schema.Int,
  matched: Schema.Int,
  placeholders: Schema.Int,
  deactivated: Schema.Int,
  bots: Schema.Int,
});

export type SlackPeoplePin = Assert<Pinned<typeof SlackPeople, GeneratedSlackPeople>>;

/** What a run wrote (or would write). */
export const SlackCounts = Schema.Struct({
  roomsCreated: Schema.Int,
  roomsMerged: Schema.Int,
  messages: Schema.Int,
  replies: Schema.Int,
  threads: Schema.Int,
  reactions: Schema.Int,
  pins: Schema.Int,
  filesLinked: Schema.Int,
  skipped: Schema.Int,
});

export type SlackCountsPin = Assert<Pinned<typeof SlackCounts, GeneratedSlackCounts>>;

/** A conversation a dry run found. */
export const SlackConversation = Schema.Struct({
  id: Schema.String,
  name: Schema.String,
  kind: Schema.String,
  archived: Schema.Boolean,
  members: Schema.Int,
  messages: Schema.Int,
  threads: Schema.Int,
});

export type SlackConversationPin = Assert<
  Pinned<typeof SlackConversation, GeneratedSlackConversation>
>;

/** `GET .../runs/:id/status`: one run's progress, polled while it is active. */
export const SlackRun = Schema.Struct({
  id: SlackRunId,
  kind: SlackRunKind,
  mode: SlackRunMode,
  status: SlackRunStatus,
  title: Schema.String,
  startedBy: Schema.String,
  createdAt: Timestamp,
  startedAt: Schema.NullOr(Timestamp),
  finishedAt: Schema.NullOr(Timestamp),
  phase: Schema.NullOr(Schema.String),
  current: Schema.NullOr(Schema.String),
  queuedBehind: Schema.Boolean,
  people: Schema.NullOr(SlackPeople),
  counts: Schema.NullOr(SlackCounts),
  apiCalls: Schema.NullOr(Schema.Int),
  issuesCount: Schema.Int,
  error: Schema.NullOr(Schema.String),
  active: Schema.Boolean,
  cancellable: Schema.Boolean,
  undoable: Schema.Boolean,
  undoBlockedReason: Schema.NullOr(Schema.String),
  planReady: Schema.Boolean,
  catchUp: Schema.Boolean,
  conversations: Schema.Array(SlackConversation),
});

export type SlackRunPin = Assert<Pinned<typeof SlackRun, GeneratedSlackRun>>;

/** A problem a run recorded. */
export const SlackIssue = Schema.Struct({
  level: Schema.String,
  slackRef: Schema.NullOr(Schema.String),
  message: Schema.String,
});

export type SlackIssuePin = Assert<Pinned<typeof SlackIssue, GeneratedSlackIssue>>;

/** `GET /admin/slack/runs/:id?page=`: the run and a page of its issues. */
export const SlackRunPage = Schema.Struct({
  run: SlackRun,
  issues: Schema.Array(SlackIssue),
  nextPage: Schema.NullOr(Schema.Int),
});

export type SlackRunPagePin = Assert<Pinned<typeof SlackRunPage, GeneratedSlackRunPage>>;

/** A run write's answer: the run afterwards and the classic notice. */
export const SlackRunChange = Schema.Struct({ run: SlackRun, notice: Schema.String });

export type SlackRunChangePin = Assert<Pinned<typeof SlackRunChange, GeneratedSlackRunChange>>;

/** A conversation on the plan and the target the classic form preselects. */
export const SlackPlanConversation = Schema.Struct({
  conversation: SlackConversation,
  target: Schema.String,
});

export type SlackPlanConversationPin = Assert<
  Pinned<typeof SlackPlanConversation, GeneratedSlackPlanConversation>
>;

/** A room a conversation can merge into. */
export const SlackRoomTarget = Schema.Struct({ id: RoomId, name: Schema.String });

export type SlackRoomTargetPin = Assert<Pinned<typeof SlackRoomTarget, GeneratedSlackRoomTarget>>;

/** A sample message, as Slack wrote it and as Smartfire will show it. */
export const SlackSample = Schema.Struct({
  conversation: Schema.String,
  slackText: Schema.String,
  html: Schema.String,
});

export type SlackSamplePin = Assert<Pinned<typeof SlackSample, GeneratedSlackSample>>;

/** `GET /admin/slack/runs/:id/plan`: a completed dry run's plan. */
export const SlackPlan = Schema.Struct({
  runId: SlackRunId,
  conversations: Schema.Array(SlackPlanConversation),
  rooms: Schema.Array(SlackRoomTarget),
  samples: Schema.Array(SlackSample),
  defaultOldest: Schema.String,
});

export type SlackPlanPin = Assert<Pinned<typeof SlackPlan, GeneratedSlackPlan>>;

/** `POST /admin/slack/runs`: the days are `YYYY-MM-DD` in the viewer's zone. */
export const StartSlackDryRun = Schema.Struct({
  includePrivate: Schema.Boolean,
  oldest: Schema.NullOr(Schema.String),
  latest: Schema.NullOr(Schema.String),
});

export type StartSlackDryRunPin = Assert<
  Pinned<typeof StartSlackDryRun, GeneratedStartSlackDryRun>
>;

/** `POST /admin/slack/runs/:id/import`. */
export const StartSlackImport = Schema.Struct({
  conversationIds: Schema.Array(Schema.String),
  roomTargets: Schema.Record(Schema.String, Schema.String),
  preset: SlackPreset,
  oldest: Schema.NullOr(Schema.String),
  latest: Schema.NullOr(Schema.String),
});

export type StartSlackImportPin = Assert<
  Pinned<typeof StartSlackImport, GeneratedStartSlackImport>
>;

/** `POST /slack/imports`: a preview, or an import of a completed preview's checked conversations. */
export const StartPersonalSlackImport = Schema.Struct({
  mode: SlackRunMode,
  dryRunId: Schema.NullOr(Schema.Int),
  conversationIds: Schema.Array(Schema.String),
});

export type StartPersonalSlackImportPin = Assert<
  Pinned<typeof StartPersonalSlackImport, GeneratedStartPersonalSlackImport>
>;
