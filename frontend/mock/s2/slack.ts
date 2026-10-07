/**
 * The Slack importer (S7): the classic setup, run and personal import pages' twin under
 * `/admin/slack/*`, `/slack/imports/*` and `/slack/connection`, kept per world so `reset()` starts
 * it over. The Slack app starts set up and the viewer connected, with one completed workspace dry
 * run to plan from. Runs don't talk to Slack: each read of an active run moves it one step on
 * (queued, then running, then finished), so the pages' polling sees progress.
 */
import type { ApiError } from "../../src/gen/ApiError.ts";
import type { SlackConnectionState } from "../../src/gen/SlackConnectionState.ts";
import type { SlackConversation } from "../../src/gen/SlackConversation.ts";
import type { SlackIssue } from "../../src/gen/SlackIssue.ts";
import type { SlackRun } from "../../src/gen/SlackRun.ts";
import type { SlackRunKind } from "../../src/gen/SlackRunKind.ts";
import type { SlackRunMode } from "../../src/gen/SlackRunMode.ts";
import type { SlackRunRow } from "../../src/gen/SlackRunRow.ts";
import type { SlackRunStatus } from "../../src/gen/SlackRunStatus.ts";
import type { SlackSetup } from "../../src/gen/SlackSetup.ts";
import { HttpError, notFound, ok } from "../http.ts";
import { field, intField, type Json, stringArrayField, stringField } from "../json.ts";
import { timestamp, VIEWER_ID, type World } from "../seed.ts";
import { firstId, type Route, route, type S2Context } from "./context.ts";

const ADMIN_CONNECT = "/slack/oauth/start?return_to=%2Faccount%2Fslack_import";

const PERSONAL_CONNECT = "/slack/oauth/start?return_to=%2Fslack%2Fimports";

const MANIFEST = `display_information:
  name: Smartfire import
oauth_config:
  redirect_urls:
    - http://127.0.0.1/slack/oauth/callback
  scopes:
    user:
      - channels:history
      - channels:read
      - groups:history
      - users:read`;

/** What a workspace dry run finds. */
const WORKSPACE_CONVERSATIONS: readonly SlackConversation[] = [
  {
    id: "C100",
    name: "general",
    kind: "Public channel",
    archived: false,
    members: 12,
    messages: 340,
    threads: 22,
  },
  {
    id: "C200",
    name: "design",
    kind: "Public channel",
    archived: false,
    members: 5,
    messages: 88,
    threads: 4,
  },
  {
    id: "G300",
    name: "leads",
    kind: "Private channel",
    archived: true,
    members: 3,
    messages: 12,
    threads: 0,
  },
];

/** What a person's preview finds. */
const PERSONAL_CONVERSATIONS: readonly SlackConversation[] = [
  {
    id: "D100",
    name: "Grace",
    kind: "Direct message",
    archived: false,
    members: 2,
    messages: 41,
    threads: 1,
  },
  {
    id: "M200",
    name: "grace, alan, you",
    kind: "Group DM",
    archived: false,
    members: 3,
    messages: 9,
    threads: 0,
  },
];

const SAMPLES = [
  {
    conversation: "general",
    slackText: "*Ship it* today",
    html: "<p><strong>Ship it</strong> today</p>",
  },
];

interface RunRecord {
  id: number;
  kind: SlackRunKind;
  mode: SlackRunMode;
  status: SlackRunStatus;
  createdAt: string;
  startedAt: string | null;
  finishedAt: string | null;
  conversations: readonly SlackConversation[];
  /** A test import's oldest day, which rules out a catch-up. */
  oldest: string | null;
  /** How many issues it recorded. */
  issues: number;
}

interface State {
  clientId: string | null;
  secretSaved: boolean;
  configuredBy: string | null;
  teamName: string | null;
  connection: SlackConnectionState;
  runs: Map<number, RunRecord>;
  nextId: number;
}

const VALIDATION: ApiError["_tag"] = "Validation";

/** Issues a run page shows at a time, as the classic page pages them. */
const ISSUES_PER_PAGE = 50;

/** The issues a finished workspace dry run recorded: two pages' worth. */
const DRY_RUN_ISSUES = 60;

/**
 * The issues a live workspace dry run has recorded at each step: they pile up while it runs, so
 * the pages grow under the person reading them (queued, running, completed).
 */
const LIVE_DRY_RUN_ISSUES = { queued: 60, running: 110, completed: 160 } as const;

/**
 * A page of a run's issues in the order they were recorded (`ORDER BY id`), as the classic page's
 * `?page=N` shows them: new issues land on the last page.
 */
function issuesPage(record: RunRecord, raw: string | null) {
  const asked = Number(raw ?? "1");
  const page = Number.isSafeInteger(asked) && asked > 0 ? asked : 1;
  const from = (page - 1) * ISSUES_PER_PAGE + 1;
  const to = Math.min(page * ISSUES_PER_PAGE, record.issues);
  const issues: SlackIssue[] = [];

  for (let n = from; n <= to; n += 1) {
    issues.push({
      level: n % 10 === 0 ? "error" : "warning",
      slackRef: `F${1000 + n}`,
      message: `File ${n} wasn't imported (files stay in Slack).`,
    });
  }

  return { issues, nextPage: page * ISSUES_PER_PAGE < record.issues ? page + 1 : null };
}

/** The classic page's alert, a 422 naming no field. */
const refusal = (message: string): HttpError =>
  new HttpError(422, { _tag: VALIDATION, message, fields: {} });

const ACTIVE: readonly SlackRunStatus[] = ["queued", "running", "undoing"];

const isActive = (record: RunRecord) => ACTIVE.includes(record.status);

function initialState(now: number): State {
  const at = timestamp(now - 3_600_000);

  return {
    clientId: "1234.5678",
    secretSaved: true,
    configuredBy: null,
    teamName: "Acme",
    connection: { state: "connected" },
    runs: new Map([
      [
        1,
        {
          id: 1,
          kind: "workspace",
          mode: "dry_run",
          status: "completed",
          createdAt: at,
          startedAt: at,
          finishedAt: at,
          conversations: WORKSPACE_CONVERSATIONS,
          oldest: null,
          issues: DRY_RUN_ISSUES,
        },
      ],
    ]),
    nextId: 2,
  };
}

/** The Slack importer module. */
export interface SlackModule {
  readonly routes: readonly Route[];
}

/**
 * Creates the Slack importer module. `requireSudo` throws `SudoRequired` while the password
 * confirmation has lapsed (the admin module's switch).
 */
export function createSlack(ctx: S2Context, requireSudo: () => void): SlackModule {
  let state: State | null = null;
  let stateWorld: World | null = null;

  const current = (): State => {
    const world = ctx.world();

    if (state === null || stateWorld !== world) {
      state = initialState(ctx.now());
      stateWorld = world;
    }

    return state;
  };

  const viewerName = () => ctx.world().users.get(VIEWER_ID)?.name ?? "You";

  const configured = (s: State) => s.clientId !== null && s.secretSaved;

  const activeRun = () => [...current().runs.values()].find(isActive) ?? null;

  const setup = (): SlackSetup => {
    const s = current();
    const active = activeRun();

    return {
      clientId: s.clientId,
      configured: configured(s),
      configuredBy: s.configuredBy,
      teamName: s.teamName,
      teamKnown: s.teamName !== null,
      connection: s.connection,
      activeRun:
        active === null
          ? null
          : { id: active.id, kind: active.kind, mode: active.mode, status: active.status },
      manifest: MANIFEST,
      connectPath: ADMIN_CONNECT,
    };
  };

  const finished = (record: RunRecord) =>
    record.status === "completed" || record.status === "failed" || record.status === "cancelled";

  /**
   * Why an import can't be undone yet, in the classic page's words: a later import of some of the
   * same conversations comes off first, and nothing else may be running.
   */
  const undoBlocked = (record: RunRecord): string | null => {
    if (record.mode !== "import" || !finished(record)) return null;

    const mine = new Set(record.conversations.map((each) => each.id));

    const later = [...current().runs.values()].find(
      (other) =>
        other.id > record.id &&
        other.mode === "import" &&
        other.status !== "undone" &&
        other.conversations.some((each) => mine.has(each.id)),
    );

    if (later !== undefined) {
      return `A later import (#${later.id}) also imported some of these conversations; undo that one first.`;
    }

    return activeRun() === null
      ? null
      : "Another import is queued or running. Wait for it to finish, then undo.";
  };

  const wire = (record: RunRecord): SlackRun => {
    const done = record.status === "completed" || record.status === "undone";
    const kind = record.kind === "workspace" ? "Workspace" : "Personal";
    const importing = record.mode === "import";

    return {
      id: record.id,
      kind: record.kind,
      mode: record.mode,
      status: record.status,
      title: `${kind} ${record.mode.replace("_", " ")} #${record.id}`,
      startedBy: viewerName(),
      createdAt: record.createdAt,
      startedAt: record.startedAt,
      finishedAt: record.finishedAt,
      phase: record.status === "running" ? "conversations" : null,
      current:
        record.status === "running" ? `#${record.conversations[0]?.name ?? "general"}` : null,
      queuedBehind: false,
      people:
        record.status === "queued"
          ? null
          : { total: 14, matched: 11, placeholders: 2, deactivated: 1, bots: 0 },
      counts:
        done && importing
          ? {
              roomsCreated: 2,
              roomsMerged: 1,
              messages: 428,
              replies: 61,
              threads: 26,
              reactions: 90,
              pins: 3,
              filesLinked: 7,
              skipped: 0,
            }
          : null,
      apiCalls: record.status === "queued" ? null : 57,
      issuesCount: record.issues,
      error: null,
      active: isActive(record),
      cancellable: record.status === "queued" || record.status === "running",
      undoable: importing && finished(record) && undoBlocked(record) === null,
      undoBlockedReason: undoBlocked(record),
      planReady:
        record.kind === "workspace" && record.mode === "dry_run" && record.status === "completed",
      catchUp:
        record.kind === "workspace" &&
        importing &&
        record.status === "completed" &&
        record.oldest === null,
      conversations:
        record.kind === "personal" && record.mode === "dry_run" && record.status === "completed"
          ? [...record.conversations]
          : [],
    };
  };

  const row = (record: RunRecord): SlackRunRow => ({
    id: record.id,
    kind: record.kind,
    mode: record.mode,
    status: record.status,
    startedBy: viewerName(),
    createdAt: record.createdAt,
  });

  /** An active run moves one step on each time it's read; a dry run's issues pile up as it goes. */
  const advance = (record: RunRecord) => {
    const at = timestamp(ctx.now());
    const live = record.kind === "workspace" && record.mode === "dry_run";

    if (record.status === "queued") {
      record.status = "running";
      record.startedAt = at;
      record.issues = live ? LIVE_DRY_RUN_ISSUES.running : record.issues;
    } else if (record.status === "running") {
      record.status = "completed";
      record.finishedAt = at;
      record.issues = live ? LIVE_DRY_RUN_ISSUES.completed : record.issues;
    } else if (record.status === "undoing") {
      record.status = "undone";
      record.finishedAt = at;
    }
  };

  const runOr404 = (id: number, admin: boolean): RunRecord => {
    const found = current().runs.get(id);

    if (found === undefined || (!admin && found.kind !== "personal")) throw notFound();

    return found;
  };

  const start = (
    kind: SlackRunKind,
    mode: SlackRunMode,
    conversations: readonly SlackConversation[],
    oldest: string | null,
  ): RunRecord => {
    const s = current();

    const record: RunRecord = {
      id: s.nextId,
      kind,
      mode,
      status: "queued",
      createdAt: timestamp(ctx.now()),
      startedAt: null,
      finishedAt: null,
      conversations,
      oldest,
      // A workspace dry run finds files it won't import; the other runs record none.
      issues: kind === "workspace" && mode === "dry_run" ? LIVE_DRY_RUN_ISSUES.queued : 0,
    };

    s.nextId += 1;
    s.runs.set(record.id, record);

    return record;
  };

  const blocker = (personal: boolean) => {
    const s = current();

    if (s.teamName === null && personal) {
      return "An administrator needs to set up Slack import first.";
    }

    if (s.connection.state !== "connected") return "Connect your Slack account first.";

    if (activeRun() !== null) {
      return personal
        ? "You already have an import running. Wait for it to finish."
        : "Another import is already running. Wait for it to finish.";
    }

    return null;
  };

  const started = (record: RunRecord, notice: string) => ok({ run: wire(record), notice });

  const selection = (body: Json | undefined, from: readonly SlackConversation[], what: string) => {
    const ids = (stringArrayField(body, "conversationIds") ?? []).filter((id) => id.trim() !== "");

    if (ids.length === 0) throw refusal("Check at least one conversation to import.");

    const picked = from.filter((each) => ids.includes(each.id));

    if (picked.length === 0) throw refusal(`Those conversations are not in the ${what}.`);

    return picked;
  };

  const change = (id: number, admin: boolean, undo: boolean) => {
    const record = runOr404(id, admin);
    const view = wire(record);

    if (undo) {
      if (!view.undoable) throw refusal(view.undoBlockedReason ?? "That run cannot be undone.");

      record.status = "undoing";

      return started(record, "Undo started.");
    }

    if (!view.cancellable) throw refusal("That run already finished.");

    record.status = "cancelled";
    record.finishedAt = timestamp(ctx.now());

    return started(record, "Import cancelled.");
  };

  const runs = (personal: boolean) =>
    [...current().runs.values()]
      .filter((record) => !personal || record.kind === "personal")
      .sort((a, b) => b.id - a.id)
      .map(row);

  return {
    routes: [
      route("GET", /^\/admin\/slack$/, () => ok(setup())),
      route("PUT", /^\/admin\/slack$/, ({ body }) => {
        requireSudo();

        const s = current();
        const clientId = (stringField(body, "clientId") ?? "").trim();
        const secret = (stringField(body, "clientSecret") ?? "").trim();

        const errors = [
          ...(clientId === "" ? ["Client ID can't be blank"] : []),
          ...(secret === "" && !s.secretSaved ? ["Client secret can't be blank"] : []),
        ];

        if (errors.length > 0) throw refusal(errors.join(", "));

        s.clientId = clientId;
        s.secretSaved = true;
        s.configuredBy = viewerName();

        return ok({ setup: setup(), notice: "Slack app credentials saved." });
      }),
      route("DELETE", /^\/admin\/slack$/, () => {
        requireSudo();

        if (activeRun() !== null) throw refusal("Finish or cancel the running import first.");

        const s = current();

        s.clientId = null;
        s.secretSaved = false;
        s.configuredBy = null;
        s.teamName = null;
        s.connection = { state: "none" };

        return ok({ setup: setup(), notice: "Slack credentials removed." });
      }),
      route("GET", /^\/admin\/slack\/runs$/, () => ok({ runs: runs(false) })),
      route("POST", /^\/admin\/slack\/runs$/, () => {
        const reason = blocker(false);

        if (reason !== null) throw refusal(reason);

        return started(
          start("workspace", "dry_run", WORKSPACE_CONVERSATIONS, null),
          "Dry run started.",
        );
      }),
      route("GET", /^\/admin\/slack\/runs\/(\d+)$/, (request) => {
        const record = runOr404(firstId(request), true);

        return ok({ run: wire(record), ...issuesPage(record, request.query.get("page")) });
      }),
      route("GET", /^\/admin\/slack\/runs\/(\d+)\/status$/, (request) => {
        const record = runOr404(firstId(request), true);

        advance(record);

        return ok(wire(record));
      }),
      route("GET", /^\/admin\/slack\/runs\/(\d+)\/plan$/, (request) => {
        const record = runOr404(firstId(request), true);

        if (!wire(record).planReady) {
          throw refusal("The plan is ready when the dry run completes.");
        }

        return ok({
          runId: record.id,
          conversations: record.conversations.map((conversation) => ({
            conversation,
            target: conversation.name === "general" ? "1" : "new",
          })),
          rooms: [...ctx.world().rooms.values()].flatMap((each) =>
            each.room.kind === "direct" ? [] : [{ id: each.room.id, name: each.room.name ?? "" }],
          ),
          samples: SAMPLES,
          defaultOldest: timestamp(ctx.now() - 14 * 86_400_000).slice(0, 10),
        });
      }),
      route("POST", /^\/admin\/slack\/runs\/(\d+)\/import$/, (request) => {
        const record = runOr404(firstId(request), true);

        if (!wire(record).planReady) throw refusal("Start from a completed dry run.");

        const picked = selection(request.body, record.conversations, "dry run");
        const reason = blocker(false);

        if (reason !== null) throw refusal(reason);

        const full = field(request.body, "preset") === "full";
        const oldest = full ? null : stringField(request.body, "oldest");

        return started(
          start("workspace", "import", picked, oldest),
          full ? "Full import started." : "Test import started.",
        );
      }),
      route("POST", /^\/admin\/slack\/runs\/(\d+)\/catch_up$/, (request) => {
        const record = runOr404(firstId(request), true);

        if (!wire(record).catchUp) {
          throw refusal("Catch-up starts from a completed full import.");
        }

        const reason = blocker(false);

        if (reason !== null) throw refusal(reason);

        return started(
          start("workspace", "import", record.conversations, null),
          "Catch-up import started.",
        );
      }),
      route("POST", /^\/admin\/slack\/runs\/(\d+)\/cancel$/, (request) =>
        change(firstId(request), true, false),
      ),
      route("POST", /^\/admin\/slack\/runs\/(\d+)\/undo$/, (request) =>
        change(firstId(request), true, true),
      ),
      route("GET", /^\/slack\/imports$/, () => {
        const s = current();

        return ok({
          teamKnown: s.teamName !== null,
          connection: s.connection,
          connectPath: PERSONAL_CONNECT,
          runs: runs(true),
        });
      }),
      route("POST", /^\/slack\/imports$/, ({ body }) => {
        if (field(body, "mode") !== "import") {
          const reason = blocker(true);

          if (reason !== null) throw refusal(reason);

          return started(
            start("personal", "dry_run", PERSONAL_CONVERSATIONS, null),
            "Preview started.",
          );
        }

        const preview = current().runs.get(intField(body, "dryRunId") ?? 0);

        if (
          preview === undefined ||
          preview.kind !== "personal" ||
          preview.mode !== "dry_run" ||
          preview.status !== "completed"
        ) {
          throw refusal("Run a preview first.");
        }

        const picked = selection(body, preview.conversations, "preview");
        const reason = blocker(true);

        if (reason !== null) throw refusal(reason);

        return started(start("personal", "import", picked, null), "Import started.");
      }),
      route("GET", /^\/slack\/imports\/(\d+)(?:\/status)?$/, (request) => {
        const record = runOr404(firstId(request), false);

        advance(record);

        return ok(wire(record));
      }),
      route("POST", /^\/slack\/imports\/(\d+)\/cancel$/, (request) =>
        change(firstId(request), false, false),
      ),
      route("POST", /^\/slack\/imports\/(\d+)\/undo$/, (request) =>
        change(firstId(request), false, true),
      ),
      route("DELETE", /^\/slack\/connection$/, () => {
        requireSudo();

        const own = [...current().runs.values()].some(
          (record) => record.kind === "personal" && isActive(record),
        );

        if (own) throw refusal("Finish or cancel your running Slack import first.");

        current().connection = { state: "none" };

        return ok({ notice: "Slack disconnected." });
      }),
    ],
  };
}
