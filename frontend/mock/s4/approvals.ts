/**
 * Agents' approval requests (S4): `GET /agents/:agentId/approvals`, `PATCH /agent_approvals/:id`
 * and `approval.updated`. The seed gives Ember the seven requests the S3 inbox already shows (same
 * ids, summaries and states) plus a page and more of older history, and a few to the other agents.
 * Each request is presented per viewer, as the contract says: `roomName` gated on room membership,
 * `approvable`/`deniable` from who the viewer is.
 *
 * A request records an `agent_approval_request` activity item; a decision updates that item's
 * `approvalStatus` and publishes `approval.updated`. The simulation has Ember ask now and then, and
 * Priya (another administrator) decide one now and then.
 */
import type { ActivityItem } from "../../src/gen/ActivityItem.ts";
import type { AgentApproval } from "../../src/gen/AgentApproval.ts";
import type { AgentApprovalPage } from "../../src/gen/AgentApprovalPage.ts";
import type { AgentApprovalStatus } from "../../src/gen/AgentApprovalStatus.ts";
import type { ApprovalDecision } from "../../src/gen/ApprovalDecision.ts";
import type { ApprovalUpdated } from "../../src/gen/ApprovalUpdated.ts";
import { forbidden, notFound, ok } from "../http.ts";
import { field, type Json, stringField } from "../json.ts";
import type { Random } from "../random.ts";
import { firstId, type Route, route, type S2Context } from "../s2/context.ts";
import { iso } from "../s2/model.ts";
import type { Activity, ActivityDraft } from "../s3/activity.ts";
import { conversationTitle } from "../s3/conversations.ts";
import { beforeOf, oneOf } from "../s3/model.ts";
import { APPROVALS, seededApprovalAt, seededApprovalId } from "../s3/seed.ts";
import { ROOM_IDS, rowTimestamp, touchedRow, USER_IDS, VIEWER_ID } from "../seed.ts";
import {
  AGENT_IDS,
  type Agents,
  HIDDEN_ROOM,
  viewerIsAdmin,
  viewerManages,
  viewerRoomName,
} from "./agents.ts";
import { sentence, validation } from "./http.ts";

const MINUTE = 60_000;

const HOUR = 60 * MINUTE;

const DAY = 24 * HOUR;

/** At most this many a page, as the server pages. */
export const APPROVAL_PAGE_SIZE = 50;

/** How long a request waits for a decision. */
const EXPIRES_AFTER_MS = 2 * DAY;

/** Requests made at run time count up from here. */
const FIRST_LIVE_APPROVAL_ID = 1000;

const STATUSES: readonly AgentApprovalStatus[] = [
  "pending",
  "approved",
  "denied",
  "cancelled",
  "expired",
];

/** A request as the server keeps it; `status` is the stored one (see `effectiveStatus`). */
interface ApprovalRecord {
  readonly id: number;
  readonly agentId: number;
  readonly roomId: number | null;
  readonly action: string;
  readonly summary: string;
  status: AgentApprovalStatus;
  readonly expiresAt: number;
  readonly createdAt: number;
  decidedById: number | null;
  decidedAt: number | null;
  decisionNote: string | null;
  updatedAt: string;
}

/** The action each of S3's seven inbox requests asks for, with who decided it and the note. */
const INBOX_DETAILS: readonly (readonly [string, number | null, string | null])[] = [
  ["deploy.production", null, null],
  ["github.merge_pull_request", null, null],
  ["messages.post", USER_IDS.riel, null],
  ["github.create_issue", USER_IDS.priya, "Two of the three are duplicates of #301."],
  ["secrets.rotate", null, null],
  ["github.close_issue", USER_IDS.priya, null],
  ["rooms.invite", null, null],
];

/** Older requests, cycled to fill Ember's history past a page. */
const HISTORY: readonly (readonly [string, string])[] = [
  ["messages.post", "Post the standup summary to #engineering"],
  ["github.create_issue", "Open an issue for the flaky huddle gateway test"],
  ["deploy.staging", "Deploy main to staging"],
  ["github.merge_pull_request", "Merge PR #297 into main"],
  ["messages.post", "Post the release notes draft to #announcements"],
  ["calendar.create_event", "Book the retro for Friday at 3pm"],
  ["fizzy.create_card", "Add a card for the Safari websocket bug"],
];

const HISTORY_SIZE = 56;

/** How the `index`-th older request ended: every fifth denied, some expired, the rest approved. */
function historyStatus(index: number): AgentApprovalStatus {
  if (index % 5 === 0) return "denied";

  return index % 7 === 0 ? "expired" : "approved";
}

/** A decision a `PATCH` body asks for. */
interface DecisionBody {
  readonly decision: ApprovalDecision;
  readonly note: string | null;
}

/** What the simulation's Ember asks for. */
const LIVE_REQUESTS: readonly (readonly [string, string])[] = [
  ["deploy.production", "Run `deploy production` for release 2.0.2"],
  ["github.merge_pull_request", "Merge PR #343 into main"],
  ["messages.post", "Post the incident summary to #announcements"],
  ["github.create_issue", "Open 2 issues from today's bug bash"],
];

/** The approvals module. */
export interface Approvals {
  readonly routes: readonly Route[];
  /** Seeds every agent's requests into a fresh store. */
  seed(): void;
  /**
   * An agent asks: a new pending request, with its activity item. `roomId` `null` asks outside a
   * room.
   */
  request(agentId: number, action: string, summary: string, roomId: number | null): AgentApproval;
  /**
   * The inbox simulation asked (`agent_approval_request` draft): Ember's request with that summary,
   * recorded as its item.
   */
  requestFromInbox(draft: ActivityDraft): ActivityItem;
  /** Someone else decides (or the agent cancels): publishes `approval.updated`. */
  settle(approvalId: number, status: AgentApprovalStatus, deciderId: number | null): AgentApproval;
  start(): void;
  stop(): void;
}

/** Creates the approvals module; its simulation idles while `paused`. */
export function createApprovals(
  ctx: S2Context,
  agents: Agents,
  activity: Activity,
  random: Random,
  paused: () => boolean,
): Approvals {
  let records = new Map<number, ApprovalRecord>();
  let nextId = FIRST_LIVE_APPROVAL_ID;
  const timers = new Set<number>();
  let running = false;

  const recordOr404 = (id: number) => {
    const record = records.get(id);

    if (record === undefined) throw notFound();

    return record;
  };

  const agentOr404 = (agentId: number) => {
    const row = agents.record(agentId);

    if (row === null) throw notFound();

    return row;
  };

  /** A pending request past `expiresAt` reads expired (`effective_status`). */
  const effectiveStatus = (record: ApprovalRecord): AgentApprovalStatus =>
    record.status === "pending" && record.expiresAt <= ctx.now() ? "expired" : record.status;

  /** The viewer may decide: they manage the agent, and both they and its user are active. */
  const decides = (record: ApprovalRecord) => {
    const row = agents.record(record.agentId);
    const user = row === null ? undefined : ctx.world().users.get(row.userId);
    const viewer = ctx.world().users.get(VIEWER_ID);

    return (
      row !== null &&
      viewerManages(ctx, row) &&
      user?.status === "active" &&
      viewer?.status === "active" &&
      viewer.role !== "bot"
    );
  };

  const adminOnly = (action: string) => action.startsWith("github.") || action.startsWith("fizzy.");

  const present = (record: ApprovalRecord): AgentApproval => {
    const row = agents.record(record.agentId);
    // Who may decide, whatever the status (the contract): the card shows buttons only while pending.
    const may = decides(record);

    return {
      id: record.id,
      agentId: record.agentId,
      agentUserId: row?.userId ?? record.agentId,
      roomId: record.roomId,
      roomName: viewerRoomName(ctx, record.roomId),
      action: record.action,
      summary: record.summary,
      status: effectiveStatus(record),
      expiresAt: iso(record.expiresAt),
      createdAt: iso(record.createdAt),
      decidedById: record.decidedById,
      decidedAt: record.decidedAt === null ? null : iso(record.decidedAt),
      decisionNote: record.decisionNote,
      updatedAt: record.updatedAt,
      githubLogin: record.action.startsWith("github.") ? "ember-bot" : null,
      fizzyUserName: record.action.startsWith("fizzy.") ? "Ember (bot)" : null,
      adminOnly: adminOnly(record.action),
      approvable: may && (!adminOnly(record.action) || viewerIsAdmin(ctx)),
      deniable: may,
    };
  };

  const usersOf = (list: readonly ApprovalRecord[]) =>
    ctx.usersFor(
      list.flatMap((record) => {
        const row = agents.record(record.agentId);
        const agentUser = row === null ? [] : [row.userId];

        return record.decidedById === null ? agentUser : [...agentUser, record.decidedById];
      }),
    );

  const publishUpdated = (record: ApprovalRecord) => {
    record.updatedAt = touchedRow(ctx.now(), record.updatedAt);

    const data: ApprovalUpdated = { approval: present(record), users: usersOf([record]) };

    ctx.publish([{ topic: "user", type: "approval.updated", data }]);

    const status = effectiveStatus(record);

    activity.updateSourceWhere(
      (item) => item.source.sourceType === "agent_approval" && item.source.sourceId === record.id,
      (source) => ({ ...source, approvalStatus: status }),
    );
  };

  // --- seed ---

  const add = (seed: Omit<ApprovalRecord, "updatedAt">): ApprovalRecord => {
    const record = {
      ...seed,
      updatedAt: rowTimestamp(
        seed.decidedAt ?? (seed.status === "expired" ? seed.expiresAt : seed.createdAt),
      ),
    };

    records.set(record.id, record);

    return record;
  };

  const seed = () => {
    const now = ctx.now();

    records = new Map();
    nextId = FIRST_LIVE_APPROVAL_ID;

    APPROVALS.forEach(([summary, status], index) => {
      const at = seededApprovalAt(now, index);
      const [action, deciderId, note] = INBOX_DETAILS[index] ?? ["messages.post", null, null];
      const decided = status === "approved" || status === "denied";

      add({
        id: seededApprovalId(index),
        agentId: AGENT_IDS.ember,
        roomId: ROOM_IDS.engineering,
        action,
        summary,
        status,
        expiresAt: status === "pending" ? now + EXPIRES_AFTER_MS : at + EXPIRES_AFTER_MS,
        createdAt: at,
        decidedById: decided ? deciderId : null,
        decidedAt: decided ? at + 25 * MINUTE : null,
        decisionNote: decided ? note : null,
      });
    });

    // Ember's one request in a room the viewer isn't in, older than the inbox's seven.
    add({
      id: seededApprovalId(APPROVALS.length),
      agentId: AGENT_IDS.ember,
      roomId: HIDDEN_ROOM.id,
      action: "pager.acknowledge",
      summary: "Acknowledge the 02:00 disk alert on the primary database",
      status: "pending",
      expiresAt: now + EXPIRES_AFTER_MS,
      createdAt: seededApprovalAt(now, APPROVALS.length),
      decidedById: null,
      decidedAt: null,
      decisionNote: null,
    });

    for (let index = 0; index < HISTORY_SIZE; index++) {
      const [action, summary] = HISTORY[index % HISTORY.length] ?? ["messages.post", "Post"];
      const at = now - 6 * DAY - (HISTORY_SIZE - index) * 9 * HOUR;
      const status = historyStatus(index);
      const decided = status !== "expired";

      add({
        id: index + 1,
        agentId: AGENT_IDS.ember,
        roomId: index % 3 === 0 ? null : ROOM_IDS.engineering,
        action,
        summary,
        status,
        expiresAt: at + EXPIRES_AFTER_MS,
        createdAt: at,
        decidedById: decided ? (index % 2 === 0 ? USER_IDS.riel : USER_IDS.priya) : null,
        decidedAt: decided ? at + 40 * MINUTE : null,
        decisionNote: status === "denied" ? "Not this week." : null,
      });
    }

    const others: readonly Omit<ApprovalRecord, "updatedAt">[] = [
      {
        id: 201,
        agentId: AGENT_IDS.atlas,
        roomId: ROOM_IDS.engineering,
        action: "deploy.production",
        summary: "Deploy release 2.0.1 to production",
        status: "pending",
        expiresAt: now + EXPIRES_AFTER_MS,
        createdAt: now - 41 * MINUTE,
        decidedById: null,
        decidedAt: null,
        decisionNote: null,
      },
      {
        id: 301,
        agentId: AGENT_IDS.scout,
        roomId: ROOM_IDS.engineering,
        action: "fizzy.create_card",
        summary: "Add cards for the 4 dependency upgrades due this sprint",
        status: "pending",
        expiresAt: now + EXPIRES_AFTER_MS,
        createdAt: now - 3 * HOUR,
        decidedById: null,
        decidedAt: null,
        decisionNote: null,
      },
      {
        id: 302,
        agentId: AGENT_IDS.scout,
        roomId: null,
        action: "github.create_issue",
        summary: "Open an issue to bump the websocket library",
        status: "approved",
        expiresAt: now - DAY,
        createdAt: now - 3 * DAY,
        decidedById: USER_IDS.theo,
        decidedAt: now - 3 * DAY + HOUR,
        decisionNote: null,
      },
      {
        id: 401,
        agentId: AGENT_IDS.herald,
        roomId: ROOM_IDS.announcements,
        action: "messages.post",
        summary: "Post the v2.0.0 release notes to #announcements",
        status: "cancelled",
        expiresAt: now - 2 * DAY,
        createdAt: now - 4 * DAY,
        decidedById: null,
        decidedAt: null,
        decisionNote: null,
      },
    ];

    for (const record of others) add(record);
  };

  // --- reads ---

  const encodeCursor = (id: number) => btoa(`approval:${id}`).replace(/=+$/, "");

  const decodeCursor = (cursor: string): number => {
    let text: string;

    try {
      text = atob(cursor);
    } catch {
      throw validation("before", "is invalid");
    }

    const id = Number(text.replace(/^approval:/, ""));

    if (!text.startsWith("approval:") || !Number.isInteger(id)) {
      throw validation("before", "is invalid");
    }

    return id;
  };

  const list = (agentId: number, query: URLSearchParams): AgentApprovalPage => {
    const before = beforeOf(query);
    const after = before === null ? null : decodeCursor(before);
    const row = agentOr404(agentId);
    const user = ctx.world().users.get(row.userId);
    const viewer = ctx.world().users.get(VIEWER_ID);

    if (
      !viewerManages(ctx, row) ||
      user?.status !== "active" ||
      viewer?.status !== "active" ||
      viewer.role === "bot"
    ) {
      throw notFound();
    }

    const mine = [...records.values()].filter((record) => record.agentId === agentId);

    const status = query.get("status");
    const filter = STATUSES.find((candidate) => candidate === status) ?? null;

    const matching = mine
      .filter((record) => filter === null || effectiveStatus(record) === filter)
      .filter((record) => after === null || record.id < after)
      .sort((left, right) => right.id - left.id);

    const page = matching.slice(0, APPROVAL_PAGE_SIZE);
    const last = page.at(-1);

    // Listing settles only the overdue requests on the page, as the classic page does.
    for (const record of page) {
      if (record.status === "pending" && effectiveStatus(record) === "expired") {
        record.status = "expired";
        publishUpdated(record);
      }
    }

    return {
      approvals: page.map(present),
      users: usersOf(page),
      nextCursor:
        matching.length > APPROVAL_PAGE_SIZE && last !== undefined ? encodeCursor(last.id) : null,
    };
  };

  // --- writes ---

  const decisionOf = (body: Json | undefined): DecisionBody => {
    const decision = oneOf(stringField(body, "decision"), ["approved", "denied", ""], "");

    if (decision === "" || (field(body, "note") != null && stringField(body, "note") === null)) {
      throw validation("decision", "must be approved or denied");
    }

    const raw = stringField(body, "note") ?? "";
    const note = raw.trim() === "" ? null : raw;

    return { decision, note };
  };

  const decide = (approvalId: number, body: Json | undefined): AgentApproval => {
    const { decision, note } = decisionOf(body);
    const record = recordOr404(approvalId);

    if (!decides(record)) throw notFound();

    if (decision === "approved" && adminOnly(record.action) && !viewerIsAdmin(ctx)) {
      const service = record.action.startsWith("fizzy.") ? "Fizzy" : "GitHub";

      throw forbidden(`Only an administrator can approve ${service} write actions`);
    }

    if (record.status === "pending" && effectiveStatus(record) === "expired") {
      record.status = "expired";
      publishUpdated(record);
    }

    const status = effectiveStatus(record);

    if (status !== "pending") {
      throw sentence(
        "base",
        status === "expired" ? "Request has expired" : `Request is already ${status}`,
      );
    }

    if (note !== null && [...note].length > 200) {
      throw sentence("base", "Decision note is too long (maximum is 200 characters)");
    }

    record.status = decision;
    record.decidedById = VIEWER_ID;
    record.decidedAt = ctx.now();
    record.decisionNote = note;
    publishUpdated(record);

    return present(record);
  };

  const ask = (agentId: number, action: string, summary: string, roomId: number | null) => {
    const row = agentOr404(agentId);
    const now = ctx.now();

    const record = add({
      id: nextId++,
      agentId,
      roomId,
      action,
      summary,
      status: "pending",
      expiresAt: now + EXPIRES_AFTER_MS,
      createdAt: now,
      decidedById: null,
      decidedAt: null,
      decisionNote: null,
    });

    const name = ctx.world().users.get(row.userId)?.name ?? "Agent";

    const item = activity.record({
      eventType: "agent_approval_request",
      source: {
        sourceType: "agent_approval",
        sourceId: record.id,
        roomId,
        threadId: null,
        messageId: null,
        eventId: null,
        creatorId: row.userId,
        title: roomId === null ? name : conversationTitle(ctx, roomId, null, name),
        body: summary,
        occurredAt: iso(now),
        approvalStatus: "pending",
        budgetCap: null,
        path: `/agents/${agentId}/approvals`,
      },
    });

    return { approval: present(record), item };
  };

  const request = (agentId: number, action: string, summary: string, roomId: number | null) =>
    ask(agentId, action, summary, roomId).approval;

  const settle = (
    approvalId: number,
    status: AgentApprovalStatus,
    deciderId: number | null,
  ): AgentApproval => {
    const record = recordOr404(approvalId);

    record.status = status;
    record.decidedById = status === "approved" || status === "denied" ? deciderId : null;
    record.decidedAt = record.decidedById === null ? null : ctx.now();
    publishUpdated(record);

    return present(record);
  };

  // --- simulation ---

  const later = (delayMs: number, run: () => void) => {
    const id = ctx.scheduler.schedule(delayMs, () => {
      timers.delete(id);
      run();
    });

    timers.add(id);
  };

  const tick = () => {
    if (!running) return;

    if (!paused()) {
      const pending = [...records.values()].filter(
        (record) => record.agentId === AGENT_IDS.ember && effectiveStatus(record) === "pending",
      );

      if (pending.length > 1 && random.int(0, 2) === 0) {
        const oldest = pending.reduce((left, right) => (left.id < right.id ? left : right));

        settle(oldest.id, random.int(0, 3) === 0 ? "denied" : "approved", USER_IDS.priya);
      } else {
        const [action, summary] = random.pick(LIVE_REQUESTS);

        request(AGENT_IDS.ember, action, summary, ROOM_IDS.engineering);
      }
    }

    later(random.int(45_000, 90_000), tick);
  };

  return {
    routes: [
      route("GET", /^\/agents\/(\d+)\/approvals$/, (request) =>
        ok(list(firstId(request), request.query)),
      ),
      route("PATCH", /^\/agent_approvals\/(\d+)$/, (request) =>
        ok(decide(firstId(request), request.body)),
      ),
    ],
    seed,
    request,
    requestFromInbox: (draft) =>
      ask(AGENT_IDS.ember, "messages.post", draft.source.body, ROOM_IDS.engineering).item,
    settle,
    start() {
      if (running) return;

      running = true;
      later(random.int(45_000, 90_000), tick);
    },
    stop() {
      running = false;

      for (const id of timers) ctx.scheduler.cancel(id);

      timers.clear();
    },
  };
}
