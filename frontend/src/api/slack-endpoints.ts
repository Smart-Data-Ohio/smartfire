/**
 * The S7 Slack importer endpoints: `/api/v1/admin/slack/*` (administrators: the app credentials
 * and the workspace runs) and `/api/v1/slack/*` (everyone: their own personal runs and their Slack
 * connection). Saving or removing the credentials and disconnecting wait for a fresh confirmation
 * once the last one has lapsed; a start the classic page would refuse fails with
 * `Validation` and the classic alert. Connecting a Slack account stays a classic OAuth round trip.
 */
import { Effect } from "effect";
import type { SaveSlackCredentials } from "../gen/SaveSlackCredentials.ts";
import type { SlackDisconnected } from "../gen/SlackDisconnected.ts";
import type { SlackPersonal } from "../gen/SlackPersonal.ts";
import type { SlackPlan } from "../gen/SlackPlan.ts";
import type { SlackRun } from "../gen/SlackRun.ts";
import type { SlackRunChange } from "../gen/SlackRunChange.ts";
import type { SlackRunList } from "../gen/SlackRunList.ts";
import type { SlackRunPage } from "../gen/SlackRunPage.ts";
import type { SlackSetup } from "../gen/SlackSetup.ts";
import type { SlackSetupChange } from "../gen/SlackSetupChange.ts";
import type { StartPersonalSlackImport } from "../gen/StartPersonalSlackImport.ts";
import type { StartSlackDryRun } from "../gen/StartSlackDryRun.ts";
import type { StartSlackImport } from "../gen/StartSlackImport.ts";
import { call, get } from "./call.ts";
import {
  SlackDisconnected as SlackDisconnectedSchema,
  SlackPersonal as SlackPersonalSchema,
  SlackPlan as SlackPlanSchema,
  SlackRunChange as SlackRunChangeSchema,
  SlackRunList as SlackRunListSchema,
  SlackRunPage as SlackRunPageSchema,
  SlackRun as SlackRunSchema,
  SlackSetupChange as SlackSetupChangeSchema,
  SlackSetup as SlackSetupSchema,
} from "./schema/slack.ts";
import { wire } from "./wire.ts";

const runReply = wire<SlackRun>(SlackRunSchema);

const changeReply = wire<SlackRunChange>(SlackRunChangeSchema);

const setupChangeReply = wire<SlackSetupChange>(SlackSetupChangeSchema);

/** Where a run's pages are: an administrator's workspace pages, or the person's own. */
const runPath = (admin: boolean, runId: number) =>
  admin ? `/admin/slack/runs/${runId}` : `/slack/imports/${runId}`;

/** `GET /admin/slack`: the app credentials, the administrator's connection and the manifest. */
export const slackSetup = Effect.fn("api.slackSetup")(function* () {
  return yield* call(get("/admin/slack"), wire<SlackSetup>(SlackSetupSchema));
});

/** `PUT /admin/slack`: saves the Client ID and, when given, a new Client Secret. */
export const saveSlackCredentials = Effect.fn("api.saveSlackCredentials")(function* (
  body: SaveSlackCredentials,
) {
  return yield* call({ method: "PUT", path: "/admin/slack", body, secret: true }, setupChangeReply);
});

/** `DELETE /admin/slack`: removes the credentials and every member's connection. */
export const removeSlackCredentials = Effect.fn("api.removeSlackCredentials")(function* () {
  return yield* call({ method: "DELETE", path: "/admin/slack" }, setupChangeReply);
});

/** `GET /admin/slack/runs`: every run, newest first. */
export const slackRuns = Effect.fn("api.slackRuns")(function* () {
  return yield* call(get("/admin/slack/runs"), wire<SlackRunList>(SlackRunListSchema));
});

/** `POST /admin/slack/runs`: a workspace dry run. */
export const startSlackDryRun = Effect.fn("api.startSlackDryRun")(function* (
  body: StartSlackDryRun,
) {
  return yield* call({ method: "POST", path: "/admin/slack/runs", body }, changeReply);
});

/** `GET /admin/slack/runs/:id?page=`: the run and a page of its issues. */
export const slackRunPage = Effect.fn("api.slackRunPage")(function* (
  runId: number,
  page: number | null,
) {
  return yield* call(
    get(runPath(true, runId), page === null ? undefined : { page: `${page}` }),
    wire<SlackRunPage>(SlackRunPageSchema),
  );
});

/** `GET .../:id/status` (or a personal run itself): one run's progress, polled while active. */
export const slackRunStatus = Effect.fn("api.slackRunStatus")(function* (
  admin: boolean,
  runId: number,
) {
  return yield* call(
    get(admin ? `${runPath(true, runId)}/status` : runPath(false, runId)),
    runReply,
  );
});

/** `GET /admin/slack/runs/:id/plan`: a completed dry run's plan. */
export const slackPlan = Effect.fn("api.slackPlan")(function* (runId: number) {
  return yield* call(get(`${runPath(true, runId)}/plan`), wire<SlackPlan>(SlackPlanSchema));
});

/** `POST /admin/slack/runs/:id/import`: a test or full import of the plan's checked conversations. */
export const startSlackImport = Effect.fn("api.startSlackImport")(function* (
  runId: number,
  body: StartSlackImport,
) {
  return yield* call({ method: "POST", path: `${runPath(true, runId)}/import`, body }, changeReply);
});

/** `POST /admin/slack/runs/:id/catch_up`: the same conversations again, from a full import. */
export const startSlackCatchUp = Effect.fn("api.startSlackCatchUp")(function* (runId: number) {
  return yield* call({ method: "POST", path: `${runPath(true, runId)}/catch_up` }, changeReply);
});

/** `POST .../:id/cancel`. */
export const cancelSlackRun = Effect.fn("api.cancelSlackRun")(function* (
  admin: boolean,
  runId: number,
) {
  return yield* call({ method: "POST", path: `${runPath(admin, runId)}/cancel` }, changeReply);
});

/** `POST .../:id/undo`: removes what the import wrote, last in first out. */
export const undoSlackRun = Effect.fn("api.undoSlackRun")(function* (
  admin: boolean,
  runId: number,
) {
  return yield* call({ method: "POST", path: `${runPath(admin, runId)}/undo` }, changeReply);
});

/** `GET /slack/imports`: the person's connection and their own runs. */
export const personalSlack = Effect.fn("api.personalSlack")(function* () {
  return yield* call(get("/slack/imports"), wire<SlackPersonal>(SlackPersonalSchema));
});

/** `POST /slack/imports`: a preview, or an import from a completed one. */
export const startPersonalSlack = Effect.fn("api.startPersonalSlack")(function* (
  body: StartPersonalSlackImport,
) {
  return yield* call({ method: "POST", path: "/slack/imports", body }, changeReply);
});

/** `DELETE /slack/connection`: drops the person's own Slack connection. */
export const disconnectSlack = Effect.fn("api.disconnectSlack")(function* () {
  return yield* call(
    { method: "DELETE", path: "/slack/connection" },
    wire<SlackDisconnected>(SlackDisconnectedSchema),
  );
});
