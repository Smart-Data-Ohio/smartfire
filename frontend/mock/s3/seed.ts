/**
 * The S3 layer of the mock workspace, laid over the S1 and S2 seed: a full activity inbox tied
 * to the seeded rooms, messages, threads and people (every tab, a mix of unread, read and
 * handled, more than a page of read items), more than a page of saved items (in progress and
 * done, with reminders, some in threads and direct messages) and scheduled messages in every
 * state (upcoming, stranded, sent and dropped). It edits a few old messages into replies and
 * GitHub notifications, adds two work threads in #launch-planning and a channel the viewer left.
 * Deterministic and placed relative to `now`, on its own PRNG; the S1 and S2 data the earlier
 * tests pin stay as they were (#general's newest hundred messages and the scheduled message
 * there are untouched).
 */
import type { ActivityEventType } from "../../src/gen/ActivityEventType.ts";
import type { ActivityItem } from "../../src/gen/ActivityItem.ts";
import type { ActivitySource } from "../../src/gen/ActivitySource.ts";
import type { ActivityState } from "../../src/gen/ActivityState.ts";
import type { AgentApprovalStatus } from "../../src/gen/AgentApprovalStatus.ts";
import type { AgentBudgetCap } from "../../src/gen/AgentBudgetCap.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { SavedItem } from "../../src/gen/SavedItem.ts";
import type { ScheduledMessage } from "../../src/gen/ScheduledMessage.ts";
import { type Mentionable, mentionsUser, renderMarkdown } from "../markdown.ts";
import { createRandom, type Random } from "../random.ts";
import { VIEWER_TIME_ZONE } from "../s2/composer.ts";
import {
  buildMessage,
  DEFAULT_AUTO_ARCHIVE_MINUTES,
  indicatorOf,
  iso,
  plainDraft,
  type ThreadRecord,
} from "../s2/model.ts";
import {
  buildWorld as buildS2World,
  MESSAGE_IDS,
  SCHEDULED_IDS,
  seededReplyId,
  THREAD_IDS,
} from "../s2/seed.ts";
import {
  BOT_ID,
  ROOM_IDS,
  type RoomRecord,
  seededMessageId,
  seededUuid,
  USER_IDS,
  VIEWER_ID,
  type World,
} from "../seed.ts";
import { conversationTitle, type Namer } from "./conversations.ts";
import { withActivityState } from "./model.ts";
import { droppedSource, huddleSource, messageSource, reminderSource } from "./sources.ts";

const MINUTE = 60_000;

const HOUR = 60 * MINUTE;

const DAY = 24 * HOUR;

/** The rooms S3 adds. */
export const S3_ROOM_IDS = {
  /** #q3-offsite: a private channel the viewer left (invisible): a stranded and a dropped message. */
  offsite: 40,
} as const;

/** The work threads S3 adds in #launch-planning. */
export const S3_THREAD_IDS = {
  /** "Pricing page copy": assigned to the viewer, in progress. */
  pricingCopy: 5,
  /** "SSO setup docs": sitting in review (an SLA nudge). */
  ssoDocs: 6,
} as const;

/** The scheduled messages S3 adds (the S2 one in #general stays id 1). */
export const S3_SCHEDULED_IDS = {
  /** To Maya, tomorrow morning. */
  dmMayaPending: 2,
  /** In the #design thread, in three hours. */
  designThreadPending: 3,
  /** #engineering, in two days. */
  engineeringPending: 4,
  /** The group direct message, in five days. */
  groupDmPending: 5,
  /** #q3-offsite, which the viewer left: pending but not sendable. */
  stranded: 6,
  /** Sent to Maya two days ago (`sentMessageId` set). */
  dmMayaSent: 7,
  /** Sent in #engineering. */
  engineeringSent: 8,
  /** Sent in the group direct message. */
  groupDmSent: 9,
  /** Dropped: its #design thread was deleted. */
  droppedThread: 10,
  /** Dropped: the viewer lost access to #q3-offsite (no reason). */
  droppedAccess: 11,
} as const;

/** A saved reminder that falls due a few minutes after the server starts (live in dev). */
export const DUE_REMINDER_DELAY_MS = 4 * MINUTE;

/** The seeded saved messages S3 tests look for. */
export const S3_MESSAGE_IDS = {
  /** #engineering: a GitHub review request from Ember, unread. */
  prReviewUnread: seededMessageId(ROOM_IDS.engineering, 95),
  /** A saved #design message whose reminder falls due `DUE_REMINDER_DELAY_MS` after start. */
  dueReminder: seededMessageId(ROOM_IDS.design, 30),
  /** A saved message in the #design thread (done). */
  savedThreadReply: seededReplyId(THREAD_IDS.design, 1),
  /** A saved message in the direct message with Maya, reminded already. */
  savedDmReminded: seededMessageId(ROOM_IDS.dmMaya, 6),
} as const;

/** Ember's agent id (`/agents/:id/approvals`): the mock reuses each bot user's id as its agent id. */
const EMBER_AGENT_ID = BOT_ID;

/** #general messages from this index up stay exactly as S1 and S2 left them. */
const GENERAL_PINNED_FROM = 280;

const PR_TITLES: readonly (readonly [number, string, number])[] = [
  [312, "Shard the Rust matrix by crate", USER_IDS.jonah],
  [318, "Cache the sccache bucket per toolchain", USER_IDS.sam],
  [321, "Move presence onto the sync hub", USER_IDS.priya],
  [327, "Keyset paging for the activity inbox", USER_IDS.jonah],
  [330, "Drop the Rails asset pipeline", USER_IDS.theo],
  [334, "Rate limit the sync endpoint", USER_IDS.jonah],
  [339, "Fix the flaky huddle gateway test", USER_IDS.sam],
  [341, "Saved items: reminders on the dispatcher", USER_IDS.priya],
];

const KEYWORDS = /\b(deploy|release|launch|ship|pricing|onboarding|migration|staging)/i;

/** Ember's approval requests in the inbox, newest first (S4's approvals mock seeds the same). */
export const APPROVALS: readonly (readonly [string, AgentApprovalStatus])[] = [
  ["Run `deploy production` for release 2.0.1", "pending"],
  ["Merge PR #318 into main", "pending"],
  ["Post the weekly metrics digest to #announcements", "approved"],
  ["Open 3 issues from the incident review", "denied"],
  ["Rotate the staging API key", "expired"],
  ["Close 14 stale issues labelled `needs-repro`", "approved"],
  ["Invite the design contractors to #design", "cancelled"],
];

/**
 * The approval id of `APPROVALS[index]`: counting down from 100, so the newest has the highest id,
 * as the approvals page orders them.
 */
export function seededApprovalId(index: number): number {
  return 100 - index;
}

/** When `APPROVALS[index]` was requested. */
export function seededApprovalAt(now: number, index: number): number {
  return now - (index * 19 + 2) * HOUR;
}

const BUDGETS: readonly (readonly [AgentBudgetCap, string, number])[] = [
  ["messages", "messages", 200],
  ["external_actions", "external actions", 25],
  ["messages", "messages", 200],
];

interface EventSeed {
  readonly title: string;
  readonly roomId: number;
  readonly organizerId: number;
  readonly type: ActivityEventType;
  /** When it starts, from now. */
  readonly startsIn: number;
  /** When the item was recorded, before now. */
  readonly ago: number;
  readonly repeats?: string;
}

const EVENTS: readonly EventSeed[] = [
  {
    title: "Launch readiness review",
    roomId: ROOM_IDS.launchPlanning,
    organizerId: USER_IDS.maya,
    type: "event_invitation",
    startsIn: 2 * DAY + 3 * HOUR,
    ago: 40 * MINUTE,
  },
  {
    title: "Design crit",
    roomId: ROOM_IDS.design,
    organizerId: USER_IDS.lucia,
    type: "event_update",
    startsIn: DAY + 2 * HOUR,
    ago: 5 * HOUR,
  },
  {
    title: "Friday demo",
    roomId: ROOM_IDS.general,
    organizerId: USER_IDS.priya,
    type: "event_cancelled",
    startsIn: 3 * DAY,
    ago: 26 * HOUR,
  },
  {
    title: "Standup",
    roomId: ROOM_IDS.engineering,
    organizerId: USER_IDS.jonah,
    type: "event_reminder",
    startsIn: -2 * DAY,
    ago: 2 * DAY + 15 * MINUTE,
  },
  {
    title: "1:1 with Maya",
    roomId: ROOM_IDS.dmMaya,
    organizerId: USER_IDS.maya,
    type: "event_invitation",
    startsIn: 4 * DAY,
    ago: 3 * DAY,
    repeats: "weekly",
  },
  {
    title: "Q3 offsite planning",
    roomId: ROOM_IDS.general,
    organizerId: USER_IDS.grace,
    type: "event_invitation",
    startsIn: 9 * DAY,
    ago: 6 * DAY,
  },
  {
    title: "Retro",
    roomId: ROOM_IDS.engineering,
    organizerId: USER_IDS.sam,
    type: "event_update",
    startsIn: -DAY,
    ago: 4 * DAY,
  },
];

const HUDDLES: readonly (readonly [number, number, boolean, number])[] = [
  [ROOM_IDS.lounge, USER_IDS.maya, false, 25 * MINUTE],
  [ROOM_IDS.dmMaya, USER_IDS.maya, true, 3 * HOUR],
  [ROOM_IDS.groupDm, USER_IDS.jonah, false, 20 * HOUR],
  [ROOM_IDS.general, USER_IDS.priya, false, 30 * HOUR],
  [ROOM_IDS.design, USER_IDS.lucia, true, 2 * DAY],
  [ROOM_IDS.lounge, USER_IDS.sam, false, 2 * DAY + 5 * HOUR],
  [ROOM_IDS.lounge, USER_IDS.theo, true, 3 * DAY],
  [ROOM_IDS.groupDm, USER_IDS.priya, false, 4 * DAY],
  [ROOM_IDS.general, USER_IDS.grace, false, 5 * DAY],
  [ROOM_IDS.lounge, USER_IDS.jonah, true, 7 * DAY],
];

const SIGN_INS: readonly (readonly [string, number])[] = [
  ["Safari on iPhone", 2 * HOUR],
  ["Firefox on Linux", 2 * DAY],
  ["Chrome on macOS", 6 * DAY],
];

const WORK_THREADS: readonly {
  readonly id: number;
  readonly parentIndex: number;
  readonly name: string;
  readonly replies: readonly (readonly [number, string])[];
}[] = [
  {
    id: S3_THREAD_IDS.pricingCopy,
    parentIndex: 12,
    name: "Pricing page copy",
    replies: [
      [USER_IDS.maya, "Assigning this to Riel. First draft by Thursday?"],
      [VIEWER_ID, "On it. I'll start from the comparison table."],
      [USER_IDS.lucia, "Mockup for the new table is in the design file."],
    ],
  },
  {
    id: S3_THREAD_IDS.ssoDocs,
    parentIndex: 18,
    name: "SSO setup docs",
    replies: [
      [USER_IDS.jonah, "Draft is up for review."],
      [USER_IDS.maya, "I'll get to it after the launch sync."],
    ],
  },
];

const SCHEDULED_TEXT = {
  dmMaya: "Morning! Notes from yesterday's call are in the doc. The pricing table is the big one.",
  designThread: "Dark mode pass looks great. One nit: the empty-state text needs more contrast.",
  engineering: "Weekly reminder: update your section of the on-call doc before Friday.",
  groupDm: "Shall we move the planning sync to Tuesday? Thursday is packed.",
  stranded: "Can everyone confirm their travel dates by Friday?",
  droppedThread: "Agreed on the softer illustrations. Shipping them in the next build.",
  droppedAccess: "Hotel block is booked, details in the doc.",
} as const;

/** Builds the whole workspace: the S1 seed, the S2 layer, and the S3 layer on top. */
export function buildWorld(now: number, seed: number): World {
  const world = buildS2World(now, seed);

  seedS3(world, now, createRandom(seed * 92_821 + 13));

  return world;
}

/** The server's room naming, for the seed's headings (a direct room is named after the others). */
function seedNamer(world: World): Namer {
  return {
    world: () => world,
    displayName(record: RoomRecord) {
      if (record.room.name !== null) return record.room.name;

      const others = record.memberIds.filter((id) => id !== VIEWER_ID);

      return (others.length > 0 ? others : [VIEWER_ID])
        .sort((a, b) => a - b)
        .flatMap((id) => world.users.get(id)?.name ?? [])
        .join(", ");
    },
  };
}

/** `%B %-d, %Y at %-I:%M %p %Z` in the viewer's zone, as the event bodies read. */
function eventTime(ms: number): string {
  const parts = new Intl.DateTimeFormat("en-US", {
    timeZone: VIEWER_TIME_ZONE,
    month: "long",
    day: "numeric",
    year: "numeric",
    hour: "numeric",
    minute: "2-digit",
    timeZoneName: "short",
  }).formatToParts(new Date(ms));

  const part = (type: Intl.DateTimeFormatPartTypes) =>
    parts.find((candidate) => candidate.type === type)?.value ?? "";

  return `${part("month")} ${part("day")}, ${part("year")} at ${part("hour")}:${part("minute")} ${part("dayPeriod")} ${part("timeZoneName")}`;
}

/** One seeded work item: its type, source and forced state (`null` for one by age). */
type WorkSeed = readonly [ActivityEventType, ActivitySource, ActivityState | null];

/** Approvals still waiting stay unread; approved ones were handled; the rest go by age. */
const APPROVAL_STATES: Readonly<Record<AgentApprovalStatus, ActivityState | null>> = {
  pending: "unread",
  approved: "handled",
  denied: null,
  cancelled: null,
  expired: null,
};

/** A source with no room, thread, message, event or creator unless `extra` names them. */
function sourceOf(
  sourceType: ActivitySource["sourceType"],
  sourceId: number,
  title: string,
  body: string,
  at: number,
  path: string,
  extra: Partial<ActivitySource> = {},
): ActivitySource {
  return {
    sourceType,
    sourceId,
    roomId: null,
    threadId: null,
    messageId: null,
    eventId: null,
    creatorId: null,
    title,
    body,
    occurredAt: iso(at),
    approvalStatus: null,
    budgetCap: null,
    path,
    ...extra,
  };
}

/** An event item's body (`event_body`). */
function eventBody(event: EventSeed, now: number): string {
  const startsAt = now + event.startsIn;
  const start = eventTime(startsAt);

  switch (event.type) {
    case "event_invitation": {
      if (event.repeats === undefined) return `You are invited: ${start}.`;

      const until = eventTime(startsAt + 70 * DAY).replace(/ at .*$/, "");

      return `You are invited: ${start} (repeats ${event.repeats} until ${until}).`;
    }

    case "event_update":
      return `The time changed: ${start}.`;
    case "event_cancelled":
      return "This event was cancelled.";
    default:
      return `Starts in 15 minutes: ${event.title} in Lounge.`;
  }
}

/** An item before it has an id: ids go out oldest first once every draft is in. */
interface ItemDraft {
  readonly eventType: ActivityEventType;
  readonly source: ActivitySource;
  readonly createdAt: number;
  /** Later than `createdAt` for grouped threads re-pointed at newer replies. */
  readonly updatedAt: number;
  /** Forced state; `null` for one by age. */
  readonly state: ActivityState | null;
}

/** Lays the S3 data over an S2 world, in place. */
export function seedS3(world: World, now: number, random: Random): void {
  const names = seedNamer(world);

  const people: Mentionable[] = [...world.users.values()].flatMap((user) =>
    user.status === "active" ? [{ id: user.id, name: user.name }] : [],
  );

  const room = (roomId: number): RoomRecord => {
    const record = world.rooms.get(roomId);

    if (record === undefined) throw new Error(`the seed has no room ${roomId}`);

    return record;
  };

  const rootAt = (roomId: number, index: number): MessageDTO => {
    const message = room(roomId).messages[index];

    if (message === undefined) throw new Error(`room ${roomId} has no message ${index}`);

    return message;
  };

  const replace = (next: MessageDTO) => {
    const list = room(next.roomId).messages;
    const index = list.findIndex((message) => message.id === next.id);

    if (index >= 0) list[index] = next;
  };

  /** Root messages outside #general's pinned tail, oldest first. */
  const editable = (roomId: number): MessageDTO[] =>
    roomId === ROOM_IDS.general
      ? room(roomId).messages.slice(0, GENERAL_PINNED_FROM)
      : [...room(roomId).messages];

  const featured = new Set<number>([
    ...Object.values(MESSAGE_IDS),
    seededMessageId(ROOM_IDS.engineering, 100),
    seededMessageId(ROOM_IDS.engineering, 135),
  ]);

  const drafts: ItemDraft[] = [];

  /** Adds an item recorded at `createdAt` (and not touched since), in `state` or by age. */
  const add = (
    eventType: ActivityEventType,
    source: ActivitySource,
    createdAt: number,
    state: ActivityState | null = null,
  ) => {
    drafts.push({ eventType, source, createdAt, updatedAt: createdAt, state });
  };

  const messageItem = (
    eventType: ActivityEventType,
    message: MessageDTO,
    state: ActivityState | null = null,
  ) => add(eventType, messageSource(names, message), Date.parse(message.createdAt), state);

  // --- #q3-offsite, the channel the viewer left ---

  world.rooms.set(S3_ROOM_IDS.offsite, {
    room: {
      id: S3_ROOM_IDS.offsite,
      kind: "closed",
      name: "q3-offsite",
      iconName: null,
      creatorId: USER_IDS.grace,
      createdAt: iso(now - 20 * DAY),
      updatedAt: iso(now - 2 * DAY),
    },
    memberIds: [USER_IDS.maya, USER_IDS.priya, USER_IDS.lucia, USER_IDS.grace],
    membership: {
      id: 100 + S3_ROOM_IDS.offsite,
      roomId: S3_ROOM_IDS.offsite,
      userId: VIEWER_ID,
      involvement: "invisible",
      unreadAt: null,
      lastReadMessageId: null,
      roomCategoryId: null,
      favoritePosition: null,
      stageRole: null,
    },
    messages: [],
    mentionCount: 0,
  });

  // --- the work threads in #launch-planning ---

  const workThreads = new Map<number, ThreadRecord>();

  for (const spec of WORK_THREADS) {
    const parent = rootAt(ROOM_IDS.launchPlanning, spec.parentIndex);
    const start = Date.parse(parent.createdAt);

    const thread: ThreadRecord = {
      id: spec.id,
      roomId: ROOM_IDS.launchPlanning,
      parentMessageId: parent.id,
      creatorId: spec.replies[0]?.[0] ?? parent.creatorId,
      name: spec.name,
      closed: false,
      locked: false,
      lastActivityAt: parent.createdAt,
      autoArchiveAfterMinutes: DEFAULT_AUTO_ARCHIVE_MINUTES,
      createdAt: iso(start + 1000),
      messages: [],
      memberIds: new Set([VIEWER_ID]),
      viewerMembership: null,
    };

    spec.replies.forEach(([creatorId, markdown], index) => {
      const at = Math.min(now - MINUTE, start + (index + 1) * 25 * MINUTE);

      const message = buildMessage(
        seededReplyId(spec.id, index),
        ROOM_IDS.launchPlanning,
        spec.id,
        plainDraft(creatorId, markdown, seededUuid(random)),
        iso(at),
        people,
      );

      thread.messages.push(message);
      thread.memberIds.add(creatorId);
      thread.lastActivityAt = message.createdAt;
    });

    thread.viewerMembership = {
      threadId: spec.id,
      involvement: "everything",
      unreadAt: null,
      joinedAt: thread.createdAt,
    };

    world.threads.set(spec.id, thread);
    replace({ ...parent, thread: indicatorOf(thread) });
    workThreads.set(spec.id, thread);
  }

  world.nextThreadId = Math.max(world.nextThreadId, S3_THREAD_IDS.ssoDocs + 1);

  // --- GitHub review requests: old #engineering messages become Ember's notifications ---

  const prIndexes = [8, 19, 27, 38, 49, 61, 72, 95];
  const prMessages: MessageDTO[] = [];

  prIndexes.forEach((index, order) => {
    const [number, title, authorId] = PR_TITLES[order] ?? [
      300,
      "Update dependencies",
      USER_IDS.sam,
    ];

    const author = world.users.get(authorId)?.name ?? "Someone";
    const url = `https://github.com/Smart-Data-Ohio/smartfire/pull/${number}`;

    const markdown = `**Review requested:** [#${number} ${title}](${url}) by ${author}. @[Riel St. Amand] is a requested reviewer.`;

    const message = {
      ...rootAt(ROOM_IDS.engineering, index),
      creatorId: BOT_ID,
      markdownSource: markdown,
      bodyHtml: renderMarkdown(markdown, people),
      editedAt: null,
    };

    replace({ ...message, updatedAt: message.createdAt });
    prMessages.push(message);
  });

  prMessages.forEach((message, order) => {
    messageItem(
      "pr_review_request",
      message,
      order === prMessages.length - 1 ? "unread" : undefined,
    );
  });

  // --- replies: someone answers the viewer's message just before theirs ---

  const replied = new Set<number>();

  for (const roomId of [
    ROOM_IDS.general,
    ROOM_IDS.engineering,
    ROOM_IDS.design,
    ROOM_IDS.launchPlanning,
    ROOM_IDS.groupDm,
  ]) {
    const list = editable(roomId);

    for (let index = 1; index < list.length && replied.size < 22; index++) {
      const previous = list[index - 1];
      const message = list[index];

      if (
        previous === undefined ||
        message === undefined ||
        previous.creatorId !== VIEWER_ID ||
        message.creatorId === VIEWER_ID ||
        message.creatorId === BOT_ID ||
        featured.has(message.id) ||
        Date.parse(message.createdAt) - Date.parse(previous.createdAt) > 20 * MINUTE
      ) {
        continue;
      }

      const reply = { ...message, replyToMessageId: previous.id };

      replace(reply);
      replied.add(reply.id);
      messageItem("reply", reply);
    }
  }

  // --- mentions: every message that mentions the viewer ---

  const mentioned = new Set<number>();

  for (const record of world.rooms.values()) {
    if (record.membership.involvement === "invisible") continue;

    for (const message of record.messages) {
      if (
        message.creatorId === VIEWER_ID ||
        replied.has(message.id) ||
        message.creatorId === BOT_ID ||
        !mentionsUser(message.bodyHtml, VIEWER_ID)
      ) {
        continue;
      }

      const unread =
        record.membership.unreadAt !== null &&
        message.id > (record.membership.lastReadMessageId ?? 0);

      mentioned.add(message.id);
      messageItem("mention", message, unread ? "unread" : undefined);
    }
  }

  // --- keyword alerts: "deploy", "release" and "launch" ---

  let alerts = 0;

  for (const roomId of [
    ROOM_IDS.engineering,
    ROOM_IDS.general,
    ROOM_IDS.random,
    ROOM_IDS.design,
    ROOM_IDS.launchPlanning,
    ROOM_IDS.groupDm,
  ]) {
    for (const message of [...editable(roomId)].reverse()) {
      if (alerts >= 60) break;

      if (
        message.creatorId === VIEWER_ID ||
        message.creatorId === BOT_ID ||
        mentioned.has(message.id) ||
        replied.has(message.id) ||
        !KEYWORDS.test(message.markdownSource ?? "")
      ) {
        continue;
      }

      alerts++;
      messageItem("keyword_alert", message);
    }
  }

  // --- followed threads ---

  const threadItem = (threadId: number, state: ActivityState) => {
    const thread = world.threads.get(threadId);
    const latest = thread?.messages.at(-1);
    const first = thread?.messages[0];

    if (thread === undefined || latest === undefined || first === undefined) return;

    drafts.push({
      eventType: "thread_activity",
      source: messageSource(names, latest),
      createdAt: Date.parse(first.createdAt),
      updatedAt: Date.parse(latest.createdAt),
      state,
    });
  };

  threadItem(THREAD_IDS.generalActive, "unread");
  threadItem(THREAD_IDS.generalClosed, "handled");
  threadItem(THREAD_IDS.generalLocked, "read");

  // --- work: an assignment, status changes and an SLA nudge ---

  const workSource = (
    sourceType: "work_thread_event" | "board_sla_nudge",
    sourceId: number,
    thread: ThreadRecord,
    actorId: number | null,
    body: string,
    at: number,
  ): ActivitySource =>
    sourceOf(
      sourceType,
      sourceId,
      conversationTitle(names, thread.roomId, thread.id),
      body,
      at,
      `/rooms/${thread.roomId}?thread=${thread.id}`,
      { roomId: thread.roomId, threadId: thread.id, creatorId: actorId },
    );

  const pricing = workThreads.get(S3_THREAD_IDS.pricingCopy);
  const sso = workThreads.get(S3_THREAD_IDS.ssoDocs);

  if (pricing !== undefined && sso !== undefined) {
    const pricingAt = Date.parse(pricing.createdAt);
    const ssoAt = Date.parse(sso.createdAt);
    const assignment = "Owner: unassigned → Riel St. Amand";
    const handoff = "Status: In progress → In review and Owner: Riel St. Amand → Lucía Fernández";

    const work: readonly WorkSeed[] = [
      [
        "work_assignment",
        workSource("work_thread_event", 1, pricing, USER_IDS.maya, assignment, pricingAt),
        "handled",
      ],
      [
        "work_update",
        workSource(
          "work_thread_event",
          2,
          pricing,
          USER_IDS.maya,
          "Status: Planned → In progress",
          pricingAt + 30 * MINUTE,
        ),
        null,
      ],
      [
        "work_update",
        workSource(
          "work_thread_event",
          3,
          sso,
          USER_IDS.jonah,
          "Status: In progress → In review",
          ssoAt,
        ),
        null,
      ],
      [
        "work_sla",
        workSource(
          "board_sla_nudge",
          1,
          sso,
          null,
          "Sitting in In review for 3.5 hours",
          now - 50 * MINUTE,
        ),
        "unread",
      ],
      [
        "work_update",
        workSource("work_thread_event", 4, pricing, USER_IDS.lucia, handoff, now - 9 * HOUR),
        null,
      ],
    ];

    for (const [eventType, source, state] of work) {
      add(eventType, source, Date.parse(source.occurredAt), state);
    }
  }

  // --- calendar events ---

  EVENTS.forEach((event, index) => {
    const id = index + 1;

    add(
      event.type,
      sourceOf(
        "event",
        id,
        conversationTitle(names, event.roomId, null, event.title),
        eventBody(event, now),
        now - event.ago - HOUR,
        `/rooms/${event.roomId}/events/${id}`,
        {
          roomId: event.roomId,
          eventId: id,
          creatorId: event.organizerId,
        },
      ),
      now - event.ago,
    );
  });

  // --- agents: approvals and budget notices ---

  APPROVALS.forEach(([summary, status], index) => {
    const at = seededApprovalAt(now, index);
    const title = conversationTitle(names, ROOM_IDS.engineering, null, "Ember");

    add(
      "agent_approval_request",
      sourceOf(
        "agent_approval",
        seededApprovalId(index),
        title,
        summary,
        at,
        `/agents/${EMBER_AGENT_ID}/approvals`,
        {
          roomId: ROOM_IDS.engineering,
          creatorId: BOT_ID,
          approvalStatus: status,
        },
      ),
      at,
      APPROVAL_STATES[status],
    );
  });

  BUDGETS.forEach(([cap, label, limit], index) => {
    const at = now - (index * 2 + 1) * DAY - 3 * HOUR;

    add(
      "agent_budget_exceeded",
      sourceOf(
        "agent_budget_notice",
        index + 1,
        `Ember · daily ${label} budget`,
        `Ember hit its daily ${label} budget (${limit}/day).`,
        at,
        `/account/bots/${BOT_ID}/edit`,
        { creatorId: BOT_ID, budgetCap: cap },
      ),
      at,
    );
  });

  // --- huddles ---

  HUDDLES.forEach(([roomId, callerId, missed, ago], index) => {
    const at = now - ago;

    add(
      missed ? "huddle_missed" : "huddle_started",
      huddleSource(names, index + 1, roomId, callerId, missed, iso(at)),
      at,
    );
  });

  // --- older huddles, a couple a day over the last ten days ---

  const huddleRooms = [
    ROOM_IDS.lounge,
    ROOM_IDS.general,
    ROOM_IDS.design,
    ROOM_IDS.groupDm,
    ROOM_IDS.dmMaya,
  ];

  for (let index = 0; index < 40; index++) {
    const roomId = random.pick(huddleRooms);
    const callers = room(roomId).memberIds.filter((id) => id !== VIEWER_ID && id !== BOT_ID);
    const callerId = random.pick(callers);
    const missed = random.chance(0.3);
    const at = now - 2 * DAY - Math.floor((index * 8 * DAY) / 40) - random.int(0, 3 * HOUR);

    add(
      missed ? "huddle_missed" : "huddle_started",
      huddleSource(names, HUDDLES.length + index + 1, roomId, callerId, missed, iso(at)),
      at,
    );
  }

  // --- security ---

  SIGN_INS.forEach(([device, ago], index) => {
    const at = now - ago;
    const body = `New sign-in to your account from ${device}, ${utcTime(at)}. Wasn't you? Review your sessions.`;

    add(
      "new_sign_in",
      sourceOf("session", index + 1, "Account security", body, at, "/users/me/sessions"),
      at,
      index === 0 ? "unread" : null,
    );
  });

  add(
    "two_factor_lockout",
    sourceOf(
      "two_factor_credential",
      1,
      "Two-step sign-in",
      "Several wrong sign-in codes were entered for your account.",
      now - 90 * DAY,
      "/users/me/profile",
    ),
    now - 4 * DAY,
    "handled",
  );

  // --- saved items ---

  const savedCandidates: MessageDTO[] = [];

  const pickSaved = (list: readonly MessageDTO[], count: number) => {
    const pool = list.filter(
      (message) =>
        !message.systemNote &&
        !featured.has(message.id) &&
        !world.saved.has(message.id) &&
        !savedCandidates.includes(message),
    );

    for (let index = 0; index < count && pool.length > 0; index++) {
      const at = Math.floor(((index + 0.5) * pool.length) / count);
      const message = pool[Math.min(at, pool.length - 1)];

      if (message !== undefined && !savedCandidates.includes(message))
        savedCandidates.push(message);
    }
  };

  const mustSave = [
    S3_MESSAGE_IDS.dueReminder,
    S3_MESSAGE_IDS.savedThreadReply,
    S3_MESSAGE_IDS.savedDmReminded,
  ];

  for (const id of mustSave) {
    const message =
      [...world.rooms.values()]
        .flatMap((record) => record.messages)
        .find((candidate) => candidate.id === id) ??
      [...world.threads.values()]
        .flatMap((thread) => thread.messages)
        .find((candidate) => candidate.id === id);

    if (message !== undefined) savedCandidates.push(message);
  }

  pickSaved(editable(ROOM_IDS.general), 12);
  pickSaved(editable(ROOM_IDS.engineering), 12);
  pickSaved(editable(ROOM_IDS.design), 8);
  pickSaved(editable(ROOM_IDS.random), 5);
  pickSaved(editable(ROOM_IDS.launchPlanning), 5);
  pickSaved(editable(ROOM_IDS.announcements), 3);
  pickSaved(editable(ROOM_IDS.dmMaya), 6);
  pickSaved(editable(ROOM_IDS.groupDm), 5);
  pickSaved(editable(ROOM_IDS.dmEmber), 2);
  pickSaved(
    [...world.threads.values()].flatMap((thread) =>
      thread.id === THREAD_IDS.generalLocked ? [] : thread.messages,
    ),
    6,
  );

  const reminded: { readonly item: SavedItem; readonly message: MessageDTO }[] = [];

  savedCandidates.forEach((message, index) => {
    const createdAt = Math.min(
      now - 2 * MINUTE,
      Date.parse(message.createdAt) + random.int(5 * MINUTE, 3 * HOUR),
    );

    let remindAt: string | null = null;
    let remindedAt: string | null = null;

    if (message.id === S3_MESSAGE_IDS.dueReminder) {
      remindAt = iso(now + DUE_REMINDER_DELAY_MS);
    } else if (message.id === S3_MESSAGE_IDS.savedDmReminded || index % 9 === 4) {
      const at = Math.min(now - 10 * MINUTE, createdAt + random.int(HOUR, DAY));

      remindAt = iso(at);
      remindedAt = iso(at + 20_000);
    } else if (index % 9 === 7) {
      remindAt = iso(now + random.int(2, 72) * HOUR);
    }

    const done =
      message.id === S3_MESSAGE_IDS.savedThreadReply ||
      (index % 3 === 2 && remindedAt === null && remindAt === null);

    const item: SavedItem = {
      id: world.nextSavedId++,
      messageId: message.id,
      status: done ? "done" : "in_progress",
      remindAt,
      remindedAt,
      createdAt: iso(createdAt),
    };

    world.saved.set(message.id, item);

    if (remindedAt !== null) reminded.push({ item, message });
  });

  reminded.forEach(({ item, message }, index) => {
    add(
      "message_reminder",
      reminderSource(names, item.id, message),
      Date.parse(item.remindedAt ?? item.createdAt),
      index === 0 ? "unread" : null,
    );
  });

  // --- scheduled messages ---

  const scheduled = (
    id: number,
    roomId: number,
    threadId: number | null,
    markdownSource: string,
    sendAt: number,
    createdAgo: number,
    after: Partial<ScheduledMessage> = {},
  ): ScheduledMessage => {
    const message: ScheduledMessage = {
      id,
      roomId,
      threadId,
      replyToMessageId: null,
      markdownSource,
      sendAt: iso(sendAt),
      state: "pending",
      sendable: true,
      sentAt: null,
      sentMessageId: null,
      droppedAt: null,
      dropReason: null,
      createdAt: iso(Math.min(sendAt, now) - createdAgo),
      ...after,
    };

    world.scheduled.set(id, message);

    return message;
  };

  scheduled(
    S3_SCHEDULED_IDS.dmMayaPending,
    ROOM_IDS.dmMaya,
    null,
    SCHEDULED_TEXT.dmMaya,
    now + 17 * HOUR,
    30 * MINUTE,
  );
  scheduled(
    S3_SCHEDULED_IDS.designThreadPending,
    ROOM_IDS.design,
    THREAD_IDS.design,
    SCHEDULED_TEXT.designThread,
    now + 3 * HOUR,
    HOUR,
  );
  scheduled(
    S3_SCHEDULED_IDS.engineeringPending,
    ROOM_IDS.engineering,
    null,
    SCHEDULED_TEXT.engineering,
    now + 2 * DAY,
    3 * HOUR,
  );
  scheduled(
    S3_SCHEDULED_IDS.groupDmPending,
    ROOM_IDS.groupDm,
    null,
    SCHEDULED_TEXT.groupDm,
    now + 5 * DAY,
    DAY,
  );
  scheduled(
    S3_SCHEDULED_IDS.stranded,
    S3_ROOM_IDS.offsite,
    null,
    SCHEDULED_TEXT.stranded,
    now + 26 * HOUR,
    3 * DAY,
    {
      sendable: false,
    },
  );

  const sentFrom = (id: number, roomId: number, back: number) => {
    const mine = room(roomId).messages.filter(
      (message) => message.creatorId === VIEWER_ID && Date.parse(message.createdAt) < now - back,
    );

    const message = mine.at(-1);

    if (message === undefined) return;

    scheduled(
      id,
      roomId,
      null,
      message.markdownSource ?? "",
      Date.parse(message.createdAt),
      6 * HOUR,
      {
        state: "sent",
        sendable: false,
        sentAt: message.createdAt,
        sentMessageId: message.id,
      },
    );
  };

  sentFrom(S3_SCHEDULED_IDS.dmMayaSent, ROOM_IDS.dmMaya, 6 * HOUR);
  sentFrom(S3_SCHEDULED_IDS.engineeringSent, ROOM_IDS.engineering, DAY);
  sentFrom(S3_SCHEDULED_IDS.groupDmSent, ROOM_IDS.groupDm, 12 * HOUR);

  const droppedThread = scheduled(
    S3_SCHEDULED_IDS.droppedThread,
    ROOM_IDS.design,
    99,
    SCHEDULED_TEXT.droppedThread,
    now - 3 * DAY,
    2 * HOUR,
    {
      state: "dropped",
      sendable: false,
      droppedAt: iso(now - 3 * DAY + 20_000),
      dropReason: "its thread was deleted",
    },
  );

  const droppedAccess = scheduled(
    S3_SCHEDULED_IDS.droppedAccess,
    S3_ROOM_IDS.offsite,
    null,
    SCHEDULED_TEXT.droppedAccess,
    now - 20 * HOUR,
    DAY,
    {
      state: "dropped",
      sendable: false,
      droppedAt: iso(now - 20 * HOUR + 20_000),
      dropReason: null,
    },
  );

  world.nextScheduledId = Math.max(
    world.nextScheduledId,
    S3_SCHEDULED_IDS.droppedAccess + 1,
    SCHEDULED_IDS.generalPending + 1,
  );

  add(
    "scheduled_message_dropped",
    droppedSource(names, droppedThread),
    Date.parse(droppedThread.droppedAt ?? droppedThread.sendAt),
    "read",
  );
  add(
    "scheduled_message_dropped",
    droppedSource(names, droppedAccess),
    Date.parse(droppedAccess.droppedAt ?? droppedAccess.sendAt),
    "unread",
  );

  // --- the inbox: ids oldest first, states by age where not set ---

  drafts.sort((a, b) => a.updatedAt - b.updatedAt || a.createdAt - b.createdAt);

  for (const draft of drafts) {
    const state = draft.state ?? stateByAge(now - draft.updatedAt, random);

    const readAt =
      state === "unread"
        ? null
        : iso(Math.min(now - 1000, draft.updatedAt + random.int(MINUTE, 6 * HOUR)));

    const handledAt =
      state === "handled" && readAt !== null
        ? iso(Math.min(now - 500, Date.parse(readAt) + random.int(MINUTE, 2 * HOUR)))
        : null;

    const item: ActivityItem = withActivityState({
      id: world.nextActivityId++,
      eventType: draft.eventType,
      state: "unread",
      readAt,
      handledAt,
      createdAt: iso(draft.createdAt),
      // Every state change bumps `updatedAt` (`save_state`), so a read or handled item sorts by it.
      updatedAt: handledAt ?? readAt ?? iso(draft.updatedAt),
      source: draft.source,
    });

    world.activity.set(item.id, item);
  }
}

/** `%B %-d, %Y at %-I:%M %p UTC`, as the sign-in body reads. */
function utcTime(ms: number): string {
  const date = new Date(ms);
  const month = new Intl.DateTimeFormat("en-US", { month: "long", timeZone: "UTC" }).format(date);
  const hours = date.getUTCHours();
  const minutes = String(date.getUTCMinutes()).padStart(2, "0");

  return `${month} ${date.getUTCDate()}, ${date.getUTCFullYear()} at ${hours % 12 === 0 ? 12 : hours % 12}:${minutes} ${hours < 12 ? "AM" : "PM"} UTC`;
}

/** An item's state from its age: the last few hours unread, the last day mixed, older ones read or handled. */
function stateByAge(age: number, random: Random): ActivityState {
  if (age < 3 * HOUR) return "unread";

  if (age < 30 * HOUR) return random.chance(0.55) ? "unread" : "read";

  return random.chance(0.18) ? "handled" : "read";
}
