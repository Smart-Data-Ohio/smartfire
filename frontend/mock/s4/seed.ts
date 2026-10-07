/**
 * The S4 work seed: tracked threads in the rooms the earlier seeds made (owned by people, by
 * Ember the agent, by a deactivated person, and unassigned), links of every kind, a result, a
 * history with each entry kind, agent steps, and board posts in a board the viewer can't open
 * here.
 */
import type { AgentStep } from "../../src/gen/AgentStep.ts";
import type { WorkHistoryEntry } from "../../src/gen/WorkHistoryEntry.ts";
import type { WorkLink } from "../../src/gen/WorkLink.ts";
import type { WorkStatus } from "../../src/gen/WorkStatus.ts";
import { type Mentionable, renderMarkdown } from "../markdown.ts";
import { DEFAULT_AUTO_ARCHIVE_MINUTES, iso, type ThreadRecord } from "../s2/model.ts";
import { THREAD_IDS } from "../s2/seed.ts";
import { S3_THREAD_IDS } from "../s3/seed.ts";
import { ROOM_IDS, USER_IDS, VIEWER_ID, type World } from "../seed.ts";
import { emptyWork, ownerActive, ownerSnapshot, setOwner, type WorkRecord } from "./work-model.ts";

const MINUTE = 60_000;

const HOUR = 60 * MINUTE;

const DAY = 24 * HOUR;

/** The seeded work, for tests and screenshots. */
export const S4_WORK_IDS = {
  /** #general "Invite-to-first-message conversion dip": in progress, owned by Ember, a run. */
  agentOwned: THREAD_IDS.generalActive,
  /** #general, closed: done, owned by Maya, with a result. */
  done: THREAD_IDS.generalClosed,
  /** #general, locked: not tracked, so it can be. */
  untracked: THREAD_IDS.generalLocked,
  /** #design "Onboarding empty states": blocked, owned by Dana, who was deactivated. */
  inactiveOwner: THREAD_IDS.design,
  /** #launch-planning "Pricing page copy": in progress, owned by the viewer. */
  viewerOwned: S3_THREAD_IDS.pricingCopy,
  /** #launch-planning "SSO setup docs": planned, unassigned, draft and closed PRs. */
  unassigned: S3_THREAD_IDS.ssoDocs,
} as const;

/** The board the seeded board posts live in. The viewer belongs to it; the SPA has no page. */
export const S4_BOARD = { roomId: 41, name: "Product roadmap" } as const;

/** The seeded board posts. */
export const S4_BOARD_POST_IDS = { publicApi: 901, darkMode: 902, billing: 903 } as const;

/** Work state the world type doesn't hold: board posts and the next ids. */
export interface WorkState {
  /** Board posts: tracked threads in a board room, each with `work`. */
  readonly boardPosts: ThreadRecord[];
  nextHistoryId: number;
  nextLinkId: number;
}

const states = new WeakMap<World, WorkState>();

/** The world's work state (an empty one for a world the work seed never saw). */
export function workStateOf(world: World): WorkState {
  const existing = states.get(world);

  if (existing !== undefined) return existing;

  const fresh: WorkState = { boardPosts: [], nextHistoryId: 1, nextLinkId: 1 };

  states.set(world, fresh);

  return fresh;
}

interface EntrySeed {
  readonly kind: WorkHistoryEntry["kind"];
  readonly ago: number;
  readonly actorId: number | null;
  readonly from?: WorkStatus | null;
  readonly to?: WorkStatus | null;
  readonly fromOwner?: number | null;
  readonly toOwner?: number | null;
  readonly note?: string;
  readonly handoff?: WorkHistoryEntry["handoff"];
}

interface LinkSeed {
  readonly kind: WorkLink["kind"];
  readonly label: string;
  readonly url: string;
  readonly state?: WorkLink["pullRequestState"];
  readonly title?: string;
  readonly startsIn?: number;
  readonly cancelled?: boolean;
}

interface StepSeed {
  readonly name: string;
  readonly status: AgentStep["status"];
  readonly input?: string;
  readonly output?: string;
  readonly durationMs?: number;
}

interface WorkSeed {
  readonly status: WorkStatus;
  readonly ownerId: number | null;
  readonly updatedAgo: number;
  readonly runUrl?: string;
  readonly links?: readonly LinkSeed[];
  readonly result?: { readonly markdown: string; readonly byId: number; readonly ago: number };
  readonly steps?: readonly StepSeed[];
  /** Newest first. */
  readonly history: readonly EntrySeed[];
}

const REPO = "https://github.com/smart-data-ohio";

const SEEDS: ReadonlyMap<number, WorkSeed> = new Map<number, WorkSeed>([
  [
    S4_WORK_IDS.agentOwned,
    {
      status: "in_progress",
      ownerId: USER_IDS.ember,
      updatedAgo: 12 * MINUTE,
      runUrl: `${REPO}/smartfire/actions/runs/11873402561`,
      links: [
        {
          kind: "pull_request",
          label: "smart-data-ohio/smartfire#412",
          url: `${REPO}/smartfire/pull/412`,
          state: "open",
          title: "Count invite-to-first-message once per invitee",
        },
        {
          kind: "drive_file",
          label: "Activation funnel, Q3",
          url: "https://drive.google.com/file/d/1q3ActivationFunnel/view",
        },
      ],
      steps: [
        {
          name: "Read the funnel dashboard query",
          status: "done",
          input: "dashboards/activation.sql",
          output: "Re-invites are counted as new invitees",
          durationMs: 840,
        },
        {
          name: "Dedupe invitees by email",
          status: "done",
          output: "Opened smart-data-ohio/smartfire#412",
          durationMs: 14_200,
        },
        { name: "Backfill the last 30 days", status: "running", input: "2026-09-06 to today" },
        { name: "Post the corrected numbers here", status: "pending" },
      ],
      history: [
        {
          kind: "update",
          ago: 40 * MINUTE,
          actorId: USER_IDS.ember,
          from: "blocked",
          to: "in_progress",
          fromOwner: USER_IDS.ember,
          toOwner: USER_IDS.ember,
          note: "CI is green again after the flaky fixture fix.",
        },
        {
          kind: "update",
          ago: 2 * HOUR,
          actorId: USER_IDS.ember,
          from: "in_progress",
          to: "blocked",
          fromOwner: USER_IDS.ember,
          toOwner: USER_IDS.ember,
          note: "Waiting on CI: the fixtures job is failing on main.",
        },
        {
          kind: "handoff",
          ago: 3 * HOUR,
          actorId: USER_IDS.jonah,
          from: "in_progress",
          to: "in_progress",
          fromOwner: USER_IDS.jonah,
          toOwner: USER_IDS.ember,
          handoff: {
            summary:
              "Events are wired on the server; the dashboard query still double counts re-invites. Next: dedupe by invitee, then backfill the last 30 days.",
            linkCount: 2,
            questionCount: 1,
          },
        },
        {
          kind: "update",
          ago: 5 * HOUR,
          actorId: USER_IDS.jonah,
          from: "planned",
          to: "in_progress",
          fromOwner: null,
          toOwner: USER_IDS.jonah,
        },
        {
          kind: "update",
          ago: 6 * HOUR,
          actorId: USER_IDS.maya,
          from: null,
          to: "planned",
          fromOwner: null,
          toOwner: null,
        },
      ],
    },
  ],
  [
    S4_WORK_IDS.done,
    {
      status: "done",
      ownerId: USER_IDS.maya,
      updatedAgo: 26 * HOUR,
      links: [
        {
          kind: "pull_request",
          label: "smart-data-ohio/smartfire#398",
          url: `${REPO}/smartfire/pull/398`,
          state: "merged",
          title: "Send invite emails from team@ with a plain-text part",
        },
      ],
      result: {
        markdown:
          "Root cause was the **invite email** landing in Promotions for Gmail users.\n\n- Switched the sender to `team@` and added a plain-text part\n- Open rate is back to 61% after two days\n\nThe funnel dedupe is a separate thread.",
        byId: USER_IDS.maya,
        ago: 26 * HOUR,
      },
      history: [
        { kind: "result", ago: 26 * HOUR, actorId: USER_IDS.maya, from: "done", to: "done" },
        {
          kind: "update",
          ago: 27 * HOUR,
          actorId: USER_IDS.maya,
          from: "in_progress",
          to: "done",
          fromOwner: USER_IDS.maya,
          toOwner: USER_IDS.maya,
        },
        {
          kind: "assignment",
          ago: 2 * DAY,
          actorId: null,
          from: "in_progress",
          to: "in_progress",
          fromOwner: null,
          toOwner: USER_IDS.maya,
        },
        { kind: "update", ago: 3 * DAY, actorId: null, from: null, to: "in_progress" },
      ],
    },
  ],
  [
    S4_WORK_IDS.inactiveOwner,
    {
      status: "blocked",
      ownerId: USER_IDS.dana,
      updatedAgo: 5 * HOUR,
      links: [
        {
          kind: "event",
          label: "Empty states review",
          url: `/rooms/${ROOM_IDS.design}/events/14`,
          startsIn: DAY + 3 * HOUR,
        },
        {
          kind: "event",
          label: "Design crit",
          url: `/rooms/${ROOM_IDS.design}/events/11`,
          startsIn: -2 * HOUR,
          cancelled: true,
        },
        {
          kind: "drive_file",
          label: "https://drive.google.com/file/d/1EmptyStatesIllustrations/view",
          url: "https://drive.google.com/file/d/1EmptyStatesIllustrations/view",
        },
      ],
      history: [
        {
          kind: "update",
          ago: 5 * HOUR,
          actorId: USER_IDS.lucia,
          from: "in_progress",
          to: "blocked",
          fromOwner: USER_IDS.dana,
          toOwner: USER_IDS.dana,
        },
        {
          kind: "update",
          ago: 4 * DAY,
          actorId: USER_IDS.dana,
          from: null,
          to: "in_progress",
          fromOwner: null,
          toOwner: USER_IDS.dana,
        },
      ],
    },
  ],
  [
    S4_WORK_IDS.viewerOwned,
    {
      status: "in_progress",
      ownerId: VIEWER_ID,
      updatedAgo: 50 * MINUTE,
      links: [
        {
          kind: "drive_file",
          label: "Pricing comparison table v3",
          url: "https://docs.google.com/document/d/1PricingTableV3/edit",
        },
        {
          kind: "event",
          label: "Pricing copy review",
          url: `/rooms/${ROOM_IDS.launchPlanning}/events/21`,
          startsIn: 2 * DAY + HOUR,
        },
      ],
      history: [
        {
          kind: "assignment",
          ago: 70 * MINUTE,
          actorId: USER_IDS.maya,
          from: "in_progress",
          to: "in_progress",
          fromOwner: null,
          toOwner: VIEWER_ID,
        },
        {
          kind: "update",
          ago: 80 * MINUTE,
          actorId: USER_IDS.maya,
          from: null,
          to: "in_progress",
          fromOwner: null,
          toOwner: null,
        },
      ],
    },
  ],
  [
    S4_WORK_IDS.unassigned,
    {
      status: "planned",
      ownerId: null,
      updatedAgo: 3 * HOUR,
      links: [
        {
          kind: "pull_request",
          label: "smart-data-ohio/docs#57",
          url: `${REPO}/docs/pull/57`,
          state: "draft",
          title: "SSO setup guide: Okta, Google and Entra",
        },
        {
          kind: "pull_request",
          label: "smart-data-ohio/internal-docs#51",
          url: `${REPO}/internal-docs/pull/51`,
          state: "closed",
        },
      ],
      history: [
        {
          kind: "update",
          ago: 3 * HOUR,
          actorId: USER_IDS.jonah,
          from: null,
          to: "planned",
          fromOwner: null,
          toOwner: null,
        },
      ],
    },
  ],
]);

interface BoardPostSeed {
  readonly id: number;
  readonly name: string;
  readonly creatorId: number;
  readonly createdAgo: number;
  readonly work: WorkSeed;
}

const BOARD_POSTS: readonly BoardPostSeed[] = [
  {
    id: S4_BOARD_POST_IDS.publicApi,
    name: "Public API v1",
    creatorId: USER_IDS.priya,
    createdAgo: 9 * DAY,
    work: {
      status: "in_progress",
      ownerId: USER_IDS.priya,
      updatedAgo: 30 * MINUTE,
      links: [
        {
          kind: "pull_request",
          label: "smart-data-ohio/smartfire#405",
          url: `${REPO}/smartfire/pull/405`,
          state: "open",
          title: "Versioned /api/v1 routes",
        },
      ],
      history: [],
    },
  },
  {
    id: S4_BOARD_POST_IDS.darkMode,
    name: "Dark mode polish",
    creatorId: USER_IDS.lucia,
    createdAgo: 14 * DAY,
    work: { status: "done", ownerId: USER_IDS.lucia, updatedAgo: 3 * DAY, history: [] },
  },
  {
    id: S4_BOARD_POST_IDS.billing,
    name: "Usage-based billing research",
    creatorId: USER_IDS.sam,
    createdAgo: 6 * DAY,
    work: { status: "planned", ownerId: null, updatedAgo: 2 * DAY, history: [] },
  },
];

function buildWork(
  world: World,
  state: WorkState,
  roomId: number,
  threadId: number,
  seed: WorkSeed,
  now: number,
  people: readonly Mentionable[],
): WorkRecord {
  const work = emptyWork(iso(now - seed.updatedAgo));

  work.status = seed.status;
  setOwner(world, work, seed.ownerId);
  // A board's members aren't modelled, so its owners count as active.
  work.ownerActive = world.rooms.has(roomId) ? ownerActive(world, roomId, seed.ownerId) : true;
  work.runUrl = seed.runUrl ?? null;

  for (const link of seed.links ?? []) {
    const event = link.kind === "event";

    work.links.push({
      id: state.nextLinkId++,
      kind: link.kind,
      label: link.label,
      url: link.url,
      pullRequestState: link.kind === "pull_request" ? (link.state ?? null) : null,
      title: link.title ?? null,
      eventStartsAt: event ? iso(now + (link.startsIn ?? 0)) : null,
      eventTimeZone: event ? "America/New_York" : null,
      eventCancelled: event && link.cancelled === true,
    });
  }

  if (seed.result !== undefined) {
    work.resultMarkdown = seed.result.markdown;
    work.resultHtml = renderMarkdown(seed.result.markdown, people);
    work.resultUpdatedById = seed.result.byId;
    work.resultUpdatedAt = iso(now - seed.result.ago);
  }

  seed.steps?.forEach((step, index) => {
    const at = iso(now - seed.updatedAgo - (seed.steps?.length ?? 0) * MINUTE + index * MINUTE);

    work.steps.push({
      id: threadId * 100 + index + 1,
      messageId: null,
      threadId,
      name: step.name,
      status: step.status,
      inputSummary: step.input ?? null,
      outputSummary: step.output ?? null,
      durationMs: step.durationMs ?? null,
      position: index,
      createdAt: at,
      updatedAt: at,
    });
  });

  // History ids ascend oldest first, so the newest entry has the highest id.
  const oldestFirst = [...seed.history].reverse().map((entry) => ({
    id: state.nextHistoryId++,
    kind: entry.kind,
    createdAt: iso(now - entry.ago),
    actorId: entry.actorId,
    fromStatus: entry.from ?? null,
    toStatus: entry.to ?? null,
    fromOwner: ownerSnapshot(world, entry.fromOwner ?? null),
    toOwner: ownerSnapshot(world, entry.toOwner ?? null),
    note: entry.note ?? null,
    handoff: entry.handoff ?? null,
  }));

  work.history.push(...oldestFirst.reverse());

  return work;
}

/** Seeds the work layer onto a built world and answers it. */
export function seedWork(world: World, now: number): World {
  const state = workStateOf(world);

  const people: Mentionable[] = [...world.users.values()].flatMap((user) =>
    user.status === "active" ? [{ id: user.id, name: user.name }] : [],
  );

  for (const [threadId, seed] of SEEDS) {
    const thread = world.threads.get(threadId);

    if (thread === undefined) continue;

    thread.work = buildWork(world, state, thread.roomId, threadId, seed, now, people);
  }

  for (const post of BOARD_POSTS) {
    const createdAt = iso(now - post.createdAgo);
    const updatedAt = iso(now - post.work.updatedAgo);

    state.boardPosts.push({
      id: post.id,
      roomId: S4_BOARD.roomId,
      parentMessageId: null,
      creatorId: post.creatorId,
      name: post.name,
      closed: false,
      locked: false,
      lastActivityAt: updatedAt,
      // Board posts never go quiet on their own.
      autoArchiveAfterMinutes: DEFAULT_AUTO_ARCHIVE_MINUTES * 365,
      createdAt,
      messages: [],
      memberIds: new Set([post.creatorId]),
      viewerMembership: null,
      work: buildWork(world, state, S4_BOARD.roomId, post.id, post.work, now, people),
    });
  }

  return world;
}
