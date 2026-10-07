/**
 * Agents (S4): the directory and profiles, live status (`agent.status`) and the steps on agent
 * messages (`agent.steps`). The seed adds four agents beside Ember (a few kinds and statuses: one
 * working with its own icon, one suspended, one with no owner, one waiting), and steps on Ember's
 * replies in its direct message. A small simulation has Ember work through a task now and then,
 * and the `/__mock/agent-*` controls drive the same paths from tests.
 *
 * The mock reuses each bot user's id as its agent id, as the S2 seed does for Ember.
 */
import type { AgentBudgetUsage } from "../../src/gen/AgentBudgetUsage.ts";
import type { AgentDirectory } from "../../src/gen/AgentDirectory.ts";
import type { AgentDirectoryRow } from "../../src/gen/AgentDirectoryRow.ts";
import type { AgentGrants } from "../../src/gen/AgentGrants.ts";
import type { AgentKind } from "../../src/gen/AgentKind.ts";
import type { AgentProfile } from "../../src/gen/AgentProfile.ts";
import type { AgentStatus } from "../../src/gen/AgentStatus.ts";
import type { AgentStatusChanged } from "../../src/gen/AgentStatusChanged.ts";
import type { AgentStep } from "../../src/gen/AgentStep.ts";
import type { AgentStepStatus } from "../../src/gen/AgentStepStatus.ts";
import type { Icon } from "../../src/gen/Icon.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { User } from "../../src/gen/User.ts";
import { notFound, ok } from "../http.ts";
import type { Random } from "../random.ts";
import { firstId, type Route, route, type S2Context } from "../s2/context.ts";
import { iso } from "../s2/model.ts";
import { BOT_ID, ROOM_IDS, USER_IDS, VIEWER_ID } from "../seed.ts";
import { validation } from "./http.ts";
import { workStateOf } from "./work-state.ts";

const MINUTE = 60_000;

const HOUR = 60 * MINUTE;

const DAY = 24 * HOUR;

/** How long a working presence lasts (`assign_working_presence`). */
export const WORKING_PRESENCE_MS = 5 * MINUTE;

/** The seeded agents' ids (each is its bot user's id too). */
export const AGENT_IDS = {
  ember: BOT_ID,
  scout: 41,
  herald: 42,
  quill: 43,
  atlas: 44,
} as const;

/** Whether the viewer is an administrator. */
export function viewerIsAdmin(ctx: S2Context): boolean {
  return ctx.world().users.get(VIEWER_ID)?.role === "administrator";
}

/**
 * Whether the viewer manages the agent (an administrator, or its owner): who sees its grants and
 * budgets, decides its approval requests and reads its ledger.
 */
export function viewerManages(ctx: S2Context, row: AgentDirectoryRow): boolean {
  return viewerIsAdmin(ctx) || row.ownerId === VIEWER_ID;
}

/**
 * A room no one in the seed belongs to (the viewer included), named only to administrators: the
 * approvals and ledger entries there show "a room you're not in" to an owner who isn't an
 * administrator (see the `viewer-role` control).
 */
export const HIDDEN_ROOM = { id: 61, name: "ops-oncall" } as const;

/**
 * A room's name as the viewer may see it (`roomName` on approvals and ledger entries): `null`
 * unless the viewer is an administrator or a member of the room.
 */
export function viewerRoomName(ctx: S2Context, roomId: number | null): string | null {
  if (roomId === null) return null;

  const record = ctx.world().rooms.get(roomId);
  const member = record?.memberIds.includes(VIEWER_ID) ?? false;

  if (!member && !viewerIsAdmin(ctx)) return null;

  if (record !== undefined) return ctx.displayName(record);

  return roomId === HIDDEN_ROOM.id ? HIDDEN_ROOM.name : null;
}

/** The seeded step ids start here. */
const FIRST_STEP_ID = 9001;

interface AgentSeed {
  readonly id: number;
  readonly name: string;
  readonly kind: AgentKind;
  readonly ownerId: number | null;
  readonly status: AgentStatus;
  readonly statusNote: string | null;
  readonly suspended: boolean;
  readonly avatarIcon: Icon | null;
  readonly provider: string | null;
  readonly runtime: string | null;
  readonly description: string | null;
  readonly bio: string;
  readonly roomIds: readonly number[];
  readonly hiddenRoomCount: number;
  readonly grants: AgentGrants;
  readonly budget: readonly AgentBudgetUsage[];
  readonly activity: readonly [number, number, number, number];
  readonly createdDaysAgo: number;
  /** `null`: never seen. */
  readonly lastSeenMinutesAgo: number | null;
  /** `null`: the status never changed. */
  readonly statusChangedMinutesAgo: number | null;
  readonly presence: string | null;
}

const SEEDS: readonly AgentSeed[] = [
  {
    id: AGENT_IDS.ember,
    name: "Ember",
    kind: "workspace",
    ownerId: VIEWER_ID,
    status: "idle",
    statusNote: null,
    suspended: false,
    avatarIcon: null,
    provider: "Anthropic",
    runtime: "claude-agent-sdk",
    description:
      "The team's AI agent. Summarises rooms, files cards and answers questions; mention @Ember anywhere or send it a direct message.",
    bio: "The team's AI agent. Mention @Ember in any room or send a direct message.",
    roomIds: [ROOM_IDS.general, ROOM_IDS.engineering, ROOM_IDS.design, ROOM_IDS.dmEmber],
    hiddenRoomCount: 2,
    grants: {
      legacy: false,
      grants: [
        { capability: "fizzy", workspaceWide: false, roomCount: 2 },
        { capability: "post_messages", workspaceWide: true, roomCount: 0 },
        { capability: "react", workspaceWide: true, roomCount: 0 },
        { capability: "read_messages", workspaceWide: true, roomCount: 0 },
      ],
    },
    budget: [
      { cap: "messages", used: 38, limit: 200 },
      { cap: "board_posts", used: 4, limit: 5 },
      { cap: "external_actions", used: 3, limit: null },
    ],
    activity: [64, 51, 38, 3],
    createdDaysAgo: 120,
    lastSeenMinutesAgo: 2,
    statusChangedMinutesAgo: 14,
    presence: null,
  },
  {
    id: AGENT_IDS.scout,
    name: "Scout",
    kind: "personal",
    ownerId: USER_IDS.theo,
    status: "working",
    statusNote: null,
    suspended: false,
    avatarIcon: {
      name: "telescope",
      title: "Telescope",
      kind: "emoji",
      character: "\u{1F52D}",
      imageUrl: null,
    },
    provider: "OpenAI",
    runtime: "codex",
    description: "Theo's research agent: triages new issues and reads up on dependencies.",
    bio: "Theo's research agent.",
    roomIds: [ROOM_IDS.engineering],
    hiddenRoomCount: 1,
    grants: {
      legacy: false,
      grants: [
        { capability: "external_action", workspaceWide: false, roomCount: 1 },
        { capability: "read_messages", workspaceWide: false, roomCount: 2 },
      ],
    },
    budget: [
      { cap: "messages", used: 12, limit: 50 },
      { cap: "board_posts", used: 0, limit: null },
      { cap: "external_actions", used: 9, limit: 10 },
    ],
    activity: [22, 22, 6, 0],
    createdDaysAgo: 30,
    lastSeenMinutesAgo: 0,
    statusChangedMinutesAgo: 3,
    presence: "Triaging new issues on smartfire",
  },
  {
    id: AGENT_IDS.herald,
    name: "Herald",
    kind: "workspace",
    ownerId: null,
    status: "failed",
    statusNote: "GitHub token expired; can't read releases",
    suspended: false,
    avatarIcon: null,
    provider: null,
    runtime: "webhook",
    description: "Posts release notes to #announcements when a tag is pushed.",
    bio: "Release notes, on every tag.",
    roomIds: [ROOM_IDS.announcements],
    hiddenRoomCount: 0,
    grants: { legacy: true, grants: [] },
    budget: [
      { cap: "messages", used: 0, limit: 20 },
      { cap: "board_posts", used: 0, limit: null },
      { cap: "external_actions", used: 0, limit: null },
    ],
    activity: [3, 0, 0, 3],
    createdDaysAgo: 210,
    lastSeenMinutesAgo: 3 * 24 * 60,
    statusChangedMinutesAgo: 26 * 60,
    presence: null,
  },
  {
    id: AGENT_IDS.quill,
    name: "Quill",
    kind: "personal",
    ownerId: USER_IDS.maya,
    status: "idle",
    statusNote: null,
    suspended: true,
    avatarIcon: null,
    provider: "Anthropic",
    runtime: null,
    description: "Maya's writing assistant, paused while its grants are reviewed.",
    bio: "Maya's writing assistant.",
    roomIds: [ROOM_IDS.design],
    hiddenRoomCount: 0,
    grants: { legacy: false, grants: [] },
    budget: [
      { cap: "messages", used: 0, limit: 40 },
      { cap: "board_posts", used: 0, limit: 5 },
      { cap: "external_actions", used: 0, limit: 0 },
    ],
    activity: [0, 0, 0, 7],
    createdDaysAgo: 60,
    lastSeenMinutesAgo: null,
    statusChangedMinutesAgo: null,
    presence: null,
  },
  {
    id: AGENT_IDS.atlas,
    name: "Atlas",
    kind: "workspace",
    ownerId: USER_IDS.priya,
    status: "waiting",
    statusNote: "Waiting for approval to deploy to production",
    suspended: false,
    avatarIcon: null,
    provider: "Anthropic",
    runtime: "claude-agent-sdk",
    description: "Runs the release checklist and deploys once someone approves.",
    bio: "The release runner.",
    roomIds: [ROOM_IDS.engineering, ROOM_IDS.launchPlanning],
    hiddenRoomCount: 0,
    grants: {
      legacy: false,
      grants: [
        { capability: "external_action", workspaceWide: true, roomCount: 0 },
        { capability: "manage_threads", workspaceWide: false, roomCount: 1 },
        { capability: "post_messages", workspaceWide: false, roomCount: 2 },
      ],
    },
    budget: [
      { cap: "messages", used: 9, limit: 100 },
      { cap: "board_posts", used: 1, limit: 10 },
      { cap: "external_actions", used: 2, limit: 5 },
    ],
    activity: [17, 15, 9, 1],
    createdDaysAgo: 90,
    lastSeenMinutesAgo: 6,
    statusChangedMinutesAgo: 41,
    presence: null,
  },
];

/** One agent as the mock holds it. */
interface AgentRecord {
  readonly seed: AgentSeed;
  row: AgentDirectoryRow;
  presence: { readonly text: string; readonly expiresAt: string } | null;
}

/** A status change the controls (and the simulation) can ask for. */
export interface StatusChange {
  /** Absent (or `undefined`) keeps the field as it is. */
  readonly status?: AgentStatus | undefined;
  readonly statusNote?: string | null | undefined;
  readonly suspended?: boolean | undefined;
  /** What it's doing now; `null` clears it, absent keeps it. */
  readonly presence?: string | null | undefined;
}

const STATUSES: readonly AgentStatus[] = ["idle", "working", "waiting", "failed"];

/**
 * The `/__mock/agent-status` control's fields as a change: `null` keeps a field, an empty
 * `presence` clears it. An unknown status is a 422.
 */
export function statusControl(
  status: string | null,
  suspended: boolean | null,
  presence: string | null,
): StatusChange {
  const known = STATUSES.find((candidate) => candidate === status);

  if (status !== null && known === undefined) {
    throw validation("status", "must be idle, working, waiting or failed");
  }

  return {
    status: known,
    suspended: suspended ?? undefined,
    presence: presence === "" ? null : (presence ?? undefined),
  };
}

/** The plan the steps control and simulation walk through. */
const PLAN: readonly { readonly name: string; readonly input: string; readonly output: string }[] =
  [
    {
      name: "Read #engineering since yesterday",
      input: "#engineering, the last 24 hours",
      output: "212 messages in 9 threads",
    },
    {
      name: "Run the test suite on main",
      input: "pnpm check",
      output: "84 files, 721 tests passed",
    },
    {
      name: "Draft the summary",
      input: "3 decisions, 2 open questions",
      output: "A 6-line summary with links",
    },
  ];

/** The agents module. */
export interface Agents {
  readonly routes: readonly Route[];
  /** Seeds the agents (and their bot users and message steps) into the current world. */
  seed(): void;
  /** Changes an agent's status and publishes `agent.status`. */
  setStatus(agentId: number, change: StatusChange): AgentStatusChanged;
  /**
   * Walks a message's three-step plan to `stage` (0: the first step running, 1: the second, 2:
   * the third, 3: all done) and publishes `agent.steps`.
   */
  setSteps(messageId: number, stage: number): readonly AgentStep[];
  /** The newest message an agent wrote in a room, for the steps control. */
  latestBy(roomId: number, userId: number): MessageDTO | null;
  /** Starts the simulation (Ember works through a task now and then). */
  start(): void;
  stop(): void;
  /** The agent records, for the approvals and ledger modules. */
  record(agentId: number): AgentDirectoryRow | null;
}

/** Creates the agents module over the server's context; its simulation idles while `paused`. */
export function createAgents(ctx: S2Context, random: Random, paused: () => boolean): Agents {
  let records = new Map<number, AgentRecord>();
  let nextStepId = FIRST_STEP_ID;
  const timers = new Set<number>();
  let running = false;

  const later = (delayMs: number, run: () => void) => {
    const id = ctx.scheduler.schedule(delayMs, () => {
      timers.delete(id);
      run();
    });

    timers.add(id);
  };

  const recordOr404 = (agentId: number): AgentRecord => {
    const record = records.get(agentId);

    if (record === undefined) throw notFound();

    return record;
  };

  // --- seed ---

  const seedUser = (seed: AgentSeed, now: number): User => {
    const held = ctx.world().users.get(seed.id);

    return {
      id: seed.id,
      name: seed.name,
      role: "bot",
      status: "active",
      bio: held?.bio ?? seed.bio,
      avatarUrl: held?.avatarUrl ?? `/users/${seed.id}/avatar`,
      hasAvatar: held?.hasAvatar ?? false,
      customStatus: null,
      avatarIcon: seed.avatarIcon,
      agent: { agentId: seed.id, kind: seed.kind, status: seed.status, suspended: seed.suspended },
      createdAt: held?.createdAt ?? iso(now - seed.createdDaysAgo * DAY),
    };
  };

  const stepOf = (
    messageId: number,
    position: number,
    name: string,
    status: AgentStepStatus,
    at: number,
    durationMs: number | null,
    summaries: readonly [string | null, string | null],
  ): AgentStep => ({
    id: nextStepId++,
    messageId,
    threadId: null,
    name,
    status,
    inputSummary: summaries[0],
    outputSummary: summaries[1],
    durationMs,
    position,
    createdAt: iso(at),
    updatedAt: iso(at + (durationMs ?? 0)),
  });

  /** Steps on Ember's replies in its DM: a clean run, a short one, and one with a failed step. */
  const seedSteps = () => {
    const room = ctx.world().rooms.get(ROOM_IDS.dmEmber);

    if (room === undefined) return;

    const replies = room.messages.filter((message) => message.creatorId === BOT_ID);

    const plans: readonly (readonly [
      string,
      AgentStepStatus,
      number,
      string | null,
      string | null,
    ])[][] = [
      [
        [
          "Read #engineering since yesterday",
          "done",
          1840,
          "#engineering, the last 24 hours",
          "212 messages in 9 threads",
        ],
        ["Group the threads by topic", "done", 920, null, "3 topics"],
        ["Draft the summary", "done", 2410, null, "A 3-item summary"],
      ],
      [
        [
          "Create the card on Fizzy",
          "done",
          610,
          "Safari websocket close codes",
          "Card #418 on Engineering",
        ],
        ["Assign it to Jonah", "done", 180, null, null],
      ],
      [
        ["Fetch today's events", "done", 450, "Riel's calendar, 12:00 to 18:00", "2 events"],
        [
          "Look up the room bookings",
          "failed",
          3020,
          "Design review, Architecture review",
          "The calendar API answered 403 for room resources",
        ],
        ["Compose the answer", "done", 640, null, null],
      ],
    ];

    plans.forEach((plan, index) => {
      const message = replies[index];

      if (message === undefined) return;

      const start = Date.parse(message.createdAt) - 8000;
      let at = start;

      const steps = plan.map(([name, status, duration, input, output], position) => {
        const step = stepOf(message.id, position, name, status, at, duration, [input, output]);

        at += duration;

        return step;
      });

      room.messages[room.messages.indexOf(message)] = { ...message, steps };
    });
  };

  const seed = () => {
    // Ember cannot manage threads in announcements. Existing work rooms keep their capabilities.
    workStateOf(ctx.world()).agentCapabilities.set(
      `${ROOM_IDS.announcements}:${AGENT_IDS.ember}`,
      new Set(["post_messages", "read_messages"]),
    );

    const now = ctx.now();
    const world = ctx.world();

    records = new Map();
    nextStepId = FIRST_STEP_ID;

    for (const each of SEEDS) {
      world.users.set(each.id, seedUser(each, now));

      const minutesAgo = (minutes: number | null) =>
        minutes === null ? null : iso(now - minutes * MINUTE);

      records.set(each.id, {
        seed: each,
        row: {
          agentId: each.id,
          userId: each.id,
          kind: each.kind,
          ownerId: each.ownerId,
          status: each.status,
          statusNote: each.statusNote,
          suspended: each.suspended,
          createdAt: iso(now - each.createdDaysAgo * DAY),
          statusChangedAt: minutesAgo(each.statusChangedMinutesAgo),
          lastSeenAt: minutesAgo(each.lastSeenMinutesAgo),
        },
        presence:
          each.presence === null
            ? null
            : { text: each.presence, expiresAt: iso(now + WORKING_PRESENCE_MS) },
      });
    }

    seedSteps();
  };

  // --- reads ---

  const isActive = (record: AgentRecord) =>
    !record.row.suspended && ctx.world().users.get(record.row.userId)?.status === "active";

  const directory = (): AgentDirectory => {
    const name = (record: AgentRecord) =>
      (ctx.world().users.get(record.row.userId)?.name ?? "").toLowerCase();

    const listed = [...records.values()]
      .filter((record) => ctx.world().users.get(record.row.userId)?.status !== "deactivated")
      .sort(
        (left, right) =>
          Number(!isActive(left)) - Number(!isActive(right)) ||
          name(left).localeCompare(name(right)) ||
          left.row.agentId - right.row.agentId,
      );

    return {
      agents: listed.map((record) => record.row),
      users: ctx.usersFor(
        listed.flatMap((record) =>
          record.row.ownerId === null
            ? [record.row.userId]
            : [record.row.userId, record.row.ownerId],
        ),
      ),
    };
  };

  /** Administrators and the owner see grants and management. */
  const manages = (record: AgentRecord) => viewerManages(ctx, record.row);

  const profile = (agentId: number): AgentProfile => {
    const record = recordOr404(agentId);
    const { seed: each } = record;

    const visible = each.roomIds.flatMap((roomId) => {
      try {
        const room = ctx.roomOr404(roomId);

        return [{ roomId, name: ctx.displayName(room) }];
      } catch {
        return [];
      }
    });

    const rooms = visible.sort((left, right) =>
      left.name.toLowerCase().localeCompare(right.name.toLowerCase()),
    );

    const [delivered, acknowledged, posted, suppressed] = each.activity;

    return {
      agent: record.row,
      provider: each.provider,
      runtime: each.runtime,
      description: each.description,
      rooms,
      hiddenRoomCount: each.hiddenRoomCount + (each.roomIds.length - visible.length),
      grants: manages(record) ? each.grants : null,
      management: manages(record)
        ? {
            activitySummary: { delivered, acknowledged, posted, suppressed },
            budgetUsage: [...each.budget],
          }
        : null,
      users: ctx.usersFor(
        record.row.ownerId === null ? [record.row.userId] : [record.row.userId, record.row.ownerId],
      ),
    };
  };

  // --- live ---

  const setStatus = (agentId: number, change: StatusChange): AgentStatusChanged => {
    const record = recordOr404(agentId);
    const now = ctx.now();
    const status = change.status ?? record.row.status;
    const statusNote = change.statusNote === undefined ? record.row.statusNote : change.statusNote;
    const statusMoved = status !== record.row.status || statusNote !== record.row.statusNote;

    record.row = {
      ...record.row,
      status,
      statusNote,
      suspended: change.suspended ?? record.row.suspended,
      statusChangedAt: statusMoved ? iso(now) : record.row.statusChangedAt,
      lastSeenAt: iso(now),
    };

    if (change.presence !== undefined) {
      record.presence =
        change.presence === null
          ? null
          : { text: change.presence, expiresAt: iso(now + WORKING_PRESENCE_MS) };
    }

    const user = ctx.world().users.get(record.row.userId);

    if (user !== undefined) {
      ctx.world().users.set(user.id, {
        ...user,
        agent: {
          agentId,
          kind: record.row.kind,
          status: record.row.status,
          suspended: record.row.suspended,
        },
      });
    }

    const live = record.presence !== null && Date.parse(record.presence.expiresAt) > now;

    const event: AgentStatusChanged = {
      agentId,
      userId: record.row.userId,
      status: record.row.status,
      statusNote: record.row.statusNote,
      statusChangedAt: record.row.statusChangedAt,
      suspended: record.row.suspended,
      workingPresence: live ? (record.presence?.text ?? null) : null,
      workingPresenceExpiresAt: live ? (record.presence?.expiresAt ?? null) : null,
    };

    ctx.publish([{ topic: "user", type: "agent.status", data: event }]);

    return event;
  };

  const locateMessage = (messageId: number) => {
    for (const room of ctx.world().rooms.values()) {
      const index = room.messages.findIndex((message) => message.id === messageId);

      if (index >= 0) return { room, index };
    }

    return null;
  };

  const setSteps = (messageId: number, stage: number): readonly AgentStep[] => {
    const found = locateMessage(messageId);

    if (found === null) throw notFound();

    if (!Number.isInteger(stage) || stage < 0 || stage > PLAN.length) {
      throw validation("stage", `must be 0 to ${PLAN.length}`);
    }

    const { room, index } = found;
    const message = room.messages[index];

    if (message === undefined) throw notFound();

    const now = ctx.now();
    const held = new Map(message.steps.map((step) => [step.name, step]));

    const steps = PLAN.map((part, position): AgentStep => {
      const status: AgentStepStatus =
        position < stage ? "done" : position === stage ? "running" : "pending";

      const previous = held.get(part.name);

      if (previous !== undefined && previous.status === status) return previous;

      return {
        id: previous?.id ?? nextStepId++,
        messageId,
        threadId: null,
        name: part.name,
        status,
        inputSummary: status === "pending" ? null : part.input,
        outputSummary: status === "done" ? part.output : null,
        durationMs: status === "done" ? 900 + position * 650 : null,
        position,
        createdAt: previous?.createdAt ?? iso(now),
        updatedAt: iso(
          Math.max(now, previous === undefined ? 0 : Date.parse(previous.updatedAt) + 1),
        ),
      };
    });

    room.messages[index] = { ...message, steps };
    ctx.publish([
      {
        topic: `room:${room.room.id}`,
        type: "agent.steps",
        data: { roomId: room.room.id, messageId, threadId: message.threadId, steps },
      },
    ]);

    return steps;
  };

  const latestBy = (roomId: number, userId: number): MessageDTO | null =>
    ctx
      .world()
      .rooms.get(roomId)
      ?.messages.findLast((message) => message.creatorId === userId) ?? null;

  // --- simulation ---

  /** Ember works through the plan on its latest DM reply, then goes idle again. */
  const task = () => {
    const message = latestBy(ROOM_IDS.dmEmber, BOT_ID);

    if (message === null) return;

    setStatus(BOT_ID, { status: "working", presence: "Summarising #engineering" });

    PLAN.forEach((_part, stage) => {
      later(1500 + stage * 2500, () => setSteps(message.id, stage));
    });

    later(1500 + PLAN.length * 2500, () => {
      setSteps(message.id, PLAN.length);
      setStatus(BOT_ID, { status: "idle", presence: null });
    });
  };

  const tick = () => {
    if (!running) return;

    if (paused()) {
      later(random.int(25_000, 45_000), tick);

      return;
    }

    if (random.chance(0.5)) {
      task();
    } else {
      const scout = records.get(AGENT_IDS.scout);
      const working = scout?.row.status === "working";

      setStatus(AGENT_IDS.scout, {
        status: working ? "idle" : "working",
        presence: working ? null : "Reading the dependency changelog",
      });
    }

    later(random.int(25_000, 45_000), tick);
  };

  return {
    routes: [
      route("GET", /^\/agents$/, () => ok(directory())),
      route("GET", /^\/agents\/(\d+)$/, (request) => ok(profile(firstId(request)))),
    ],
    seed,
    setStatus,
    setSteps,
    latestBy,
    start() {
      if (running) return;

      running = true;
      later(random.int(8000, 15_000), tick);
    },
    stop() {
      running = false;

      for (const id of timers) ctx.scheduler.cancel(id);

      timers.clear();
    },
    record: (agentId) => records.get(agentId)?.row ?? null,
  };
}
