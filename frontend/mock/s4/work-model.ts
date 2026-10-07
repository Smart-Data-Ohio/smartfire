/**
 * What the mock keeps for a thread tracked as work (status, owner, links, result, steps and
 * history), and the pure helpers that turn it into the contract's `WorkFacts`, `WorkDetail`
 * and permission flags (`ChannelThread#work_*` in the Rust tree).
 */

import type { AgentCapability } from "../../src/gen/AgentCapability.ts";
import type { AgentStep } from "../../src/gen/AgentStep.ts";
import type { ThreadPermissions } from "../../src/gen/ThreadPermissions.ts";
import type { User } from "../../src/gen/User.ts";
import type { WorkDetail } from "../../src/gen/WorkDetail.ts";
import type { WorkFacts } from "../../src/gen/WorkFacts.ts";
import type { WorkHandoffReceiver } from "../../src/gen/WorkHandoffReceiver.ts";
import type { WorkHistoryEntry } from "../../src/gen/WorkHistoryEntry.ts";
import type { WorkLink } from "../../src/gen/WorkLink.ts";
import type { WorkOwnerCandidate } from "../../src/gen/WorkOwnerCandidate.ts";
import type { WorkOwnerSnapshot } from "../../src/gen/WorkOwnerSnapshot.ts";
import type { WorkStatus } from "../../src/gen/WorkStatus.ts";
import type { ThreadRecord } from "../s2/model.ts";
import type { World } from "../seed.ts";
import { S4_BOARD, workStateOf } from "./work-state.ts";

/** `ChannelThread::RESULT_LIMIT`. */
export const RESULT_LIMIT = 20_000;

/** `WorkHandoff::SUMMARY_LIMIT`. */
export const HANDOFF_SUMMARY_LIMIT = 2_000;

/** `WorkHandoff::COLLECTION_LIMIT`: links, and open questions, per handoff. */
export const HANDOFF_COLLECTION_LIMIT = 10;

/** `WorkHandoff::ENTRY_LIMIT`: one link or open question. */
export const HANDOFF_ENTRY_LIMIT = 500;

/** How much of a handoff summary a history entry keeps. */
export const HISTORY_SUMMARY_LIMIT = 200;

/** `ChannelThread::WORK_STATUSES`. */
export const WORK_STATUSES: readonly WorkStatus[] = ["planned", "in_progress", "blocked", "done"];

/**
 * A thread's work columns. `status` goes back to `null` when tracking stops; the history stays,
 * as the classic app keeps its `work_thread_events`.
 */
export interface WorkRecord {
  status: WorkStatus | null;
  ownerId: number | null;
  /** The owner's user as last written, whole, as `WorkFacts.owner` carries it. */
  owner: User | null;
  /** `work_owner_active`, decided when the owner was last written (not republished later). */
  ownerActive: boolean;
  runUrl: string | null;
  /** Oldest first. */
  readonly links: WorkLink[];
  resultMarkdown: string | null;
  resultHtml: string | null;
  resultUpdatedById: number | null;
  resultUpdatedAt: string | null;
  /** In `(position, id)` order. */
  readonly steps: AgentStep[];
  /** Newest first. */
  readonly history: WorkHistoryEntry[];
  /** The thread's `updated_at` as far as work goes: bumped by every work change. */
  updatedAt: string;
}

/** A thread with no work yet: untracked, no history. */
export function emptyWork(at: string): WorkRecord {
  return {
    status: null,
    ownerId: null,
    owner: null,
    ownerActive: false,
    runUrl: null,
    links: [],
    resultMarkdown: null,
    resultHtml: null,
    resultUpdatedById: null,
    resultUpdatedAt: null,
    steps: [],
    history: [],
    updatedAt: at,
  };
}

/** The thread's `WorkFacts`, or `null` when it isn't tracked. */
export function workFacts(work: WorkRecord | undefined): WorkFacts | null {
  if (work === undefined || work.status === null) return null;

  return {
    status: work.status,
    owner: work.owner,
    ownerActive: work.owner !== null && work.ownerActive,
    runUrl: work.runUrl,
    resultUpdatedAt: work.resultUpdatedAt,
    links: work.links,
  };
}

/** Points the work at `ownerId` (or nobody), with the owner's user as it is now. */
export function setOwner(world: World, work: WorkRecord, ownerId: number | null): void {
  work.ownerId = ownerId;
  work.owner = ownerId === null ? null : (world.users.get(ownerId) ?? null);
}

/** Whether the thread is tracked as work. */
export function isTracked(thread: ThreadRecord): boolean {
  return thread.work !== undefined && thread.work.status !== null;
}

/** The work list's agents filter joins the agent record, regardless of a bot's role. */
export function isAgentOwner(world: World, ownerId: number | null): boolean {
  return ownerId !== null && world.users.get(ownerId)?.agent != null;
}

/** The agent behind a bot user, if it's an active agent (not suspended). */
function activeAgent(user: User | undefined): User["agent"] {
  if (user === undefined || user.status !== "active" || user.role !== "bot") return null;

  const agent = user.agent;

  return agent === null || agent.suspended ? null : agent;
}

/** Whether `userId` is an active human in the room. */
export function isActiveHumanMember(world: World, roomId: number, userId: number): boolean {
  const user = world.users.get(userId);
  const member = world.rooms.get(roomId)?.memberIds.includes(userId) === true;

  return user !== undefined && user.role !== "bot" && user.status === "active" && member;
}

function holdsCapability(
  world: World,
  roomId: number,
  agentId: number,
  capability: AgentCapability,
): boolean {
  return workStateOf(world).agentCapabilities.get(`${roomId}:${agentId}`)?.has(capability) ?? true;
}

/** Whether `userId` is an active agent in the room that may post there. */
export function isPostingAgentMember(world: World, roomId: number, userId: number): boolean {
  const member = world.rooms.get(roomId)?.memberIds.includes(userId) === true;
  const agent = activeAgent(world.users.get(userId));

  return agent !== null && member && holdsCapability(world, roomId, agent.agentId, "post_messages");
}

/** `work_owner_active`: the owner can act on the work. */
export function ownerActive(world: World, roomId: number, ownerId: number | null): boolean {
  if (ownerId === null) return false;

  return (
    isActiveHumanMember(world, roomId, ownerId) || isPostingAgentMember(world, roomId, ownerId)
  );
}

/** An owner as the history records it: the name is a snapshot. */
export function ownerSnapshot(world: World, ownerId: number | null): WorkOwnerSnapshot | null {
  if (ownerId === null) return null;

  return { userId: ownerId, name: world.users.get(ownerId)?.name ?? null };
}

/** The five work flags (`ThreadPermissions`), for a viewer who `manages` the thread's settings. */
export function workPermissions(
  thread: ThreadRecord,
  manages: boolean,
  viewerId: number,
): Pick<
  ThreadPermissions,
  "canConvertWork" | "canManageWork" | "canUpdateWorkStatus" | "canAssignWork" | "canRemoveWork"
> {
  const tracked = isTracked(thread);
  const owns = tracked && thread.work?.ownerId === viewerId;
  const manageWork = tracked && (manages || owns);

  return {
    canConvertWork: manages && !tracked,
    canManageWork: manageWork,
    canUpdateWorkStatus: manageWork,
    canAssignWork: tracked && manages,
    canRemoveWork: tracked && manages && thread.roomId !== S4_BOARD.roomId,
  };
}

/** What the mock says about each agent in the owner picker (`agents.provider`, `description`). */
const AGENT_PROFILES: ReadonlyMap<number, { provider: string; description: string }> = new Map([
  [9, { provider: "Anthropic", description: "Workspace agent: triage, drafts and PR reviews" }],
]);

const byName = (world: World) => (a: number, b: number) =>
  (world.users.get(a)?.name ?? "")
    .toLowerCase()
    .localeCompare((world.users.get(b)?.name ?? "").toLowerCase());

/** The room's active humans, then its posting agents, each by lower-cased name. */
export function ownerCandidates(world: World, roomId: number): WorkOwnerCandidate[] {
  const members = world.rooms.get(roomId)?.memberIds ?? [];
  const humans = members.filter((id) => isActiveHumanMember(world, roomId, id)).sort(byName(world));

  const agents = members
    .filter((id) => isPostingAgentMember(world, roomId, id))
    .sort(byName(world));

  return [
    ...humans.map((userId) => ({ userId, provider: null, description: null })),
    ...agents.map((userId) => {
      const profile = AGENT_PROFILES.get(userId);

      return {
        userId,
        provider: profile?.provider ?? null,
        description: profile?.description ?? null,
      };
    }),
  ];
}

/** Receiver permission checks match the real backend / classic, in the same order. */
export function receiverError(
  world: World,
  roomId: number,
  ownerId: number | null,
  receiverAgentId: number,
): string | null {
  const user = [...world.users.values()].find((user) => user.agent?.agentId === receiverAgentId);

  if (user === undefined || !isPostingAgentMember(world, roomId, user.id)) {
    return "Receiver must be an active agent member of this room with permission to post";
  }

  if (!holdsCapability(world, roomId, receiverAgentId, "manage_threads")) {
    return "Receiver must hold the manage_threads capability in this room";
  }

  if (!holdsCapability(world, roomId, receiverAgentId, "read_messages")) {
    return "Receiver must hold the read_messages capability in this room";
  }

  return user.id === ownerId ? "Receiver is already the owner of this work" : null;
}

/** The agents in the room the work can go to, except its owner, by lower-cased name. */
export function handoffReceivers(
  world: World,
  roomId: number,
  ownerId: number | null,
): WorkHandoffReceiver[] {
  const members = world.rooms.get(roomId)?.memberIds ?? [];

  return members
    .filter((id) => {
      const agent = activeAgent(world.users.get(id));

      return agent !== null && receiverError(world, roomId, ownerId, agent.agentId) === null;
    })
    .sort(byName(world))
    .flatMap((userId) => {
      const agent = activeAgent(world.users.get(userId));

      return agent === null ? [] : [{ agentId: agent.agentId, userId }];
    });
}

/** The thread pane's work section for the viewer, or `null` when the thread isn't tracked. */
export function workDetail(
  world: World,
  thread: ThreadRecord,
  permissions: Pick<ThreadPermissions, "canAssignWork" | "canManageWork">,
): WorkDetail | null {
  const work = thread.work;

  if (work === undefined || work.status === null) return null;

  return {
    resultMarkdown: work.resultMarkdown,
    resultHtml: work.resultHtml,
    resultUpdatedById: work.resultUpdatedById,
    steps: work.steps,
    history: work.history,
    ownerCandidates: permissions.canAssignWork ? ownerCandidates(world, thread.roomId) : [],
    handoffReceivers: permissions.canManageWork
      ? handoffReceivers(world, thread.roomId, work.ownerId)
      : [],
  };
}

/** Everyone a work detail refers to: the result's editor, actors, candidates (the owner rides whole on the facts). */
export function workUserIds(thread: ThreadRecord, detail: WorkDetail | null): number[] {
  const work = thread.work;

  if (work === undefined || detail === null) return [];

  const ids: number[] = [];

  if (detail.resultUpdatedById !== null) ids.push(detail.resultUpdatedById);

  for (const entry of detail.history) {
    if (entry.actorId !== null) ids.push(entry.actorId);
  }

  for (const candidate of detail.ownerCandidates) ids.push(candidate.userId);

  for (const receiver of detail.handoffReceivers) ids.push(receiver.userId);

  return ids;
}

/** Trims each entry and drops blanks and repeats (`WorkHandoff::normalize_list`). */
export function normalizeList(entries: readonly string[]): string[] {
  const kept: string[] = [];

  for (const entry of entries) {
    const trimmed = entry.trim();

    if (trimmed !== "" && !kept.includes(trimmed)) kept.push(trimmed);
  }

  return kept;
}

/** Ruby's `truncate(limit, omission: "...")`: at most `limit` characters, "..." included. */
export function truncate(text: string, limit: number): string {
  const chars = [...text];

  return chars.length <= limit ? text : `${chars.slice(0, limit - 3).join("")}...`;
}
