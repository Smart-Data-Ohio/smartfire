import { Schema } from "effect";
import { describe, expect, it } from "vitest";
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
} from "../../src/api/schema/slack.ts";
import type { SlackPersonal } from "../../src/gen/SlackPersonal.ts";
import type { SlackPlan } from "../../src/gen/SlackPlan.ts";
import type { SlackRun } from "../../src/gen/SlackRun.ts";
import type { SlackRunChange } from "../../src/gen/SlackRunChange.ts";
import type { SlackRunList } from "../../src/gen/SlackRunList.ts";
import type { SlackRunPage } from "../../src/gen/SlackRunPage.ts";
import type { SlackSetup } from "../../src/gen/SlackSetup.ts";
import type { SlackSetupChange } from "../../src/gen/SlackSetupChange.ts";
import type { Json } from "../json.ts";
import type { MockServer } from "../server.ts";
import { errorOf, expectStatus, get, harness, send } from "./testing.ts";

const API = "/api/v1";

/** Starts a run and checks the reply's shape. */
async function started(
  server: MockServer,
  path: string,
  body: Json,
  notice: string,
): Promise<SlackRun> {
  const change = await expectStatus<SlackRunChange>(server, "POST", `${API}${path}`, body, 200);

  Schema.decodeUnknownSync(SlackRunChangeSchema)(change);
  expect(change.notice).toBe(notice);

  return change.run;
}

/** Expects the classic alert as a 422. */
async function refused(server: MockServer, method: string, path: string, body: Json) {
  const json = await expectStatus<Json>(server, method, `${API}${path}`, body, 422);

  return errorOf(json);
}

/** Reads an active run's status until it settles, returning each status seen. */
async function settle(server: MockServer, path: string): Promise<string[]> {
  const seen: string[] = [];

  for (let read = 0; read < 5; read += 1) {
    const run = await get<SlackRun>(server, `${API}${path}`);

    Schema.decodeUnknownSync(SlackRunSchema)(run);
    seen.push(run.status);

    if (!run.active) break;
  }

  return seen;
}

describe("the mock's Slack import pages", () => {
  it("answer the contract's shapes", async () => {
    const { server } = harness();
    const setup = await get<SlackSetup>(server, `${API}/admin/slack`);

    Schema.decodeUnknownSync(SlackSetupSchema)(setup);
    expect(setup.configured).toBe(true);
    expect(setup.connection.state).toBe("connected");

    const runs = await get<SlackRunList>(server, `${API}/admin/slack/runs`);

    Schema.decodeUnknownSync(SlackRunListSchema)(runs);
    expect(runs.runs.map((run) => run.id)).toEqual([1]);

    Schema.decodeUnknownSync(SlackRunPageSchema)(
      await get<SlackRunPage>(server, `${API}/admin/slack/runs/1`),
    );

    const plan = await get<SlackPlan>(server, `${API}/admin/slack/runs/1/plan`);

    Schema.decodeUnknownSync(SlackPlanSchema)(plan);
    expect(plan.conversations.map((row) => row.conversation.id)).toEqual(["C100", "C200", "G300"]);

    Schema.decodeUnknownSync(SlackPersonalSchema)(
      await get<SlackPersonal>(server, `${API}/slack/imports`),
    );
  });

  it("move a dry run on with each status read", async () => {
    const { server } = harness();

    const run = await started(
      server,
      "/admin/slack/runs",
      { includePrivate: true, oldest: null, latest: null },
      "Dry run started.",
    );

    expect(run.status).toBe("queued");
    expect(await settle(server, `/admin/slack/runs/${run.id}/status`)).toEqual([
      "running",
      "completed",
    ]);
  });

  it("refuse a second run while one is active", async () => {
    const { server } = harness();
    const body = { includePrivate: true, oldest: null, latest: null };

    await started(server, "/admin/slack/runs", body, "Dry run started.");

    expect(await refused(server, "POST", "/admin/slack/runs", body)).toEqual({
      tag: "Validation",
      message: "Another import is already running. Wait for it to finish.",
    });
  });

  it("import the checked conversations, then undo it", async () => {
    const { server } = harness();

    const body = (ids: string[]) => ({
      conversationIds: ids,
      roomTargets: {},
      preset: "full",
      oldest: null,
      latest: null,
    });

    expect((await refused(server, "POST", "/admin/slack/runs/1/import", body([]))).message).toBe(
      "Check at least one conversation to import.",
    );

    const run = await started(
      server,
      "/admin/slack/runs/1/import",
      body(["C100"]),
      "Full import started.",
    );

    await settle(server, `/admin/slack/runs/${run.id}/status`);

    const done = await get<SlackRun>(server, `${API}/admin/slack/runs/${run.id}/status`);

    expect(done.undoable).toBe(true);
    expect(done.catchUp).toBe(true);

    await started(server, `/admin/slack/runs/${run.id}/undo`, null, "Undo started.");
    expect(await settle(server, `/admin/slack/runs/${run.id}/status`)).toEqual(["undone"]);
  });

  it("guard the credentials with the password confirmation", async () => {
    const { server } = harness();

    await send(server, "POST", "/__mock/lapse-sudo", { on: true });

    const lapsed = await expectStatus<Json>(
      server,
      "PUT",
      `${API}/admin/slack`,
      { clientId: "9", clientSecret: null },
      403,
    );

    expect(errorOf(lapsed).tag).toBe("SudoRequired");

    await send(server, "POST", "/__mock/lapse-sudo", { on: false });

    const change = await expectStatus<SlackSetupChange>(
      server,
      "PUT",
      `${API}/admin/slack`,
      { clientId: "9", clientSecret: null },
      200,
    );

    Schema.decodeUnknownSync(SlackSetupChangeSchema)(change);
    expect(change.setup.clientId).toBe("9");
  });

  it("preview and import a person's own conversations", async () => {
    const { server } = harness();

    const preview = await started(
      server,
      "/slack/imports",
      { mode: "dry_run", dryRunId: null, conversationIds: [] },
      "Preview started.",
    );

    await settle(server, `/slack/imports/${preview.id}`);

    const done = await get<SlackRun>(server, `${API}/slack/imports/${preview.id}`);

    expect(done.conversations.map((each) => each.id)).toEqual(["D100", "M200"]);

    const imported = await started(
      server,
      "/slack/imports",
      { mode: "import", dryRunId: preview.id, conversationIds: ["D100"] },
      "Import started.",
    );

    expect(imported.kind).toBe("personal");
    // The workspace run is the administrator's, not the person's.
    expect((await send(server, "GET", `${API}/slack/imports/1`)).status).toBe(404);

    const disconnected = await expectStatus<Json>(
      server,
      "DELETE",
      `${API}/slack/connection`,
      null,
      422,
    );

    expect(errorOf(disconnected).message).toBe("Finish or cancel your running Slack import first.");

    await settle(server, `/slack/imports/${imported.id}`);
    Schema.decodeUnknownSync(SlackDisconnectedSchema)(
      await expectStatus<Json>(server, "DELETE", `${API}/slack/connection`, null, 200),
    );
  });
});
