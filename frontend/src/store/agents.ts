/**
 * Agents in the store (S4): the directory, the profiles opened so far, what each agent says it's
 * doing now (working presence), and the steps on agent messages. Pure reducers and selectors;
 * `store.ts` wraps the reducers in `mutations`.
 *
 * `agent.status` is the one live source: it updates the agent's badge on its user
 * (`User.agent`), its directory row, its profile and its working presence, in one change. Working
 * presence lapses at `expiresAt` with no event, so readers pass the time and the hooks re-render
 * when it's up.
 *
 * Steps merge per step by `updatedAt` (on a tie the later arrival wins), never by the message's
 * own `updatedAt`, since a step change doesn't bump it (see `AgentStep` in the contract).
 */
import type { AgentDirectory } from "../gen/AgentDirectory.ts";
import type { AgentDirectoryRow } from "../gen/AgentDirectoryRow.ts";
import type { AgentProfile } from "../gen/AgentProfile.ts";
import type { AgentStatusChanged } from "../gen/AgentStatusChanged.ts";
import type { AgentStep } from "../gen/AgentStep.ts";
import type { AgentStepsChanged } from "../gen/AgentStepsChanged.ts";
import { land, receive } from "./freshness.ts";
import type { LoadStatus, MessageDTO, User } from "./model.ts";
import { mergeUserList } from "./ordering.ts";
import type { State } from "./state.ts";

/** What an agent says it's doing now, until `expiresAt`. */
export interface WorkingPresence {
  readonly text: string;
  readonly expiresAt: string;
}

/** The directory's load state; its rows live in `AgentsSlice.rows`. */
export interface AgentDirectoryList {
  /** Agent ids in the directory's order: active agents first, then by lower-cased name. */
  readonly ids: readonly number[];
  readonly status: LoadStatus;
  readonly error: string | null;
  /** Bumped by every load; a reply lands only in the generation it started in. */
  readonly generation: number;
}

/** One agent's profile as loaded. */
export interface AgentProfileEntry {
  readonly status: LoadStatus;
  readonly profile: AgentProfile | null;
  readonly error: string | null;
  /** The server has no such agent (404): nothing to retry. */
  readonly missing: boolean;
  readonly generation: number;
}

export interface AgentsSlice {
  readonly directory: AgentDirectoryList;
  /** Every directory row seen (the directory's and the profiles'), by agent id. */
  readonly rows: Readonly<Record<number, AgentDirectoryRow>>;
  readonly profiles: Readonly<Record<number, AgentProfileEntry>>;
  /** Working presence by agent id; absent when unset. Check `expiresAt` when reading. */
  readonly working: Readonly<Record<number, WorkingPresence>>;
  /** Retained even before the first GET, since status timestamps do not cover presence changes. */
  readonly live: Readonly<
    Record<number, { readonly version: number; readonly change: AgentStatusChanged }>
  >;
}

export const emptyAgents: AgentsSlice = {
  directory: { ids: [], status: "idle", error: null, generation: 0 },
  rows: {},
  profiles: {},
  working: {},
  live: {},
};

const emptyProfile: AgentProfileEntry = {
  status: "idle",
  profile: null,
  error: null,
  missing: false,
  generation: 0,
};

function withAgents(state: State, agents: AgentsSlice): State {
  return agents === state.agents ? state : { ...state, agents };
}

// --- directory ---

/** Active (not suspended, user active) first, then by lower-cased name, as the server orders. */
export function directoryOrder(
  ids: readonly number[],
  rows: Readonly<Record<number, AgentDirectoryRow>>,
  users: Readonly<Record<number, User>>,
): readonly number[] {
  const rank = (agentId: number) => {
    const row = rows[agentId];
    const user = row === undefined ? undefined : users[row.userId];

    return row !== undefined && !row.suspended && user?.status === "active" ? 0 : 1;
  };

  const name = (agentId: number) => {
    const row = rows[agentId];

    return (row === undefined ? "" : (users[row.userId]?.name ?? "")).toLowerCase();
  };

  const sorted = [...ids].sort(
    (left, right) =>
      rank(left) - rank(right) || name(left).localeCompare(name(right)) || left - right,
  );

  return sorted.every((id, index) => ids[index] === id) ? ids : sorted;
}

/** The directory is loading (a first load, or a reload that keeps the rows shown meanwhile). */
export function setDirectoryLoading(state: State): State {
  const { directory } = state.agents;

  return withAgents(state, {
    ...state.agents,
    directory: {
      ...directory,
      status: directory.status === "ready" ? "ready" : "loading",
      error: null,
      generation: directory.generation + 1,
    },
  });
}

/** The directory's generation now, for a load to land in. */
export function directoryGeneration(state: State): number {
  return state.agents.directory.generation;
}

/** The directory landed, unless a newer load started since. */
export function landDirectory(
  state: State,
  page: AgentDirectory,
  generation: number,
  sentLive: AgentsSlice["live"] = state.agents.live,
): State {
  if (generation !== state.agents.directory.generation) {
    return state;
  }

  const rows = { ...state.agents.rows };

  for (const row of page.agents) {
    rows[row.agentId] = keepLiveStatus(state, row, sentLive);
  }

  const users = mergeUserList(state.users, withAgentBadges(page.users, rows));

  return {
    ...state,
    users,
    agents: {
      ...state.agents,
      rows,
      directory: {
        ids: directoryOrder(
          page.agents.map((row) => row.agentId),
          rows,
          users,
        ),
        status: "ready",
        error: null,
        generation,
      },
    },
  };
}

/** The directory failed to load: a shown one keeps its rows. */
export function setDirectoryFailed(state: State, error: string, generation: number): State {
  const { directory } = state.agents;

  if (generation !== directory.generation) {
    return state;
  }

  return withAgents(state, {
    ...state.agents,
    directory: { ...directory, status: directory.status === "ready" ? "ready" : "error", error },
  });
}

// --- profiles ---

export function profileOf(state: State, agentId: number): AgentProfileEntry {
  return state.agents.profiles[agentId] ?? emptyProfile;
}

function withProfile(state: State, agentId: number, entry: AgentProfileEntry): State {
  return withAgents(state, {
    ...state.agents,
    profiles: { ...state.agents.profiles, [agentId]: entry },
  });
}

/** The profile is loading; a shown one stays while it reloads. */
export function setProfileLoading(state: State, agentId: number): State {
  const entry = profileOf(state, agentId);

  return withProfile(state, agentId, {
    ...entry,
    status: entry.status === "ready" ? "ready" : "loading",
    error: null,
    generation: entry.generation + 1,
  });
}

/** The profile landed: its row and users join the store too. */
export function landProfile(
  state: State,
  profile: AgentProfile,
  generation: number,
  sentLive: AgentsSlice["live"] = state.agents.live,
): State {
  const agentId = profile.agent.agentId;

  if (generation !== profileOf(state, agentId).generation) {
    return state;
  }

  const agent = keepLiveStatus(state, profile.agent, sentLive);
  const rows = { ...state.agents.rows, [agentId]: agent };
  const users = withAgentBadges(profile.users, rows);

  return {
    ...state,
    users: mergeUserList(state.users, users),
    agents: {
      ...state.agents,
      rows,
      profiles: {
        ...state.agents.profiles,
        [agentId]: {
          status: "ready",
          profile: { ...profile, agent, users },
          error: null,
          missing: false,
          generation,
        },
      },
    },
  };
}

/** The profile failed to load; `missing` when the agent doesn't exist (404). */
export function setProfileFailed(
  state: State,
  agentId: number,
  error: string,
  missing: boolean,
  generation: number,
): State {
  const entry = profileOf(state, agentId);

  if (generation !== entry.generation) {
    return state;
  }

  return withProfile(state, agentId, {
    ...entry,
    status: entry.status === "ready" && !missing ? "ready" : "error",
    profile: missing ? null : entry.profile,
    error,
    missing,
  });
}

// --- agent.status ---

function withStatus(row: AgentDirectoryRow, change: AgentStatusChanged): AgentDirectoryRow {
  return {
    ...row,
    status: change.status,
    statusNote: change.statusNote,
    statusChangedAt: change.statusChangedAt,
    suspended: change.suspended,
  };
}

function keepLiveStatus(
  state: State,
  row: AgentDirectoryRow,
  sentLive: AgentsSlice["live"],
): AgentDirectoryRow {
  const live = state.agents.live[row.agentId];

  return live !== undefined &&
    (live.version !== sentLive[row.agentId]?.version ||
      (live.change.statusChangedAt ?? "") > (row.statusChangedAt ?? ""))
    ? withStatus(row, live.change)
    : row;
}

function withAgentBadges(users: readonly User[], rows: AgentsSlice["rows"]): User[] {
  return users.map((user) => {
    const row = user.agent === null ? undefined : rows[user.agent.agentId];

    return row === undefined || user.agent === null
      ? user
      : { ...user, agent: { ...user.agent, status: row.status, suspended: row.suspended } };
  });
}

/**
 * `agent.status`: the badge on the agent's user, its directory row (which may move, when a
 * suspension changes its rank), its profile and its working presence.
 */
export function applyAgentStatus(state: State, change: AgentStatusChanged): State {
  const { agentId, userId } = change;
  const previous = state.agents.live[agentId];

  const incomingAt = change.statusChangedAt ?? "";

  if (
    (previous?.change.statusChangedAt ?? "") > incomingAt ||
    (state.agents.rows[agentId]?.statusChangedAt ?? "") > incomingAt
  ) {
    return state;
  }

  const user = state.users[userId];

  const users =
    user === undefined || user.agent === null
      ? state.users
      : {
          ...state.users,
          [userId]: {
            ...user,
            agent: { ...user.agent, status: change.status, suspended: change.suspended },
          },
        };

  const row = state.agents.rows[agentId];

  const rows =
    row === undefined
      ? state.agents.rows
      : { ...state.agents.rows, [agentId]: withStatus(row, change) };

  const entry = state.agents.profiles[agentId];

  const profiles =
    entry?.profile === undefined || entry.profile === null
      ? state.agents.profiles
      : {
          ...state.agents.profiles,
          [agentId]: {
            ...entry,
            profile: { ...entry.profile, agent: withStatus(entry.profile.agent, change) },
          },
        };

  const { [agentId]: _lapsed, ...others } = state.agents.working;

  const working =
    change.workingPresence === null || change.workingPresenceExpiresAt === null
      ? others
      : {
          ...others,
          [agentId]: { text: change.workingPresence, expiresAt: change.workingPresenceExpiresAt },
        };

  const directory = state.agents.directory;
  const ids = directoryOrder(directory.ids, rows, users);

  return {
    ...state,
    users,
    agents: {
      ...state.agents,
      rows,
      profiles,
      working,
      live: { ...state.agents.live, [agentId]: { version: (previous?.version ?? 0) + 1, change } },
      directory: ids === directory.ids ? directory : { ...directory, ids },
    },
  };
}

/** What the agent says it's doing at `now`, or `null` when unset or lapsed. */
export function workingPresenceAt(state: State, agentId: number, now: number): string | null {
  const presence = state.agents.working[agentId];

  return presence === undefined || Date.parse(presence.expiresAt) <= now ? null : presence.text;
}

/** When the soonest working presence lapses (ms), or `null` when none is held. */
export function nextWorkingExpiry(state: State, now: number): number | null {
  let soonest: number | null = null;

  for (const presence of Object.values(state.agents.working)) {
    const at = Date.parse(presence.expiresAt);

    if (at > now) {
      soonest = soonest === null ? at : Math.min(soonest, at);
    }
  }

  return soonest;
}

/** Whether `user` is an agent at work now: working, and not suspended. */
export function isWorking(user: User | undefined): boolean {
  return user?.agent?.status === "working" && !user.agent.suspended;
}

const NO_IDS: readonly number[] = [];

/** How many of a room's newest loaded messages count when finding the agents present. */
const RECENT_MESSAGES = 50;

/**
 * The agents known to be in a room: its direct members and header preview, and the authors of
 * its newest loaded messages. The contract carries no full member list, so this is what the
 * client can tell; an agent that never spoke and isn't in the preview isn't counted.
 */
export function roomAgentIds(state: State, roomId: number): readonly number[] {
  const detail = state.rooms[roomId]?.detail ?? null;
  const ids = new Set<number>();

  if (detail !== null) {
    for (const id of [...detail.directMemberIds, ...detail.memberPreviewIds]) {
      ids.add(id);
    }
  }

  for (const messageId of (state.timelines[roomId]?.ids ?? NO_IDS).slice(-RECENT_MESSAGES)) {
    const message = state.messages[messageId];

    if (message !== undefined) {
      ids.add(message.creatorId);
    }
  }

  const agents = [...ids].filter((id) => state.users[id]?.agent != null).sort((a, b) => a - b);

  return agents.length === 0 ? NO_IDS : agents;
}

// --- steps ---

function compareSteps(left: AgentStep, right: AgentStep): number {
  return left.position - right.position || left.id - right.id;
}

/**
 * `held` and `incoming` merged by id, each step the copy with the later `updatedAt` (on a tie,
 * `incoming`), in `(position, id)` order. Answers `held` itself when nothing changed.
 */
export function mergeSteps(
  held: readonly AgentStep[],
  incoming: readonly AgentStep[],
): readonly AgentStep[] {
  if (incoming.length === 0) {
    return held;
  }

  const byId = new Map(held.map((step) => [step.id, step]));
  let changed = false;

  for (const step of incoming) {
    const kept = byId.get(step.id);

    if (kept === undefined || kept.updatedAt <= step.updatedAt) {
      changed ||= kept === undefined || !sameStep(kept, step);
      byId.set(step.id, step);
    }
  }

  return changed ? [...byId.values()].sort(compareSteps) : held;
}

/** Work step reads use tickets for equal timestamps; events share the same per-step keys. */
export function mergeFreshSteps(
  state: State,
  held: readonly AgentStep[],
  incoming: readonly AgentStep[],
  ticket?: number,
) {
  let freshness = state.freshness;
  const byId = new Map(held.map((step) => [step.id, step]));

  for (const step of incoming) {
    const copies = {
      held: byId.get(step.id),
      incoming: step,
      same: sameStep,
      timestamp: (value: AgentStep) => value.updatedAt,
    };

    const result =
      ticket === undefined
        ? receive(freshness, `agent-step:${step.id}`, copies)
        : land(freshness, `agent-step:${step.id}`, ticket, copies);

    freshness = result.freshness;

    if (result.value !== undefined) {
      byId.set(step.id, result.value);
    }
  }

  const steps = [...byId.values()].sort(compareSteps);

  return {
    freshness,
    steps:
      steps.length === held.length &&
      steps.every((step, index) => sameStep(step, held[index] ?? step))
        ? held
        : steps,
  };
}

function sameStep(left: AgentStep, right: AgentStep): boolean {
  return (
    left.updatedAt === right.updatedAt &&
    left.status === right.status &&
    left.name === right.name &&
    left.position === right.position &&
    left.durationMs === right.durationMs &&
    left.inputSummary === right.inputSummary &&
    left.outputSummary === right.outputSummary
  );
}

/**
 * Two copies of one message: the newer by the message's `updatedAt` (on a tie, the one `tie`
 * names), with the steps of both merged. Answers `held` when it stays as it was.
 */
export function mergeMessageCopies(
  held: MessageDTO,
  incoming: MessageDTO,
  tie: "held" | "incoming",
): MessageDTO {
  const incomingWins =
    incoming.updatedAt > held.updatedAt ||
    (tie === "incoming" && incoming.updatedAt === held.updatedAt);

  if (incomingWins) {
    // The held copy's steps count where they're newer than the incoming copy's.
    const steps = mergeSteps(incoming.steps, newerOnly(held.steps, incoming.steps));

    return steps === incoming.steps ? incoming : { ...incoming, steps: [...steps] };
  }

  const steps = mergeSteps(held.steps, incoming.steps);

  return steps === held.steps ? held : { ...held, steps: [...steps] };
}

/** The steps of `candidates` strictly newer than their copy in `base` (or missing from it). */
function newerOnly(
  candidates: readonly AgentStep[],
  base: readonly AgentStep[],
): readonly AgentStep[] {
  const byId = new Map(base.map((step) => [step.id, step]));

  return candidates.filter((step) => {
    const other = byId.get(step.id);

    return other === undefined || other.updatedAt < step.updatedAt;
  });
}

/**
 * `agent.steps` merged into a held message or work detail. Unheld parents pick up their steps
 * when they load.
 */
export function applyAgentSteps(state: State, change: AgentStepsChanged): State {
  if (change.messageId === null) {
    const threadId = change.threadId;
    const detail = threadId === null ? undefined : state.work.details[threadId];

    if (threadId === null || detail === undefined) {
      return state;
    }

    const { steps, freshness } = mergeFreshSteps(state, detail.steps, change.steps);

    return steps === detail.steps
      ? state
      : {
          ...state,
          freshness,
          work: {
            ...state.work,
            details: { ...state.work.details, [threadId]: { ...detail, steps: [...steps] } },
          },
        };
  }

  const message = state.messages[change.messageId];

  if (message === undefined) {
    return state;
  }

  const steps = mergeSteps(message.steps, change.steps);

  return steps === message.steps
    ? state
    : {
        ...state,
        messages: { ...state.messages, [message.id]: { ...message, steps: [...steps] } },
      };
}
