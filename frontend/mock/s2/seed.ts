/**
 * The S2 layer of the mock workspace, laid over the S1 seed without changing its rooms, counts
 * or unread state: reactions and boosts, pins, files, threads, a forward, a saved message, a
 * pending scheduled message, starred people and an agent's slash commands. Like the S1 seed it
 * is deterministic and placed relative to `now`; it draws from its own PRNG so the S1 data stays
 * exactly as it was.
 */
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { Reaction } from "../../src/gen/Reaction.ts";
import { type Mentionable, renderMarkdown } from "../markdown.ts";
import { createRandom, type Random } from "../random.ts";
import { seedCards } from "../s3/cards.ts";
import {
  ROOM_IDS,
  type RoomRecord,
  seededMessageId,
  seededUuid,
  seedWorld,
  USER_IDS,
  VIEWER_ID,
  type World,
} from "../seed.ts";
import {
  boardDeckPdf,
  funnelSvg,
  onboardingMockupPng,
  rateLimiterSource,
  signupsChartSvg,
} from "./assets.ts";
import { reactionContent } from "./emoji.ts";
import {
  buildMessage,
  DEFAULT_AUTO_ARCHIVE_MINUTES,
  defaultThreadName,
  indicatorOf,
  iso,
  plainDraft,
  type ThreadRecord,
} from "./model.ts";
import { attachmentOf, storeBlob } from "./uploads.ts";

const MINUTE = 60_000;

const HOUR = 60 * MINUTE;

const general = (index: number) => seededMessageId(ROOM_IDS.general, index);

/** The seeded threads. */
export const THREAD_IDS = {
  /** #general: about 6 replies from 3 people, the viewer following, the last 3 unread. */
  generalActive: 1,
  /** #general: an older thread someone closed. */
  generalClosed: 2,
  /** #general: a locked thread (replies refused). */
  generalLocked: 3,
  /** #design: a thread on a recent message; the viewer isn't in it. */
  design: 4,
} as const;

/** The first id a seeded thread reply gets: `THREAD_REPLY_BASE + threadId * 100 + index`. */
export const THREAD_REPLY_BASE = 500_000;

/** The seeded id of a thread's reply at `index` (oldest first). */
export function seededReplyId(threadId: number, index: number): number {
  return THREAD_REPLY_BASE + threadId * 100 + index;
}

/** The seeded messages S2 screens and tests start from. */
export const MESSAGE_IDS = {
  /** #general: three emoji reactions from several people, the viewer among them. */
  generalReactions: general(397),
  /** #general: Maya's `signups-by-week.svg` chart (1200×675), with reactions. */
  generalChart: general(395),
  /** #general: the root of the active thread. */
  generalThreadRoot: general(392),
  /** #general: Priya's `Q3-board-update.pdf`, pinned. */
  generalPdf: general(389),
  /** #general: a `:shipit:` custom-icon reaction and two text boosts. */
  generalBoosts: general(386),
  /** #general: a 👍 the viewer gave. */
  generalViewerReaction: general(383),
  /** #general: Theo's forward of a #design message, with a note. */
  generalForward: general(380),
  /** #general: pinned. */
  generalPinned: general(370),
  /** #general: a ❤️ the viewer gave, just before the unread tail. */
  generalOlderReaction: general(345),
  /** #general: the message the viewer saved. */
  generalSaved: general(344),
  /** #general: the root of the closed thread. */
  generalClosedThreadRoot: general(300),
  /** #general: pinned, older. */
  generalPinnedOld: general(250),
  /** #general: the root of the locked thread. */
  generalLockedThreadRoot: general(220),
  /** The active thread's reply carrying `signup-funnel.svg`. */
  generalThreadFunnel: seededReplyId(THREAD_IDS.generalActive, 0),
  /** The viewer's own reply in the active thread (replies after it are unread). */
  generalThreadViewerReply: seededReplyId(THREAD_IDS.generalActive, 2),
  /** #design: Lucía's `onboarding-v3.png` mockup (1200×750). */
  designImage: seededMessageId(ROOM_IDS.design, 64),
  /** #design: the root of the #design thread. */
  designThreadRoot: seededMessageId(ROOM_IDS.design, 60),
  /** #engineering: Jonah's `rate_limiter.rs`. */
  engineeringCode: seededMessageId(ROOM_IDS.engineering, 135),
  /** #engineering: pinned. */
  engineeringPinned: seededMessageId(ROOM_IDS.engineering, 100),
  /** #launch-planning: pinned. */
  launchPinned: seededMessageId(ROOM_IDS.launchPlanning, 20),
} as const;

/** The seeded scheduled message. */
export const SCHEDULED_IDS = {
  /** #general, tomorrow at 13:00 UTC. */
  generalPending: 1,
} as const;

/** The people the viewer starred. */
export const STARRED_USER_IDS: readonly number[] = [USER_IDS.maya, USER_IDS.priya, USER_IDS.theo];

/** The note on the seeded forward. */
export const FORWARD_NOTE = "Worth a look before Thursday's review";

interface SeedReply {
  readonly creatorId: number;
  readonly markdown: string;
  readonly file?: "funnel";
}

const ACTIVE_THREAD: readonly SeedReply[] = [
  { creatorId: USER_IDS.jonah, markdown: "Here's the funnel broken out by step.", file: "funnel" },
  {
    creatorId: USER_IDS.priya,
    markdown:
      "The drop is almost all between *invite sent* and *invite accepted*. Deliverability again?",
  },
  {
    creatorId: VIEWER_ID,
    markdown: "Could be. I'll check the bounce rate on the invite template.",
  },
  {
    creatorId: USER_IDS.jonah,
    markdown:
      "Bounces look normal. But the invite email went out without the workspace name for two days.",
  },
  {
    creatorId: USER_IDS.priya,
    markdown: "That would do it. People don't click an invite from a workspace they can't place.",
  },
  { creatorId: USER_IDS.jonah, markdown: "Fix is merged. I'll post the numbers again on Friday." },
];

const CLOSED_THREAD: readonly SeedReply[] = [
  { creatorId: USER_IDS.sam, markdown: "I can take this one." },
  { creatorId: USER_IDS.grace, markdown: "Thanks! The runbook is linked in the doc." },
  { creatorId: USER_IDS.sam, markdown: "Done, closing this out." },
];

const LOCKED_THREAD: readonly SeedReply[] = [
  { creatorId: USER_IDS.theo, markdown: "Can we keep this to the agreed format?" },
  { creatorId: USER_IDS.maya, markdown: "Agreed. Moving the rest of the discussion to the doc." },
  { creatorId: USER_IDS.priya, markdown: "Locking this so the decision stays easy to find." },
  { creatorId: USER_IDS.grace, markdown: "👍" },
];

const DESIGN_THREAD: readonly SeedReply[] = [
  { creatorId: USER_IDS.maya, markdown: "Love the softer empty states." },
  { creatorId: USER_IDS.theo, markdown: "Do we have a dark mode pass yet?" },
  { creatorId: USER_IDS.lucia, markdown: "Tomorrow. I'll drop it in here." },
];

/** Builds the whole workspace: the S1 seed with the S2 layer on top. */
export function buildWorld(now: number, seed: number): World {
  const world = seedWorld(now, createRandom(seed));

  seedS2(world, now, createRandom(seed * 65_537 + 11));
  seedCards(world, now);

  return world;
}

/** Lays the S2 data over an S1 world, in place. */
export function seedS2(world: World, now: number, random: Random): void {
  const people: Mentionable[] = [...world.users.values()].flatMap((user) =>
    user.status === "active" ? [{ id: user.id, name: user.name }] : [],
  );

  const hex = (length: number) =>
    Array.from({ length }, () => random.int(0, 15).toString(16)).join("");

  const blobs = { world: () => world, hex };

  const room = (roomId: number): RoomRecord => {
    const record = world.rooms.get(roomId);

    if (record === undefined) throw new Error(`the seed has no room ${roomId}`);

    return record;
  };

  /** Rewrites a seeded root message in place. */
  const edit = (messageId: number, change: (message: MessageDTO) => Partial<MessageDTO>) => {
    for (const record of world.rooms.values()) {
      const index = record.messages.findIndex((message) => message.id === messageId);
      const message = record.messages[index];

      if (message !== undefined) {
        record.messages[index] = { ...message, ...change(message) };

        return;
      }
    }
  };

  const messageAt = (messageId: number): MessageDTO => {
    for (const record of world.rooms.values()) {
      const message = record.messages.find((candidate) => candidate.id === messageId);

      if (message !== undefined) return message;
    }

    throw new Error(`the seed has no message ${messageId}`);
  };

  const text = (creatorId: number, markdown: string) => () => ({
    creatorId,
    markdownSource: markdown,
    bodyHtml: renderMarkdown(markdown, people),
  });

  const reactions = (pills: readonly (readonly [string, readonly number[]])[]): Reaction[] =>
    pills.map(([raw, reactorIds]) => {
      const content = reactionContent(raw);

      if (content === null) throw new Error(`${raw} isn't a reaction`);

      return { ...content, reactorIds: [...reactorIds] };
    });

  // --- #general: the featured messages ---

  const chart = storeBlob(blobs, "signups-by-week.svg", "image/svg+xml", signupsChartSvg());
  const deck = storeBlob(blobs, "Q3-board-update.pdf", "application/pdf", boardDeckPdf());
  const funnel = storeBlob(blobs, "signup-funnel.svg", "image/svg+xml", funnelSvg());
  const mockup = storeBlob(blobs, "onboarding-v3.png", "image/png", onboardingMockupPng());
  const code = storeBlob(blobs, "rate_limiter.rs", "text/x-rust", rateLimiterSource());

  edit(MESSAGE_IDS.generalReactions, (message) => ({
    ...text(USER_IDS.grace, "Shipped the new onboarding checklist to 100% of new workspaces 🎉")(),
    reactions: reactions([
      ["🎉", [USER_IDS.maya, USER_IDS.jonah, USER_IDS.priya, VIEWER_ID]],
      ["🔥", [USER_IDS.theo, USER_IDS.sam]],
      ["👏", [USER_IDS.lucia]],
    ]),
    updatedAt: message.updatedAt,
  }));

  edit(MESSAGE_IDS.generalChart, () => ({
    ...text(
      USER_IDS.maya,
      "Weekly signups since the pricing change. The dip in week 3 is the holiday, not the change.",
    )(),
    attachment: attachmentOf(chart),
    reactions: reactions([
      ["📈", [USER_IDS.priya, USER_IDS.jonah]],
      ["👀", [USER_IDS.theo]],
    ]),
  }));

  edit(
    MESSAGE_IDS.generalThreadRoot,
    text(
      USER_IDS.jonah,
      "Signup funnel numbers for last week are in. Invite-to-first-message conversion dropped to **41%**.",
    ),
  );

  edit(MESSAGE_IDS.generalPdf, () => ({
    ...text(
      USER_IDS.priya,
      "Board update deck for Thursday. Comments welcome before Wednesday noon.",
    )(),
    attachment: attachmentOf(deck),
  }));

  edit(MESSAGE_IDS.generalBoosts, (message) => ({
    ...text(USER_IDS.sam, "Release 2.0.0 is tagged. Deploying after lunch.")(),
    reactions: reactions([[":shipit:", [USER_IDS.jonah, USER_IDS.theo, VIEWER_ID]]]),
    boosts: [
      {
        id: world.nextBoostId++,
        boosterId: USER_IDS.maya,
        content: "nice work",
        createdAt: iso(Date.parse(message.createdAt) + 2 * MINUTE),
      },
      {
        id: world.nextBoostId++,
        boosterId: USER_IDS.theo,
        content: "finally!",
        createdAt: iso(Date.parse(message.createdAt) + 3 * MINUTE),
      },
    ],
  }));

  edit(MESSAGE_IDS.generalViewerReaction, () => ({
    reactions: reactions([["👍", [VIEWER_ID, USER_IDS.maya]]]),
  }));

  edit(MESSAGE_IDS.generalOlderReaction, () => ({
    reactions: reactions([
      ["❤️", [VIEWER_ID, USER_IDS.lucia]],
      ["😂", [USER_IDS.sam]],
    ]),
  }));

  // --- the forward: the newest #design message at least 10 minutes older than the copy ---

  const copyAt = Date.parse(messageAt(MESSAGE_IDS.generalForward).createdAt);
  const design = room(ROOM_IDS.design);
  let source: MessageDTO | undefined;

  for (let index = 62; index >= 0 && source === undefined; index--) {
    const candidate = design.messages[index];

    if (candidate !== undefined && Date.parse(candidate.createdAt) <= copyAt - 10 * MINUTE) {
      source = candidate;
    }
  }

  if (source !== undefined) {
    const original = source;

    edit(MESSAGE_IDS.generalForward, (message) => ({
      creatorId: USER_IDS.theo,
      markdownSource: original.markdownSource,
      bodyHtml: original.bodyHtml,
      forwardedFromMessageId: original.id,
      forwardedAt: message.createdAt,
      forwardNote: FORWARD_NOTE,
      editedAt: null,
      updatedAt: message.createdAt,
    }));
  }

  // --- pins ---

  const pin = (messageId: number, pinnerId: number) => {
    const message = messageAt(messageId);

    world.pins.set(messageId, {
      roomId: message.roomId,
      pin: {
        messageId,
        pinnerId,
        pinnedAt: iso(Math.min(now - MINUTE, Date.parse(message.createdAt) + 5 * MINUTE)),
      },
    });
    edit(messageId, () => ({ pinned: true }));
  };

  pin(MESSAGE_IDS.generalPdf, USER_IDS.priya);
  pin(MESSAGE_IDS.generalPinned, USER_IDS.maya);
  pin(MESSAGE_IDS.generalPinnedOld, USER_IDS.jonah);
  pin(MESSAGE_IDS.engineeringPinned, USER_IDS.jonah);
  pin(MESSAGE_IDS.launchPinned, USER_IDS.maya);

  // --- the saved message ---

  world.saved.set(MESSAGE_IDS.generalSaved, {
    id: world.nextSavedId++,
    messageId: MESSAGE_IDS.generalSaved,
    status: "in_progress",
    remindAt: null,
    remindedAt: null,
    createdAt: iso(
      Math.min(now - HOUR, Date.parse(messageAt(MESSAGE_IDS.generalSaved).createdAt) + HOUR),
    ),
  });

  // --- #design and #engineering files ---

  edit(MESSAGE_IDS.designImage, () => ({
    ...text(USER_IDS.lucia, "Onboarding v3 mockups. The checklist moves into the sidebar.")(),
    attachment: attachmentOf(mockup),
  }));

  edit(MESSAGE_IDS.engineeringCode, () => ({
    ...text(USER_IDS.jonah, "Token bucket rate limiter for the sync endpoint, ready for review.")(),
    attachment: attachmentOf(code),
  }));

  // --- threads ---

  const thread = (
    id: number,
    parentId: number,
    name: string | null,
    replies: readonly SeedReply[],
    spanMs: number,
  ): ThreadRecord => {
    const parent = messageAt(parentId);
    const start = Date.parse(parent.createdAt);
    const end = Math.min(now - 30_000, start + spanMs);

    const record: ThreadRecord = {
      id,
      roomId: parent.roomId,
      parentMessageId: parent.id,
      creatorId: replies[0]?.creatorId ?? parent.creatorId,
      name: name ?? defaultThreadName(parent),
      closed: false,
      locked: false,
      lastActivityAt: parent.createdAt,
      autoArchiveAfterMinutes: DEFAULT_AUTO_ARCHIVE_MINUTES,
      createdAt: iso(start + 1000),
      messages: [],
      memberIds: new Set(),
      viewerMembership: null,
    };

    replies.forEach((reply, index) => {
      const at = Math.round(start + ((end - start) * (index + 1)) / (replies.length + 1));

      const message = buildMessage(
        seededReplyId(id, index),
        parent.roomId,
        id,
        {
          ...plainDraft(reply.creatorId, reply.markdown, seededUuid(random)),
          attachment: reply.file === "funnel" ? attachmentOf(funnel) : null,
        },
        iso(at),
        people,
      );

      record.messages.push(message);
      record.memberIds.add(reply.creatorId);
      record.lastActivityAt = message.createdAt;
    });

    world.threads.set(id, record);
    edit(parent.id, (message) => ({ thread: indicatorOf(record), updatedAt: message.updatedAt }));

    return record;
  };

  const active = thread(
    THREAD_IDS.generalActive,
    MESSAGE_IDS.generalThreadRoot,
    "Invite-to-first-message conversion dip",
    ACTIVE_THREAD,
    3 * HOUR,
  );

  const viewerReply = active.messages[2];
  const firstUnread = active.messages[3];

  active.viewerMembership = {
    threadId: active.id,
    involvement: "everything",
    unreadAt: firstUnread?.createdAt ?? null,
    joinedAt: viewerReply?.createdAt ?? active.createdAt,
  };

  const recentReply = active.messages[4];

  if (recentReply !== undefined) {
    active.messages[4] = {
      ...recentReply,
      reactions: reactions([["💯", [USER_IDS.jonah]]]),
    };
  }

  const closed = thread(
    THREAD_IDS.generalClosed,
    MESSAGE_IDS.generalClosedThreadRoot,
    null,
    CLOSED_THREAD,
    2 * HOUR,
  );

  closed.closed = true;

  const locked = thread(
    THREAD_IDS.generalLocked,
    MESSAGE_IDS.generalLockedThreadRoot,
    "Meeting format decision",
    LOCKED_THREAD,
    90 * MINUTE,
  );

  locked.locked = true;

  thread(
    THREAD_IDS.design,
    MESSAGE_IDS.designThreadRoot,
    "Onboarding empty states",
    DESIGN_THREAD,
    2 * HOUR,
  );
  world.nextThreadId = THREAD_IDS.design + 1;

  // --- the scheduled message: tomorrow at 13:00 UTC ---

  const today = new Date(now);

  const sendAt = Date.UTC(
    today.getUTCFullYear(),
    today.getUTCMonth(),
    today.getUTCDate() + 1,
    13,
    0,
    0,
  );

  world.scheduled.set(SCHEDULED_IDS.generalPending, {
    id: SCHEDULED_IDS.generalPending,
    roomId: ROOM_IDS.general,
    threadId: null,
    replyToMessageId: null,
    replyTarget: null,
    markdownSource: "Reminder: retro notes are due by end of day. Add yours to the doc 🙏",
    excerpt: "Reminder: retro notes are due by end of day. Add yours to the doc 🙏",
    sendAt: iso(sendAt),
    state: "pending",
    sendable: true,
    sentAt: null,
    sentMessageId: null,
    droppedAt: null,
    dropReason: null,
    createdAt: iso(now - 2 * HOUR),
  });
  world.nextScheduledId = SCHEDULED_IDS.generalPending + 1;

  // --- stars and agent commands ---

  for (const id of STARRED_USER_IDS) world.stars.add(id);

  world.dndAllowed.add(USER_IDS.maya);

  world.agentCommands.set(ROOM_IDS.engineering, [
    { name: "deploy-status", description: "Summarize the latest deploy", agentName: "Ember" },
    { name: "triage", description: null, agentName: "Ember" },
  ]);
}
