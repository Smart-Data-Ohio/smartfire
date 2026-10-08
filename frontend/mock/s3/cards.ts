/**
 * The S3 cards in the mock: polls (results, create, vote), event attendance, and the per-viewer
 * previews of GitHub pull requests, Fizzy cards and quoted messages, with every state the
 * contract has (crates/api_types/src/cards.rs). The seed is one room of its own, #product-updates,
 * holding a message for each card kind and state, so the S1/S2 rooms and counts stay as they
 * were.
 */
import type { AttendanceResponse } from "../../src/gen/AttendanceResponse.ts";
import type { EventAttendance } from "../../src/gen/EventAttendance.ts";
import type { EventCard } from "../../src/gen/EventCard.ts";
import type { FizzyCardPreview } from "../../src/gen/FizzyCardPreview.ts";
import type { GithubPullRequest } from "../../src/gen/GithubPullRequest.ts";
import type { GithubPullRequestCard } from "../../src/gen/GithubPullRequestCard.ts";
import type { MessageCard } from "../../src/gen/MessageCard.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { Poll } from "../../src/gen/Poll.ts";
import type { PollResults } from "../../src/gen/PollResults.ts";
import type { QuotePreview } from "../../src/gen/QuotePreview.ts";
import type { QuotePreviewResult } from "../../src/gen/QuotePreviewResult.ts";
import { forbidden, type MockResponse, notFound, ok, validation } from "../http.ts";
import {
  booleanField,
  field,
  intField,
  type Json,
  stringArrayField,
  stringField,
} from "../json.ts";
import type { Mentionable } from "../markdown.ts";
import { createRandom } from "../random.ts";
import { firstId, type Route, route, type S2Context } from "../s2/context.ts";
import { buildMessage, plainDraft } from "../s2/model.ts";
import { clientMessageIdOf } from "../s2/posting.ts";
import {
  BOT_ID,
  ROOM_IDS,
  type RoomRecord,
  seededMessageId,
  seededUuid,
  timestamp,
  USER_IDS,
  VIEWER_ID,
  type World,
} from "../seed.ts";

const MINUTE = 60_000;

const HOUR = 60 * MINUTE;

const DAY = 24 * HOUR;

/** #product-updates: an id well clear of the S1/S2 rooms and the ones created at run time. */
export const CARDS_ROOM_ID = 45;

const message = (index: number) => seededMessageId(CARDS_ROOM_ID, index);

/** The seeded card messages, oldest first, for tests and screenshots. */
export const CARD_IDS = {
  room: CARDS_ROOM_ID,
  messages: {
    welcome: message(0),
    drive: message(1),
    githubOpen: message(2),
    githubMerged: message(3),
    githubDraftAndLoading: message(4),
    githubFailedAndHidden: message(5),
    xPost: message(6),
    xLoadingAndFailed: message(7),
    eventRecurring: message(8),
    eventCancelled: message(9),
    fizzyLoaded: message(10),
    fizzyNotConnectedAndNotFound: message(11),
    fizzyFailedAndLoading: message(12),
    quoteInline: message(13),
    quoteFetched: message(14),
    linkedin: message(15),
    linkedinChip: message(16),
    linkImage: message(17),
    linkPlain: message(18),
    suppressed: message(19),
    unknownKind: message(20),
    pollClosed: message(21),
    pollMultiple: message(22),
    pollAnonymous: message(23),
    pollOpen: message(24),
  },
  polls: { closed: 1, multiple: 2, anonymous: 3, open: 4 },
  events: { recurring: 1, cancelled: 2 },
  pullRequests: { open: 412, merged: 398, draft: 420, loading: 421, failed: 7, hidden: 8 },
  fizzy: { loaded: 31, notConnected: 32, notFound: 33, failed: 34, loading: 35 },
  references: { fetched: 51, hidden: 52, inline: 53 },
} as const;

const HUMANS = [1, 2, 3, 4, 5, 6, 7, 8];

// --- what the module keeps ---------------------------------------------------------------------

interface PollRecord {
  readonly id: number;
  readonly messageId: number;
  readonly roomId: number;
  readonly multiple: boolean;
  readonly anonymous: boolean;
  readonly closesAt: string | null;
  closedAt: string | null;
  readonly options: readonly { readonly id: number; readonly label: string }[];
  /** Each voter's chosen option ids. */
  readonly ballots: Map<number, readonly number[]>;
  asOf: string;
}

interface EventRecord {
  readonly card: EventCard;
  readonly responses: Map<number, AttendanceResponse>;
  readonly canApplyToFuture: boolean;
}

/** The cards part of the world. */
interface CardsWorld {
  readonly polls: Map<number, PollRecord>;
  nextPollId: number;
  nextOptionId: number;
  readonly events: Map<number, EventRecord>;
  /** The card under a message, by pull request id. */
  readonly pullRequests: Map<number, GithubPullRequestCard>;
  readonly fizzy: Map<number, FizzyCardPreview>;
  readonly quotes: Map<number, QuotePreviewResult>;
  /** Fetches served per preview (`github:412`), for tests. */
  readonly fetches: Map<string, number>;
}

const cardsWorlds = new WeakMap<World, CardsWorld>();

function emptyCardsWorld(): CardsWorld {
  return {
    polls: new Map(),
    nextPollId: 100,
    nextOptionId: 1000,
    events: new Map(),
    pullRequests: new Map(),
    fizzy: new Map(),
    quotes: new Map(),
    fetches: new Map(),
  };
}

function cardsOf(world: World): CardsWorld {
  const held = cardsWorlds.get(world);

  if (held !== undefined) return held;

  const fresh = emptyCardsWorld();

  cardsWorlds.set(world, fresh);

  return fresh;
}

/** An `asOf` after `previous`, even on a clock that stands still. */
function nextAsOf(now: number, previous: string | null): string {
  return timestamp(Math.max(now, previous === null ? 0 : Date.parse(previous) + 1));
}

function isClosed(poll: PollRecord, now: number): boolean {
  return poll.closedAt !== null || (poll.closesAt !== null && Date.parse(poll.closesAt) <= now);
}

/** The poll as sent (`Poll::results_payload`). */
function pollOf(poll: PollRecord, now: number): Poll {
  const options = poll.options.map((option) => {
    const voterIds = [...poll.ballots.entries()].flatMap(([userId, chosen]) =>
      chosen.includes(option.id) ? [userId] : [],
    );

    return {
      id: option.id,
      label: option.label,
      votes: voterIds.length,
      voterIds: poll.anonymous ? [] : voterIds.sort((a, b) => a - b),
    };
  });

  return {
    id: poll.id,
    messageId: poll.messageId,
    asOf: poll.asOf,
    multiple: poll.multiple,
    anonymous: poll.anonymous,
    closesAt: poll.closesAt,
    closedAt: poll.closedAt,
    closed: isClosed(poll, now),
    totalVotes: options.reduce((sum, option) => sum + option.votes, 0),
    options,
  };
}

function attendanceOf(event: EventRecord, viewerIsHuman: boolean): EventAttendance {
  const count = (response: AttendanceResponse) =>
    [...event.responses.values()].filter((value) => value === response).length;

  return {
    eventId: event.card.eventId,
    response: event.responses.get(VIEWER_ID) ?? null,
    goingCount: count("going"),
    maybeCount: count("maybe"),
    declinedCount: count("declined"),
    respondable: viewerIsHuman && !event.card.cancelled,
    canApplyToFuture: event.canApplyToFuture,
  };
}

// --- the seed ----------------------------------------------------------------------------------

const REPO = { owner: "smartdata", repo: "smartfire" } as const;

const prUrl = (number: number, repo: string = REPO.repo) =>
  `https://github.com/${REPO.owner}/${repo}/pull/${number}`;

function githubRef(pullRequestId: number, number: number, repo: string = REPO.repo): MessageCard {
  return {
    kind: "github",
    data: { pullRequestId, owner: REPO.owner, repo, number, url: prUrl(number, repo) },
  };
}

function pullRequest(
  number: number,
  change: Partial<GithubPullRequest> & Pick<GithubPullRequest, "title" | "status">,
): GithubPullRequestCard {
  return {
    state: "loaded",
    owner: REPO.owner,
    repo: REPO.repo,
    number,
    url: prUrl(number),
    authorLogin: "jonah-k",
    authorAvatarUrl: null,
    baseBranch: "main",
    headBranch: `feature/pr-${number}`,
    review: null,
    checks: null,
    githubUpdatedAt: null,
    discussionThreadId: null,
    files: null,
    ...change,
  };
}

function fizzyRef(fizzyCardId: number, number: number): MessageCard {
  return {
    kind: "fizzy",
    data: {
      fizzyCardId,
      accountId: "897362094",
      number,
      url: `https://app.fizzy.do/897362094/cards/${number}`,
    },
  };
}

/** An inline SVG picture (the mock serves no remote images), in two of the theme's hues. */
function artwork(from: string, to: string, label: string, width = 1200, height = 630): string {
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">` +
    `<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${from}"/>` +
    `<stop offset="1" stop-color="${to}"/></linearGradient></defs>` +
    `<rect width="100%" height="100%" fill="url(#g)"/>` +
    `<circle cx="${width * 0.78}" cy="${height * 0.3}" r="${height * 0.22}" fill="#ffffff" fill-opacity="0.18"/>` +
    `<text x="64" y="${height - 72}" font-family="system-ui, sans-serif" font-size="${Math.round(height / 9)}" font-weight="700" fill="#ffffff">${label}</text>` +
    "</svg>";

  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

interface SeedPost {
  readonly creatorId: number;
  readonly markdown: string;
  readonly cards?: readonly MessageCard[];
  readonly embedsSuppressed?: boolean;
}

/** Lays #product-updates and its cards over a world, in place. */
export function seedCards(world: World, now: number): void {
  const random = createRandom(4_099);
  const state = cardsOf(world);

  const people: Mentionable[] = [...world.users.values()].flatMap((user) =>
    user.status === "active" ? [{ id: user.id, name: user.name }] : [],
  );

  const general = world.rooms.get(ROOM_IDS.general);
  const quotedGeneral = general?.messages.at(-30);
  const welcomeAt = now - 2 * DAY;

  const quoteOf = (source: MessageDTO, roomLabel: string, excerpt: string): QuotePreview => ({
    messageId: source.id,
    roomId: source.roomId,
    threadId: source.threadId,
    creatorId: source.creatorId,
    authorName: world.users.get(source.creatorId)?.name ?? "Someone",
    roomLabel,
    excerpt,
    createdAt: source.createdAt,
  });

  const recurring: EventCard = {
    eventId: CARD_IDS.events.recurring,
    roomId: CARDS_ROOM_ID,
    title: "Weekly product sync",
    organizerId: USER_IDS.priya,
    startsAt: timestamp(Date.UTC(2026, 9, 8, 15, 0, 0)),
    endsAt: timestamp(Date.UTC(2026, 9, 8, 15, 45, 0)),
    timeZone: "America/New_York",
    recurring: true,
    cancelled: false,
    venueRoomId: null,
    venueName: null,
    meetLink: "https://meet.google.com/abc-defg-hij",
  };

  const cancelled: EventCard = {
    ...recurring,
    eventId: CARD_IDS.events.cancelled,
    title: "Launch retro",
    organizerId: USER_IDS.maya,
    startsAt: timestamp(Date.UTC(2026, 9, 9, 18, 0, 0)),
    endsAt: timestamp(Date.UTC(2026, 9, 9, 19, 0, 0)),
    recurring: false,
    cancelled: true,
    venueRoomId: ROOM_IDS.lounge,
    venueName: "Lounge",
    meetLink: null,
  };

  const posts: SeedPost[] = [
    {
      creatorId: USER_IDS.priya,
      markdown: "Welcome to #product-updates: launches, pull requests and plans in one place.",
    },
    {
      creatorId: USER_IDS.maya,
      markdown: "Q4 roadmap draft: https://drive.google.com/file/d/1RoadmapQ4draft/view",
      cards: [
        {
          kind: "drive",
          data: {
            fileId: "1RoadmapQ4draft",
            url: "https://drive.google.com/file/d/1RoadmapQ4draft/view",
          },
        },
      ],
    },
    {
      creatorId: USER_IDS.jonah,
      markdown: `The sync rate limiter is ready for review: ${prUrl(412)}`,
      cards: [githubRef(CARD_IDS.pullRequests.open, 412)],
    },
    {
      creatorId: USER_IDS.sam,
      markdown: `Merged the sidebar unread fix: ${prUrl(398)}`,
      cards: [githubRef(CARD_IDS.pullRequests.merged, 398)],
    },
    {
      creatorId: USER_IDS.theo,
      markdown: `Two for the onboarding work: ${prUrl(420)} and ${prUrl(421)}`,
      cards: [
        githubRef(CARD_IDS.pullRequests.draft, 420),
        githubRef(CARD_IDS.pullRequests.loading, 421),
      ],
    },
    {
      creatorId: USER_IDS.lucia,
      markdown: `From the infra repos: ${prUrl(7, "deploy")} and ${prUrl(8, "secrets")}`,
      cards: [
        githubRef(CARD_IDS.pullRequests.failed, 7, "deploy"),
        githubRef(CARD_IDS.pullRequests.hidden, 8, "secrets"),
      ],
    },
    {
      creatorId: USER_IDS.maya,
      markdown: "Nice write-up on the launch: https://x.com/smartdata/status/1843000000000000001",
      cards: [
        {
          kind: "x",
          data: {
            fetch: "loaded",
            postId: "1843000000000000001",
            url: "https://x.com/smartdata/status/1843000000000000001",
            authorName: "Smart Data",
            authorHandle: "smartdata",
            authorAvatarUrl: null,
            text: "Smartfire 2.0 is live: a faster backend, threads that keep up, and a new app on the way. Thanks to everyone who tested the betas.",
            postedAt: timestamp(now - 30 * HOUR),
            replies: 18,
            reposts: 64,
            likes: 412,
            media: [
              {
                kind: "photo",
                url: artwork("#e8590c", "#c2255c", "Smartfire 2.0"),
                thumbnailUrl: null,
                width: 1200,
                height: 630,
                alt: "The Smartfire 2.0 launch banner",
              },
            ],
            quote: {
              url: "https://x.com/rielstamand/status/1842000000000000002",
              authorName: "Riel St. Amand",
              authorHandle: "rielstamand",
              text: "Cutover done. Every request is on the new backend now.",
            },
          },
        },
      ],
    },
    {
      creatorId: USER_IDS.grace,
      markdown:
        "Two more: https://x.com/someone/status/1843000000000000003 and https://x.com/gone/status/1843000000000000004",
      cards: [
        {
          kind: "x",
          data: {
            fetch: "loading",
            postId: "1843000000000000003",
            url: "https://x.com/someone/status/1843000000000000003",
            authorName: "someone",
            authorHandle: null,
            authorAvatarUrl: null,
            text: null,
            postedAt: null,
            replies: null,
            reposts: null,
            likes: null,
            media: [],
            quote: null,
          },
        },
        {
          kind: "x",
          data: {
            fetch: "failed",
            postId: "1843000000000000004",
            url: "https://x.com/gone/status/1843000000000000004",
            authorName: "gone",
            authorHandle: null,
            authorAvatarUrl: null,
            text: null,
            postedAt: null,
            replies: null,
            reposts: null,
            likes: null,
            media: [],
            quote: null,
          },
        },
      ],
    },
    {
      creatorId: USER_IDS.priya,
      markdown: `Product sync moves to Thursdays: /rooms/${CARDS_ROOM_ID}/events/${recurring.eventId}`,
      cards: [{ kind: "event", data: recurring }],
    },
    {
      creatorId: USER_IDS.maya,
      markdown: `Calling off the retro this week: /rooms/${CARDS_ROOM_ID}/events/${cancelled.eventId}`,
      cards: [{ kind: "event", data: cancelled }],
    },
    {
      creatorId: USER_IDS.jonah,
      markdown: "Tracking the importer work here: https://app.fizzy.do/897362094/cards/214",
      cards: [fizzyRef(CARD_IDS.fizzy.loaded, 214)],
    },
    {
      creatorId: USER_IDS.sam,
      markdown:
        "https://app.fizzy.do/897362094/cards/215 and https://app.fizzy.do/897362094/cards/216",
      cards: [fizzyRef(CARD_IDS.fizzy.notConnected, 215), fizzyRef(CARD_IDS.fizzy.notFound, 216)],
    },
    {
      creatorId: USER_IDS.theo,
      markdown:
        "https://app.fizzy.do/897362094/cards/217 and https://app.fizzy.do/897362094/cards/218",
      cards: [fizzyRef(CARD_IDS.fizzy.failed, 217), fizzyRef(CARD_IDS.fizzy.loading, 218)],
    },
  ];

  const welcomeIndex = 0;

  posts.push(
    {
      creatorId: USER_IDS.lucia,
      markdown: "Pinning the room's purpose for newcomers, see the very first message.",
      cards: [],
    },
    {
      creatorId: USER_IDS.maya,
      markdown: "Context from #general, and a link from a room you can't see.",
      cards: [
        { kind: "quote", data: { referenceId: CARD_IDS.references.fetched, preview: null } },
        { kind: "quote", data: { referenceId: CARD_IDS.references.hidden, preview: null } },
      ],
    },
    {
      creatorId: USER_IDS.grace,
      markdown: "Our hiring post is up: https://www.linkedin.com/posts/smartdata_hiring-activity-1",
      cards: [
        {
          kind: "linkedin",
          data: {
            url: "https://www.linkedin.com/posts/smartdata_hiring-activity-1",
            title: "We're hiring: senior product engineer",
            description:
              "Smart Data is looking for an engineer who loves fast, calm software. Remote across North America.",
            imageUrl: artwork("#1c7ed6", "#5f3dc4", "We're hiring"),
            embedUrl: "https://www.linkedin.com/embed/feed/update/urn:li:share:1",
          },
        },
      ],
    },
    {
      creatorId: USER_IDS.priya,
      markdown: "https://www.linkedin.com/feed/update/urn:li:activity:2",
      cards: [
        {
          kind: "linkedin",
          data: {
            url: "https://www.linkedin.com/feed/update/urn:li:activity:2",
            title: null,
            description: null,
            imageUrl: null,
            embedUrl: null,
          },
        },
      ],
    },
    {
      creatorId: USER_IDS.jonah,
      markdown: "Worth a read before Thursday: https://calm.engineering/posts/fast-by-default",
      cards: [
        {
          kind: "link",
          data: {
            url: "https://calm.engineering/posts/fast-by-default",
            siteName: "Calm Engineering",
            title: "Fast by default: budgets that hold",
            description:
              "How small teams keep interfaces quick as they grow, with budgets checked on every change.",
            imageUrl: artwork("#2b8a3e", "#0b7285", "Fast by default"),
          },
        },
      ],
    },
    {
      creatorId: USER_IDS.sam,
      markdown: "Release notes: https://docs.smartdata.example/releases/2-0",
      cards: [
        {
          kind: "link",
          data: {
            url: "https://docs.smartdata.example/releases/2-0",
            siteName: null,
            title: "Smartfire 2.0 release notes",
            description: null,
            imageUrl: null,
          },
        },
      ],
    },
    {
      creatorId: USER_IDS.theo,
      markdown: `Previews off for this one: ${prUrl(412)} and https://calm.engineering/posts/fast-by-default`,
      embedsSuppressed: true,
      cards: [githubRef(CARD_IDS.pullRequests.open, 412)],
    },
    {
      creatorId: BOT_ID,
      markdown: "Daily digest: 3 pull requests merged, 2 waiting for review.",
      cards: [],
    },
  );

  const record: RoomRecord = {
    room: {
      id: CARDS_ROOM_ID,
      kind: "open",
      name: "product-updates",
      iconName: null,
      creatorId: USER_IDS.priya,
      createdAt: timestamp(welcomeAt - DAY),
      updatedAt: timestamp(welcomeAt),
    },
    memberIds: [...HUMANS, BOT_ID],
    membership: {
      id: 100 + CARDS_ROOM_ID,
      roomId: CARDS_ROOM_ID,
      userId: VIEWER_ID,
      involvement: "mentions",
      unreadAt: null,
      lastReadMessageId: null,
      roomCategoryId: null,
      favoritePosition: null,
      stageRole: null,
    },
    messages: [],
    mentionCount: 0,
  };

  const polls: readonly SeedPost[] = [
    { creatorId: USER_IDS.priya, markdown: "Which day works for the offsite?" },
    { creatorId: USER_IDS.maya, markdown: "Which features make the beta? Pick all you'd ship." },
    { creatorId: USER_IDS.jonah, markdown: "How confident are we in the **Oct 20** date?" },
    { creatorId: USER_IDS.priya, markdown: "Lunch for the launch party?" },
  ];

  const all = [...posts, ...polls];
  const newestAt = now - 20 * MINUTE;

  all.forEach((post, index) => {
    const at = timestamp(
      Math.round(welcomeAt + ((newestAt - welcomeAt) * index) / Math.max(1, all.length - 1)),
    );

    const built = buildMessage(
      message(index),
      CARDS_ROOM_ID,
      null,
      plainDraft(post.creatorId, post.markdown, seededUuid(random)),
      at,
      people,
    );

    record.messages.push({
      ...built,
      embedsSuppressed: post.embedsSuppressed ?? false,
      cards: [...(post.cards ?? [])],
    });
  });

  // The inline quote points at the room's own first message (its preview rides on the card).
  const welcome = record.messages[welcomeIndex];
  const inlineIndex = 13;
  const inline = record.messages[inlineIndex];

  if (welcome !== undefined && inline !== undefined) {
    record.messages[inlineIndex] = {
      ...inline,
      cards: [
        {
          kind: "quote",
          data: {
            referenceId: CARD_IDS.references.inline,
            preview: quoteOf(welcome, "product-updates", welcome.markdownSource ?? ""),
          },
        },
      ],
    };
  }

  // A card of a kind added after this build: the client skips it, the body still shows.
  const unknownIndex = 20;
  const unknown = record.messages[unknownIndex];

  if (unknown !== undefined) {
    // SAFETY: deliberately off the generated union; the wire may carry kinds the client lacks.
    const future = JSON.parse('{"kind":"youtube","data":{"videoId":"dQw4w9WgXcQ"}}') as MessageCard;

    record.messages[unknownIndex] = { ...unknown, cards: [future] };
  }

  const newest = record.messages.at(-1);

  record.room = { ...record.room, updatedAt: newest?.createdAt ?? record.room.updatedAt };
  record.membership = { ...record.membership, lastReadMessageId: newest?.id ?? null };
  world.rooms.set(CARDS_ROOM_ID, record);

  // --- polls ---

  const addPoll = (
    id: number,
    messageIndex: number,
    labels: readonly string[],
    options: Pick<PollRecord, "multiple" | "anonymous" | "closesAt" | "closedAt">,
    votes: readonly (readonly [number, readonly number[]])[],
  ) => {
    const held = record.messages[messageIndex];

    if (held === undefined) return;

    const poll: PollRecord = {
      id,
      messageId: held.id,
      roomId: CARDS_ROOM_ID,
      ...options,
      options: labels.map((label, index) => ({ id: id * 10 + index + 1, label })),
      ballots: new Map(
        votes.map(([userId, picks]) => [userId, picks.map((pick) => id * 10 + pick + 1)]),
      ),
      asOf: held.createdAt,
    };

    state.polls.set(id, poll);
    record.messages[messageIndex] = { ...held, poll: pollOf(poll, now) };
  };

  const first = posts.length;

  addPoll(
    CARD_IDS.polls.closed,
    first,
    ["Tuesday", "Wednesday", "Thursday"],
    {
      multiple: false,
      anonymous: false,
      closesAt: null,
      closedAt: timestamp(now - 3 * HOUR),
    },
    [
      [USER_IDS.maya, [1]],
      [USER_IDS.jonah, [1]],
      [USER_IDS.priya, [2]],
      [VIEWER_ID, [1]],
      [USER_IDS.sam, [0]],
    ],
  );

  addPoll(
    CARD_IDS.polls.multiple,
    first + 1,
    ["Slack import", "Polls", "Huddles", "Boards"],
    { multiple: true, anonymous: false, closesAt: null, closedAt: null },
    [
      [VIEWER_ID, [0, 1]],
      [USER_IDS.maya, [0, 2]],
      [USER_IDS.theo, [1]],
      [USER_IDS.lucia, [0, 1, 3]],
    ],
  );

  addPoll(
    CARD_IDS.polls.anonymous,
    first + 2,
    ["Very", "Somewhat", "Not yet"],
    { multiple: false, anonymous: true, closesAt: timestamp(now + 2 * HOUR), closedAt: null },
    [
      [USER_IDS.maya, [0]],
      [USER_IDS.jonah, [1]],
      [USER_IDS.sam, [1]],
      [VIEWER_ID, [0]],
    ],
  );

  addPoll(
    CARD_IDS.polls.open,
    first + 3,
    ["Tacos", "Pizza", "Sushi", "Something else"],
    { multiple: false, anonymous: false, closesAt: null, closedAt: null },
    [
      [USER_IDS.maya, [0]],
      [USER_IDS.jonah, [1]],
      [USER_IDS.priya, [0]],
      [USER_IDS.grace, [2]],
    ],
  );

  // --- events ---

  state.events.set(recurring.eventId, {
    card: recurring,
    responses: new Map([
      [USER_IDS.priya, "going"],
      [USER_IDS.maya, "going"],
      [USER_IDS.jonah, "maybe"],
      [USER_IDS.sam, "declined"],
    ]),
    canApplyToFuture: true,
  });

  state.events.set(cancelled.eventId, {
    card: cancelled,
    responses: new Map([
      [USER_IDS.maya, "going"],
      [VIEWER_ID, "going"],
    ]),
    canApplyToFuture: false,
  });

  // --- GitHub ---

  // The open pull request has no discussion thread yet: starting one on its message (as the
  // classic "Discuss" does) makes it one, and the thread's header lists the changed files. A
  // seeded thread would join the switcher's and the S2 specs' thread lists.

  state.pullRequests.set(
    CARD_IDS.pullRequests.open,
    pullRequest(412, {
      title: "Rate limit the sync endpoint with a token bucket",
      status: "open",
      authorLogin: "jonah-k",
      headBranch: "sync-rate-limit",
      review: "approved",
      checks: "passing",
      githubUpdatedAt: timestamp(now - 2 * HOUR),
    }),
  );

  state.pullRequests.set(
    CARD_IDS.pullRequests.merged,
    pullRequest(398, {
      title: "Count unread rooms once in the sidebar",
      status: "merged",
      authorLogin: "sam-r",
      headBranch: "sidebar-unread",
      review: "approved",
      checks: "passing",
      githubUpdatedAt: timestamp(now - DAY),
    }),
  );

  state.pullRequests.set(
    CARD_IDS.pullRequests.draft,
    pullRequest(420, {
      title: "Onboarding checklist in the sidebar",
      status: "draft",
      authorLogin: "theo-b",
      headBranch: "onboarding-checklist",
      review: "changes_requested",
      checks: "pending",
      githubUpdatedAt: timestamp(now - 6 * HOUR),
    }),
  );

  state.pullRequests.set(CARD_IDS.pullRequests.loading, { state: "loading" });
  state.pullRequests.set(CARD_IDS.pullRequests.failed, {
    state: "failed",
    message: "GitHub didn't answer in time. Try again in a minute.",
  });
  state.pullRequests.set(CARD_IDS.pullRequests.hidden, { state: "hidden" });

  // --- Fizzy ---

  state.fizzy.set(CARD_IDS.fizzy.loaded, {
    state: "loaded",
    title: "Import Slack history with per-user opt-in",
    url: "https://app.fizzy.do/897362094/cards/214",
    boardName: "Smartfire",
    status: "column",
    columnName: "In progress",
    assignees: [
      { name: "Jonah Kim", avatarUrl: null },
      { name: "Priya Shah", avatarUrl: null },
      { name: "Sam Rivera", avatarUrl: null },
    ],
    hasMoreAssignees: true,
    tags: ["importer", "launch"],
    stepsTotal: 8,
    stepsCompleted: 5,
    lastActiveAt: timestamp(now - 3 * HOUR),
  });

  state.fizzy.set(CARD_IDS.fizzy.notConnected, { state: "not_connected" });
  state.fizzy.set(CARD_IDS.fizzy.notFound, { state: "not_found" });
  state.fizzy.set(CARD_IDS.fizzy.failed, {
    state: "failed",
    message: "Fizzy is unavailable right now.",
  });
  state.fizzy.set(CARD_IDS.fizzy.loading, { state: "loading" });

  // --- quotes ---

  state.quotes.set(
    CARD_IDS.references.fetched,
    quotedGeneral === undefined
      ? { state: "hidden" }
      : {
          state: "loaded",
          ...quoteOf(quotedGeneral, "general", quotedGeneral.markdownSource ?? ""),
        },
  );

  state.quotes.set(CARD_IDS.references.hidden, { state: "hidden" });
}

// --- the endpoints -----------------------------------------------------------------------------

/** The cards endpoints and the `/__mock/cards` control. */
export interface CardsModule {
  readonly routes: readonly Route[];
  /** `/__mock/cards`: another person votes, a poll closes, a card changes. */
  control(body: Json | undefined): Json;
}

/** The option ids of a `VotePoll` body. */
function optionIdsOf(body: Json | undefined): number[] {
  const raw = field(body, "optionIds");

  if (!Array.isArray(raw)) throw validation("optionIds", "Option ids must be a list");

  return raw.flatMap((value: Json) => (Number.isInteger(value) ? [Number(value)] : []));
}

export interface CalendarAttendance {
  readonly readAttendance: (roomId: number, eventId: number) => MockResponse | null;
  readonly respond: (
    roomId: number,
    eventId: number,
    body: Json | undefined,
  ) => MockResponse | null;
}

export function createCards(ctx: S2Context, calendar?: CalendarAttendance): CardsModule {
  const state = () => cardsOf(ctx.world());

  const pollOr404 = (roomId: number, pollId: number): PollRecord => {
    ctx.roomOr404(roomId);

    const poll = state().polls.get(pollId);

    if (poll === undefined || poll.roomId !== roomId) throw notFound("Poll not found");

    return poll;
  };

  /** Where a message lives (root timeline or a thread), with a setter for its stored copy. */
  const findMessage = (roomId: number, messageId: number) => {
    const record = ctx.roomOr404(roomId);

    const lists = [
      record.messages,
      ...[...ctx.world().threads.values()].flatMap((thread) =>
        thread.roomId === roomId ? [thread.messages] : [],
      ),
    ];

    for (const list of lists) {
      const index = list.findIndex((held) => held.id === messageId);
      const held = list[index];

      if (held !== undefined) {
        return {
          message: held,
          replace: (next: MessageDTO) => {
            list[index] = next;
          },
        };
      }
    }

    return null;
  };

  const topicOf = (held: MessageDTO) =>
    held.threadId === null ? `room:${held.roomId}` : `thread:${held.threadId}`;

  /** Stores the poll's new state on its message and tells its conversation. */
  const publishPoll = (poll: PollRecord) => {
    const located = findMessage(poll.roomId, poll.messageId);
    const sent = pollOf(poll, ctx.now());

    if (located === null) return sent;

    located.replace({ ...located.message, poll: sent });
    ctx.publish([
      {
        topic: topicOf(located.message),
        type: "poll.updated",
        data: { roomId: poll.roomId, threadId: located.message.threadId, poll: sent },
      },
    ]);

    return sent;
  };

  const castBallot = (poll: PollRecord, userId: number, optionIds: readonly number[]) => {
    if (isClosed(poll, ctx.now())) throw validation("poll", "This poll is closed");

    const known = new Set(poll.options.map((option) => option.id));

    if (optionIds.some((id) => !known.has(id))) {
      throw validation("optionIds", "Choose options from this poll");
    }

    if (!poll.multiple && optionIds.length > 1) {
      throw validation("optionIds", "Choose one option");
    }

    if (optionIds.length === 0) {
      poll.ballots.delete(userId);
    } else {
      poll.ballots.set(userId, [...new Set(optionIds)]);
    }

    poll.asOf = nextAsOf(ctx.now(), poll.asOf);

    return publishPoll(poll);
  };

  const results = (poll: PollRecord): PollResults => ({
    poll: pollOf(poll, ctx.now()),
    myOptionIds: [...(poll.ballots.get(VIEWER_ID) ?? [])],
  });

  const vote = (roomId: number, pollId: number, body: Json | undefined) => {
    const poll = pollOr404(roomId, pollId);
    const sent = castBallot(poll, VIEWER_ID, optionIdsOf(body));
    const myOptionIds = [...(poll.ballots.get(VIEWER_ID) ?? [])];

    ctx.publish([
      {
        topic: "user",
        type: "poll.ballot",
        data: {
          pollId,
          messageId: poll.messageId,
          roomId,
          threadId: null,
          myOptionIds,
          asOf: sent.asOf,
        },
      },
    ]);

    return ok({ poll: sent, myOptionIds });
  };

  /** `POST /rooms/:roomId/polls`: the question as the viewer's message, poll attached. */
  const create = (roomId: number, body: Json | undefined) => {
    const record = ctx.roomOr404(roomId);
    const clientMessageId = clientMessageIdOf(body);
    const key = `${roomId}:${VIEWER_ID}:${clientMessageId}`;
    const world = ctx.world();
    const duplicate = world.sentByClientId.get(key);

    if (duplicate !== undefined) return ok(duplicate);

    const question = (stringField(body, "question") ?? "").trim();

    const labels = (stringArrayField(body, "options") ?? [])
      .map((label) => label.trim())
      .filter((label) => label !== "");

    const closesAt = stringField(body, "closesAt");

    if (question === "") throw validation("question", "Question can't be blank");

    if (labels.length < 2) throw validation("options", "Add at least 2 options");

    if (labels.length > 10) throw validation("options", "Use at most 10 options");

    if (labels.some((label) => label.length > 200)) {
      throw validation("options", "Options are at most 200 characters");
    }

    if (closesAt !== null && !(Date.parse(closesAt) > ctx.now())) {
      throw validation("closesAt", "Closes at must be in the future");
    }

    const cards = state();
    const createdAt = timestamp(Math.max(ctx.now(), Date.parse(record.room.updatedAt) + 1));
    const pollId = cards.nextPollId++;

    const poll: PollRecord = {
      id: pollId,
      messageId: world.nextMessageId,
      roomId,
      multiple: booleanField(body, "multiple") ?? false,
      anonymous: booleanField(body, "anonymous") ?? false,
      closesAt,
      closedAt: null,
      options: labels.map((label) => ({ id: cards.nextOptionId++, label })),
      ballots: new Map(),
      asOf: createdAt,
    };

    const built = buildMessage(
      world.nextMessageId++,
      roomId,
      null,
      plainDraft(VIEWER_ID, question, clientMessageId),
      createdAt,
      ctx.mentionables(),
    );

    const posted = { ...built, poll: pollOf(poll, ctx.now()) };

    cards.polls.set(pollId, poll);
    record.messages.push(posted);
    record.room = { ...record.room, updatedAt: createdAt };
    record.membership = { ...record.membership, unreadAt: null, lastReadMessageId: posted.id };
    record.mentionCount = 0;
    world.sentByClientId.set(key, posted);

    ctx.publish([
      { topic: `room:${roomId}`, type: "message.created", data: posted },
      { topic: "user", type: "sidebar.row.upserted", data: ctx.sidebarRow(record) },
    ]);

    return ok(posted, 201);
  };

  const eventOr404 = (roomId: number, eventId: number): EventRecord => {
    ctx.roomOr404(roomId);

    const event = state().events.get(eventId);

    if (event === undefined || event.card.roomId !== roomId) throw notFound("Event not found");

    return event;
  };

  const viewerIsHuman = () => ctx.world().users.get(VIEWER_ID)?.role !== "bot";

  const respondTo = (roomId: number, eventId: number, body: Json | undefined) => {
    const event = eventOr404(roomId, eventId);
    const response = stringField(body, "response");

    if (!viewerIsHuman() || event.card.cancelled)
      throw forbidden("You can't respond to this event");

    if (response !== "going" && response !== "maybe" && response !== "declined") {
      throw validation("response", "Response must be going, maybe or declined");
    }

    event.responses.set(VIEWER_ID, response);

    return ok(attendanceOf(event, viewerIsHuman()));
  };

  /** 404 unless a message in the room (the one named, when named) carries a matching card. */
  const requireCard = (
    roomId: number,
    messageId: number | null,
    matches: (card: MessageCard) => boolean,
  ) => {
    const record = ctx.roomOr404(roomId);

    const candidates =
      messageId === null
        ? record.messages
        : [findMessage(roomId, messageId)?.message].flatMap((held) =>
            held === undefined ? [] : [held],
          );

    if (!candidates.some((held) => held.cards.some(matches))) throw notFound("Card not found");
  };

  const countFetch = (key: string) => {
    const fetches = state().fetches;

    fetches.set(key, (fetches.get(key) ?? 0) + 1);
  };

  /** Whether the message (in the room, on its root timeline) links the pull request. */
  const linksPullRequest = (roomId: number, messageId: number, pullRequestId: number) =>
    findMessage(roomId, messageId)?.message.cards.some(
      (held) => held.kind === "github" && held.data.pullRequestId === pullRequestId,
    ) ?? false;

  /** The pull request's discussion: the thread started on a message that links it. */
  const discussionOf = (roomId: number, pullRequestId: number) =>
    [...ctx.world().threads.values()].find(
      (thread) =>
        thread.roomId === roomId &&
        thread.parentMessageId !== null &&
        linksPullRequest(roomId, thread.parentMessageId, pullRequestId),
    ) ?? null;

  const github = (roomId: number, pullRequestId: number, query: URLSearchParams) => {
    const threadId = Number(query.get("threadId") ?? Number.NaN);
    const messageId = Number(query.get("messageId") ?? Number.NaN);
    const card = state().pullRequests.get(pullRequestId) ?? { state: "hidden" };
    const discussion = discussionOf(roomId, pullRequestId);

    if (Number.isInteger(threadId)) {
      ctx.roomOr404(roomId);

      if (discussion === null || discussion.id !== threadId) throw notFound("Card not found");

      countFetch(`github:${pullRequestId}:thread`);

      return ok(
        card.state === "loaded"
          ? { ...card, discussionThreadId: discussion.id, files: changedFiles() }
          : card,
      );
    }

    if (!Number.isInteger(messageId)) {
      throw validation("messageId", "messageId or threadId is required");
    }

    if (!linksPullRequest(roomId, messageId, pullRequestId)) throw notFound("Card not found");

    countFetch(`github:${pullRequestId}`);

    return ok(
      card.state === "loaded" ? { ...card, discussionThreadId: discussion?.id ?? null } : card,
    );
  };

  const fizzy = (roomId: number, fizzyCardId: number, query: URLSearchParams) => {
    const messageId = Number(query.get("messageId") ?? Number.NaN);

    if (!Number.isInteger(messageId)) throw validation("messageId", "messageId is required");

    requireCard(
      roomId,
      messageId,
      (held) => held.kind === "fizzy" && held.data.fizzyCardId === fizzyCardId,
    );
    countFetch(`fizzy:${fizzyCardId}`);

    return ok(state().fizzy.get(fizzyCardId) ?? { state: "not_found" });
  };

  const quote = (roomId: number, referenceId: number) => {
    requireCard(
      roomId,
      null,
      (held) => held.kind === "quote" && held.data.referenceId === referenceId,
    );
    countFetch(`quote:${referenceId}`);

    return ok(state().quotes.get(referenceId) ?? { state: "hidden" });
  };

  const routes: Route[] = [
    route("POST", /^\/rooms\/(\d+)\/polls$/, (request) => create(firstId(request), request.body)),
    route("GET", /^\/rooms\/(\d+)\/polls\/(\d+)$/, ({ ids: [roomId = 0, pollId = 0] }) =>
      ok(results(pollOr404(roomId, pollId))),
    ),
    route(
      "POST",
      /^\/rooms\/(\d+)\/polls\/(\d+)\/vote$/,
      ({ ids: [roomId = 0, pollId = 0], body }) => vote(roomId, pollId, body),
    ),
    route(
      "GET",
      /^\/rooms\/(\d+)\/events\/(\d+)\/attendance$/,
      ({ ids: [roomId = 0, eventId = 0] }) =>
        calendar?.readAttendance(roomId, eventId) ??
        ok(attendanceOf(eventOr404(roomId, eventId), viewerIsHuman())),
    ),
    route(
      "PUT",
      /^\/rooms\/(\d+)\/events\/(\d+)\/attendance$/,
      ({ ids: [roomId = 0, eventId = 0], body }) =>
        calendar?.respond(roomId, eventId, body) ?? respondTo(roomId, eventId, body),
    ),
    route(
      "GET",
      /^\/rooms\/(\d+)\/github\/pull_requests\/(\d+)\/card$/,
      ({ ids: [roomId = 0, pullRequestId = 0], query }) => github(roomId, pullRequestId, query),
    ),
    route(
      "GET",
      /^\/rooms\/(\d+)\/fizzy\/cards\/(\d+)\/card$/,
      ({ ids: [roomId = 0, cardId = 0], query }) => fizzy(roomId, cardId, query),
    ),
    route(
      "GET",
      /^\/rooms\/(\d+)\/message_links\/(\d+)\/card$/,
      ({ ids: [roomId = 0, referenceId = 0] }) => quote(roomId, referenceId),
    ),
  ];

  /** Republishes a message's cards with a later `asOf` (as a finished fetch or refresh does). */
  const refreshCards = (roomId: number, messageId: number) => {
    const located = findMessage(roomId, messageId);

    if (located === null) throw notFound("Message not found");

    const asOf = nextAsOf(ctx.now(), located.message.cardsAsOf);

    located.replace({ ...located.message, cardsAsOf: asOf });
    ctx.publish([
      {
        topic: topicOf(located.message),
        type: "message.cards",
        data: {
          messageId,
          roomId,
          threadId: located.message.threadId,
          cards: located.message.cards,
          asOf,
        },
      },
    ]);

    return { messageId, asOf };
  };

  const control = (body: Json | undefined): Json => {
    const op = stringField(body, "op");
    const roomId = intField(body, "roomId") ?? CARDS_ROOM_ID;

    switch (op) {
      case "vote": {
        const poll = pollOr404(roomId, intField(body, "pollId") ?? 0);

        return castBallot(poll, intField(body, "userId") ?? USER_IDS.maya, optionIdsOf(body));
      }

      case "close-poll": {
        const poll = pollOr404(roomId, intField(body, "pollId") ?? 0);

        poll.closedAt = timestamp(ctx.now());
        poll.asOf = nextAsOf(ctx.now(), poll.asOf);

        return publishPoll(poll);
      }

      case "github-loaded": {
        const pullRequestId = intField(body, "pullRequestId") ?? 0;

        state().pullRequests.set(
          pullRequestId,
          pullRequest(intField(body, "number") ?? pullRequestId, {
            title: stringField(body, "title") ?? "Fetched pull request",
            status: "open",
            checks: "pending",
            review: "review_required",
          }),
        );

        return refreshCards(roomId, intField(body, "messageId") ?? 0);
      }

      case "refresh":
        return refreshCards(roomId, intField(body, "messageId") ?? 0);
      case "fetches":
        return Object.fromEntries(state().fetches);
      default:
        throw validation("op", "op must be vote, close-poll, github-loaded, refresh or fetches");
    }
  };

  return { routes, control };
}

/** The files a pull request's thread header lists. */
function changedFiles(): GithubPullRequest["files"] {
  return {
    files: [
      { filename: "crates/sync/src/rate_limit.rs", status: "added", additions: 148, deletions: 0 },
      { filename: "crates/sync/src/lib.rs", status: "modified", additions: 12, deletions: 3 },
      { filename: "crates/sync/tests/rate_limit.rs", status: "added", additions: 96, deletions: 0 },
      { filename: "config/sync.toml", status: "modified", additions: 4, deletions: 1 },
    ],
    totalCount: 4,
  };
}

/** For `seedCards`'s callers that want the polls' current state (tests). */
export function pollsOf(world: World, now: number): Poll[] {
  return [...cardsOf(world).polls.values()].map((poll) => pollOf(poll, now));
}
