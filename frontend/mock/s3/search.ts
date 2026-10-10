/**
 * Global search (S3): `GET /search` over the seeded messages with the server's operator parser
 * (`SearchQuery::parse`), the sections above the hits, and the viewer's recent searches. The
 * board posts come from the live world; the non-board work statuses and event search fixtures
 * remain local to this module.
 */
import type { ConversationName } from "../../src/gen/ConversationName.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { RecentSearch } from "../../src/gen/RecentSearch.ts";
import type { RecentSearchList } from "../../src/gen/RecentSearchList.ts";
import type { RoomKind } from "../../src/gen/RoomKind.ts";
import type { SearchChip } from "../../src/gen/SearchChip.ts";
import type { SearchOperator } from "../../src/gen/SearchOperator.ts";
import type { SearchResults } from "../../src/gen/SearchResults.ts";
import type { SearchSection } from "../../src/gen/SearchSection.ts";
import type { SearchSectionRow } from "../../src/gen/SearchSectionRow.ts";
import type { WorkStatus } from "../../src/gen/WorkStatus.ts";
import { noContent, ok, validation } from "../http.ts";
import { type Json, stringField } from "../json.ts";
import { mentionsUser } from "../markdown.ts";
import { VIEWER_TIME_ZONE } from "../s2/composer.ts";
import { type Route, route, type S2Context } from "../s2/context.ts";
import { THREAD_IDS } from "../s2/seed.ts";
import { zonedTime } from "../s2/when.ts";
import { ROOM_IDS, type RoomRecord, timestamp, VIEWER_ID, type World } from "../seed.ts";

/** Messages a page (`Message::PAGE_SIZE`). */
export const SEARCH_PAGE_SIZE = 40;

/** Recent searches kept per person. */
export const MAX_RECENT_SEARCHES = 10;

/** Up to this many rows per section. */
const SECTION_LIMIT = 10;

/** The search-only seeds, for tests and screenshots. */
export const SEARCH_SEED = {
  /** The real seeded board, also listed in the sidebar. */
  boardRoomId: 900,
  boardRoomName: "Roadmap",
  /** Who belongs to the board: its posts are found only for them. */
  boardMemberIds: [VIEWER_ID, 2, 4, 6, 9],
  /** Board posts on it (thread ids, clear of the world's). */
  boardPostIds: { onboardingChecklist: 9001, launchWeek: 9002, pricingPage: 9003 },
  /** Events in seeded rooms. */
  eventIds: { onboardingReview: 9101, launchDryRun: 9102, launchParty: 9103 },
  /** The viewer's recent searches when the world is built, most recent first. */
  recents: ["launch checklist", "from:@maya has:file", "in:#engineering rate limiter"],
} as const;

/** Work statuses on the seeded (non-board) threads. */
const WORK_STATUSES: ReadonlyMap<number, WorkStatus> = new Map([
  [THREAD_IDS.generalActive, "in_progress"],
  [THREAD_IDS.design, "planned"],
  [THREAD_IDS.generalClosed, "done"],
]);

interface EventSeed {
  readonly id: number;
  readonly roomId: number;
  readonly title: string;
  readonly description: string;
  /** From the world's build time; negative is past. */
  readonly hoursFromNow: number;
  readonly cancelled: boolean;
}

const EVENTS: readonly EventSeed[] = [
  {
    id: SEARCH_SEED.eventIds.onboardingReview,
    roomId: ROOM_IDS.design,
    title: "Onboarding v3 review",
    description: "Walk through the new empty states and the sidebar checklist.",
    hoursFromNow: 26,
    cancelled: false,
  },
  {
    id: SEARCH_SEED.eventIds.launchDryRun,
    roomId: ROOM_IDS.launchPlanning,
    title: "Launch dry run",
    description: "Run the announcement end to end, status page included.",
    hoursFromNow: 50,
    cancelled: false,
  },
  {
    id: SEARCH_SEED.eventIds.launchParty,
    roomId: ROOM_IDS.general,
    title: "Launch party",
    description: "Snacks in the kitchen once the launch is out.",
    hoursFromNow: 98,
    cancelled: true,
  },
];

// --- the query parser (`SearchQuery::parse`) ---

/** The filters a query's operators set. */
export interface SearchFilters {
  readonly fromNames: readonly string[];
  readonly inRooms: readonly string[];
  readonly hasValues: readonly string[];
  readonly beforeDate: string | null;
  readonly afterDate: string | null;
  readonly onDate: string | null;
  readonly threadOnly: boolean;
}

/** A parsed query: its operators as chips and filters, and the text left over. */
export interface ParsedSearch {
  readonly raw: string;
  readonly text: string;
  readonly chips: readonly SearchChip[];
  readonly filters: SearchFilters;
}

const OPERATORS =
  /(?:^|[ \t\n\v\f\r])((from_id|in_id|mentions|sort|from|in|has|before|after|on|is):([^ \t\n\v\f\r]+))/gu;

const OPERATOR_NAMES: readonly SearchOperator[] = [
  "from",
  "in",
  "from_id",
  "in_id",
  "mentions",
  "sort",
  "has",
  "before",
  "after",
  "on",
  "is",
];

const HAS_VALUES = ["link", "file", "image", "pin", "mention", "audio", "video"];

const DATE = /^\d{4}-\d{2}-\d{2}$/u;

/** Runs of whitespace collapsed, trimmed (Rails' `squish`). */
export function squish(text: string): string {
  return text.replace(/\s+/gu, " ").trim();
}

function operatorOf(name: string | undefined): SearchOperator | null {
  return OPERATOR_NAMES.find((operator) => operator === name) ?? null;
}

/** A real calendar day written `YYYY-MM-DD`. */
function validDate(value: string): boolean {
  if (!DATE.test(value)) return false;

  const [year = 0, month = 0, day = 0] = value.split("-").map(Number);
  const date = new Date(Date.UTC(year, month - 1, day));

  return (
    date.getUTCFullYear() === year && date.getUTCMonth() === month - 1 && date.getUTCDate() === day
  );
}

/** What an operator's value means, or `null` when it doesn't parse (the token stays text). */
function cleanValue(operator: SearchOperator, value: string): string | null {
  switch (operator) {
    case "from":
    case "in": {
      const prefix = operator === "from" ? "@" : "#";
      const bare = value.startsWith(prefix) ? value.slice(1) : value;
      const trimmed = bare.replace(/[,.!?;:)]+$/u, "");

      return trimmed.trim() === "" ? null : trimmed;
    }

    case "from_id":
    case "in_id":
      return /^[1-9]\d*$/u.test(value) && Number.isSafeInteger(Number(value)) ? value : null;
    case "mentions":
      return value.toLowerCase() === "me" ? "me" : null;
    case "sort":
      return ["newest", "oldest", "relevance"].includes(value.toLowerCase())
        ? value.toLowerCase()
        : null;
    case "has": {
      const lower = value.toLowerCase();

      return HAS_VALUES.includes(lower) ? lower : null;
    }

    case "is":
      return value.toLowerCase() === "thread" ? "true" : null;
    default:
      return validDate(value) ? value : null;
  }
}

/** Parses `raw` as the server does: operators become chips; the rest is the text. */
export function parseSearchQuery(raw: string): ParsedSearch {
  const chips: SearchChip[] = [];
  const fromNames: string[] = [];
  const inRooms: string[] = [];
  const hasValues: string[] = [];
  const dates = new Map<SearchOperator, string>();
  let threadOnly = false;
  let remaining = raw;

  for (const match of raw.matchAll(OPERATORS)) {
    const token = match[1] ?? "";
    const operator = operatorOf(match[2]);
    const value = operator === null ? null : cleanValue(operator, match[3] ?? "");

    if (operator === null || value === null) continue;

    if (operator === "from") fromNames.push(value);

    if (operator === "in") inRooms.push(value);

    if (operator === "has" && !hasValues.includes(value)) hasValues.push(value);

    if (operator === "before" || operator === "after" || operator === "on") {
      dates.set(operator, value);
    }

    if (operator === "is") threadOnly = true;

    chips.push({
      operator,
      value,
      token,
      label: `${operator}: ${value}`,
      removeQuery: squish(raw.replace(token, "")),
    });

    remaining = remaining.replace(token, "");
  }

  return {
    raw,
    text: squish(remaining),
    chips,
    filters: {
      fromNames,
      inRooms,
      hasValues,
      beforeDate: dates.get("before") ?? null,
      afterDate: dates.get("after") ?? null,
      onDate: dates.get("on") ?? null,
      threadOnly,
    },
  };
}

/** The text's words (split on anything that isn't a letter, digit, mark or `_`). */
export function textTokens(text: string): string[] {
  return text.split(/[^\p{L}\p{N}\p{M}_]+/u).filter((token) => token !== "");
}

function hasFilters(filters: SearchFilters): boolean {
  return (
    filters.fromNames.length > 0 ||
    filters.inRooms.length > 0 ||
    filters.hasValues.length > 0 ||
    filters.beforeDate !== null ||
    filters.afterDate !== null ||
    filters.onDate !== null ||
    filters.threadOnly
  );
}

// --- matching ---

/** A light stand-in for FTS5's Porter stemmer: enough that "invites" finds "invited". */
export function stem(word: string): string {
  let lower = word.toLowerCase();

  if (lower.length > 4 && lower.endsWith("ies")) {
    lower = `${lower.slice(0, -3)}y`;
  } else if (lower.endsWith("sses")) {
    lower = lower.slice(0, -2);
  } else if (lower.length > 5 && lower.endsWith("ing")) {
    lower = lower.slice(0, -3);
  } else if (lower.length > 4 && lower.endsWith("ed")) {
    lower = lower.slice(0, -2);
  } else if (lower.length > 3 && lower.endsWith("s") && !lower.endsWith("ss")) {
    lower = lower.slice(0, -1);
  }

  return lower.length > 4 && lower.endsWith("e") ? lower.slice(0, -1) : lower;
}

const ENTITIES = new Map([
  ["&amp;", "&"],
  ["&lt;", "<"],
  ["&gt;", ">"],
  ["&quot;", '"'],
  ["&#39;", "'"],
  ["&nbsp;", " "],
]);

/** A message body's plain text, as the search index holds it. */
export function plainText(html: string): string {
  return html
    .replace(/<[^>]*>/gu, " ")
    .replace(/&(?:amp|lt|gt|quot|#39|nbsp);/gu, (entity) => ENTITIES.get(entity) ?? entity);
}

/** The instant a day in the viewer's zone starts, `offset` days after `date`. */
function midnight(date: string, offset = 0): string {
  const [year = 0, month = 0, day = 0] = date.split("-").map(Number);
  const shifted = new Date(Date.UTC(year, month - 1, day + offset));

  return timestamp(
    zonedTime(
      VIEWER_TIME_ZONE,
      shifted.getUTCFullYear(),
      shifted.getUTCMonth(),
      shifted.getUTCDate(),
      0,
      0,
    ),
  );
}

function containsFolded(haystack: string | null, needle: string): boolean {
  return haystack?.toLowerCase().includes(needle.toLowerCase()) ?? false;
}

// --- the cursor ---

/** The opaque cursor: base64url of `<createdAt>|<id>`. */
export function encodeCursor(message: Pick<MessageDTO, "createdAt" | "id">, rank?: number): string {
  return btoa(`${message.createdAt}|${message.id}${rank === undefined ? "" : `|${rank}`}`)
    .replace(/\+/gu, "-")
    .replace(/\//gu, "_")
    .replace(/=+$/u, "");
}

interface Cursor {
  readonly rank?: number;
  readonly createdAt: string;
  readonly id: number;
}

/** The cursor's position, or `null` when it doesn't decode. */
export function decodeCursor(cursor: string): Cursor | null {
  if (!/^[A-Za-z0-9_-]+$/u.test(cursor)) return null;

  const padded = cursor.replace(/-/gu, "+").replace(/_/gu, "/");
  const text = atob(padded.padEnd(Math.ceil(padded.length / 4) * 4, "="));
  const [createdAt = "", id = "", ...rest] = text.split("|");

  if (rest.length > 1 || !/^\d+$/u.test(id) || Number.isNaN(Date.parse(createdAt))) {
    return null;
  }

  const rank = rest[0] === undefined ? undefined : Number(rest[0]);

  if (rank !== undefined && !Number.isFinite(rank)) return null;

  return rank === undefined ? { createdAt, id: Number(id) } : { createdAt, id: Number(id), rank };
}

function newerFirst(a: { createdAt: string; id: number }, b: { createdAt: string; id: number }) {
  if (a.createdAt !== b.createdAt) return a.createdAt < b.createdAt ? 1 : -1;

  return b.id - a.id;
}

// --- per-world state ---

interface SearchWorld {
  readonly builtAt: number;
  recents: RecentSearch[];
  nextRecentId: number;
}

/** The search module's own state for each world; `reset()` builds a new world, so a new one. */
const states = new WeakMap<World, SearchWorld>();

export interface Search {
  readonly routes: readonly Route[];
}

/** The search endpoints. */
export function createSearch(ctx: S2Context): Search {
  const stateOf = (): SearchWorld => {
    const world = ctx.world();
    const held = states.get(world);

    if (held !== undefined) return held;

    const builtAt = ctx.now();

    const created: SearchWorld = {
      builtAt,
      recents: SEARCH_SEED.recents.map((query, index) => ({
        id: SEARCH_SEED.recents.length - index,
        query,
        searchedAt: timestamp(builtAt - (index + 1) * 3 * 3_600_000),
      })),
      nextRecentId: SEARCH_SEED.recents.length + 1,
    };

    states.set(world, created);

    return created;
  };

  /** The rooms searched: the viewer's. */
  const searchedRooms = (): RoomRecord[] =>
    [...ctx.world().rooms.values()].filter((record) => record.memberIds.includes(VIEWER_ID));

  const matches = (parsed: ParsedSearch): MessageDTO[] => {
    const world = ctx.world();
    const { filters } = parsed;
    const stems = textTokens(parsed.text).map(stem);

    const authors =
      filters.fromNames.length === 0
        ? null
        : new Set(
            [...world.users.values()].flatMap((user) =>
              filters.fromNames.some((name) => containsFolded(user.name, name)) ? [user.id] : [],
            ),
          );

    // `in:` names rooms; a direct message never matches it.
    const rooms = searchedRooms().filter(
      (record) =>
        filters.inRooms.length === 0 ||
        (record.room.kind !== "direct" &&
          filters.inRooms.some((name) => containsFolded(record.room.name, name))),
    );

    const roomIds = new Set(rooms.map((record) => record.room.id));

    const replies = [...world.threads.values()]
      .filter((thread) => roomIds.has(thread.roomId))
      .flatMap((thread) => thread.messages);

    const after = filters.afterDate === null ? null : midnight(filters.afterDate, 1);
    const before = filters.beforeDate === null ? null : midnight(filters.beforeDate);
    const onStart = filters.onDate === null ? null : midnight(filters.onDate);
    const onEnd = filters.onDate === null ? null : midnight(filters.onDate, 1);

    const keep = (message: MessageDTO): boolean => {
      if (message.systemNote) return false;

      const ids = (operator: SearchOperator) =>
        parsed.chips.filter((chip) => chip.operator === operator).map((chip) => Number(chip.value));

      const authorIds = ids("from_id");
      const channelIds = ids("in_id");

      if (authorIds.length > 0 && !authorIds.includes(message.creatorId)) return false;

      if (channelIds.length > 0 && !channelIds.includes(message.roomId)) return false;

      if (
        parsed.chips.some((chip) => chip.operator === "mentions") &&
        !mentionsUser(message.bodyHtml, VIEWER_ID)
      )
        return false;

      if (authors !== null && !authors.has(message.creatorId)) return false;

      if (filters.threadOnly && message.threadId === null) return false;

      if (before !== null && message.createdAt >= before) return false;

      if (after !== null && message.createdAt < after) return false;

      if (onStart !== null && onEnd !== null) {
        if (message.createdAt < onStart || message.createdAt >= onEnd) return false;
      }

      for (const has of filters.hasValues) {
        if (has === "link" && !/href=|https?:\/\//u.test(message.bodyHtml)) return false;

        if (
          has === "file" &&
          message.attachment === null &&
          !message.cards.some((card) => card.kind === "drive")
        ) {
          return false;
        }

        if (has === "image" && !(message.attachment?.contentType.startsWith("image/") ?? false)) {
          return false;
        }

        if (has === "mention" && !message.bodyHtml.includes("application/vnd.campfire.mention"))
          return false;

        if (has === "audio" && !message.attachment?.contentType.startsWith("audio/")) return false;

        if (has === "video" && !message.attachment?.contentType.startsWith("video/")) return false;

        if (has === "pin" && !world.pins.has(message.id)) return false;
      }

      if (stems.length === 0) return true;

      const words = new Set(textTokens(plainText(message.bodyHtml)).map(stem));

      return stems.every((word) => words.has(word));
    };

    return [...rooms.flatMap((record) => record.messages), ...replies]
      .filter(keep)
      .sort(newerFirst);
  };

  const sectionRows = (parsed: ParsedSearch): SearchSection[] => {
    const tokens = textTokens(parsed.text);

    if (
      tokens.length === 0 ||
      parsed.chips.some(
        (chip) =>
          chip.operator === "from_id" ||
          chip.operator === "mentions" ||
          (chip.operator === "has" && ["mention", "audio", "video"].includes(chip.value)),
      )
    )
      return [];

    const { builtAt } = stateOf();
    const world = ctx.world();
    const inRooms = parsed.filters.inRooms;

    const inRoom = (name: string | null, kind: RoomKind = "open") =>
      inRooms.length === 0 ||
      (kind !== "direct" && inRooms.some((wanted) => containsFolded(name, wanted)));

    const named = (...fields: string[]) =>
      tokens.every((token) => fields.some((text) => containsFolded(text, token)));

    const rooms = new Map(searchedRooms().map((record) => [record.room.id, record]));

    const onBoard =
      world.rooms.get(SEARCH_SEED.boardRoomId)?.memberIds.includes(VIEWER_ID) ?? false;

    const boardPosts: SearchSectionRow[] =
      onBoard && inRoom(SEARCH_SEED.boardRoomName, "board")
        ? [...world.threads.values()]
            .filter((post) => post.roomId === SEARCH_SEED.boardRoomId)
            .flatMap((post) =>
              named(post.name)
                ? [
                    {
                      id: post.id,
                      roomId: SEARCH_SEED.boardRoomId,
                      roomKind: "board",
                      title: post.name,
                      time: post.lastActivityAt,
                      workStatus: post.work?.status ?? null,
                      cancelled: false,
                    },
                  ]
                : [],
            )
        : [];

    const workThreads: SearchSectionRow[] = [...world.threads.values()].flatMap((thread) => {
      const record = rooms.get(thread.roomId);
      const workStatus = WORK_STATUSES.get(thread.id) ?? null;

      if (record === undefined || workStatus === null || record.room.kind === "board") return [];

      if (!inRoom(record.room.name, record.room.kind) || !named(thread.name)) return [];

      return [
        {
          id: thread.id,
          roomId: thread.roomId,
          roomKind: record.room.kind,
          title: thread.name,
          time: thread.lastActivityAt,
          workStatus,
          cancelled: false,
        },
      ];
    });

    const events: SearchSectionRow[] = EVENTS.flatMap((event) => {
      const record = rooms.get(event.roomId);

      if (record === undefined || !inRoom(record.room.name, record.room.kind)) return [];

      if (!named(event.title, event.description)) return [];

      return [
        {
          id: event.id,
          roomId: event.roomId,
          roomKind: record.room.kind,
          title: event.title,
          time: timestamp(builtAt + event.hoursFromNow * 3_600_000),
          workStatus: null,
          cancelled: event.cancelled,
        },
      ];
    });

    const sorted = (rows: SearchSectionRow[]) =>
      rows
        .sort((a, b) => (a.time === b.time ? b.id - a.id : a.time < b.time ? 1 : -1))
        .slice(0, SECTION_LIMIT);

    const sections: SearchSection[] = [
      { kind: "board_posts", rows: sorted(boardPosts) },
      { kind: "work_threads", rows: sorted(workThreads) },
      { kind: "events", rows: sorted(events) },
    ];

    const channelIds = parsed.chips
      .filter((chip) => chip.operator === "in_id")
      .map((chip) => Number(chip.value));

    return sections.flatMap((section) => {
      const rows =
        channelIds.length === 0
          ? section.rows
          : section.rows.filter((row) => channelIds.includes(row.roomId));

      return rows.length === 0 ? [] : [{ ...section, rows }];
    });
  };

  /** What each row calls its room and thread, once per pair. */
  const conversationsFor = (
    messages: readonly MessageDTO[],
    sections: readonly SearchSection[],
  ): ConversationName[] => {
    const world = ctx.world();
    const names = new Map<string, ConversationName>();

    const add = (roomId: number, threadId: number | null) => {
      const key = `${roomId}:${threadId ?? ""}`;

      if (names.has(key)) return;

      const record = world.rooms.get(roomId);
      const board = roomId === SEARCH_SEED.boardRoomId;
      const roomKind: RoomKind = board ? "board" : (record?.room.kind ?? "open");

      names.set(key, {
        roomId,
        threadId,
        roomKind,
        roomName: board
          ? SEARCH_SEED.boardRoomName
          : record === undefined
            ? "Unknown room"
            : ctx.displayName(record),
        roomIconName: record?.room.iconName ?? null,
        threadName: threadId === null ? null : (world.threads.get(threadId)?.name ?? null),
      });
    };

    for (const message of messages) add(message.roomId, message.threadId);

    for (const section of sections) {
      for (const row of section.rows) {
        add(row.roomId, section.kind === "events" ? null : row.id);
      }
    }

    return [...names.values()];
  };

  const search = (query: URLSearchParams): SearchResults => {
    const raw = query.get("q") ?? "";
    const before = query.get("before");
    const cursor = before === null || before === "" ? null : decodeCursor(before);

    if (before !== null && before !== "" && cursor === null) {
      throw validation("before", "Before is not a valid search cursor");
    }

    const parsed = parseSearchQuery(raw);

    const blank =
      textTokens(parsed.text).length === 0 &&
      !hasFilters(parsed.filters) &&
      !parsed.chips.some((chip) => ["from_id", "in_id", "mentions"].includes(chip.operator));

    if (blank) {
      return {
        query: squish(raw),
        chips: [],
        messages: [],
        users: [],
        conversations: [],
        nextCursor: null,
        sections: [],
      };
    }

    const sort = parsed.chips.findLast((chip) => chip.operator === "sort")?.value ?? "newest";
    const matched = matches(parsed);
    const terms = textTokens(parsed.text).map(stem);

    const ranks = new Map(
      matched.map((message) => {
        const words = textTokens(plainText(message.bodyHtml)).map(stem);
        const count = words.filter((word) => terms.includes(word)).length;

        return [message.id, count];
      }),
    );

    const order = (a: Cursor, b: Cursor) => {
      if (sort === "oldest") return -newerFirst(a, b);

      if (sort === "relevance" && terms.length > 0) {
        const score = (b.rank ?? ranks.get(b.id) ?? 0) - (a.rank ?? ranks.get(a.id) ?? 0);

        if (score !== 0) return score;
      }

      return newerFirst(a, b);
    };

    matched.sort(order);

    const older =
      cursor === null ? matched : matched.filter((message) => order(message, cursor) > 0);

    const page = older.slice(0, SEARCH_PAGE_SIZE);
    const oldest = page.at(-1);
    const sections = cursor === null ? sectionRows(parsed) : [];

    return {
      query: squish(raw),
      chips: [...parsed.chips],
      messages: [...page].reverse(),
      users: ctx.usersFor(page.map((message) => message.creatorId)),
      conversations: conversationsFor(page, sections),
      nextCursor:
        older.length > SEARCH_PAGE_SIZE && oldest !== undefined
          ? encodeCursor(oldest, sort === "relevance" ? ranks.get(oldest.id) : undefined)
          : null,
      sections,
    };
  };

  const recentList = (): RecentSearchList => ({ searches: [...stateOf().recents] });

  const record = (body: Json | undefined): RecentSearchList => {
    const query = squish(stringField(body, "query") ?? "");

    if (query === "") throw validation("query", "Enter a word to search for.");

    const state = stateOf();
    const held = state.recents.find((search) => search.query === query);
    const searchedAt = timestamp(ctx.now());

    const row: RecentSearch =
      held === undefined ? { id: state.nextRecentId, query, searchedAt } : { ...held, searchedAt };

    if (held === undefined) state.nextRecentId += 1;

    state.recents = [row, ...state.recents.filter((search) => search.query !== query)].slice(
      0,
      MAX_RECENT_SEARCHES,
    );

    return recentList();
  };

  return {
    routes: [
      route("GET", /^\/search$/u, ({ query }) => ok(search(query))),
      route("GET", /^\/search\/recents$/u, () => ok(recentList())),
      route("POST", /^\/search\/recents$/u, ({ body }) => ok(record(body))),
      route("DELETE", /^\/search\/recents$/u, () => {
        stateOf().recents = [];

        return noContent();
      }),
    ],
  };
}
