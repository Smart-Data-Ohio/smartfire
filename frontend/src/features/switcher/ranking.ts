/**
 * The quick switcher's results: instant local items from the sidebar, merged with the server's
 * catalogue (`GET /switcher`: every room, everyone you can message, recent threads) once it
 * arrives, deduplicated, then ranked against the query into sections.
 */
import type { SwitcherRoomKind } from "../../gen/SwitcherRoomKind.ts";
import type { RoomKind, SidebarRow } from "../../store/model.ts";
import type { SidebarState } from "../../store/state.ts";
import { rowPillCount, rowUnread, sidebarSections } from "../sidebar/sections.ts";
import { matchScore, normalizeQuery } from "./match.ts";

/** Something the switcher can open: a room, a person (their DM, made on demand) or a thread. */
export type SwitcherItemKind = "room" | "person" | "thread";

export interface SwitcherItem {
  /** Unique across kinds: `room:12`, `person:5`, `thread:3`. */
  readonly key: string;
  readonly kind: SwitcherItemKind;
  readonly label: string;
  /** A thread's room, shown faint after the label. */
  readonly detail: string | null;
  /** Where it goes; `null` for a person you have no DM with yet. */
  readonly roomId: number | null;
  /** For the glyph: a channel's lock or hash, a group DM's avatars. */
  readonly roomKind: RoomKind | null;
  readonly userId: number | null;
  /** A group DM's other members (avatars). */
  readonly memberIds: readonly number[];
  readonly threadId: number | null;
  readonly unread: boolean;
  /** The sidebar pill's number: mentions, or every unread message in a DM. */
  readonly count: number;
  readonly muted: boolean;
  readonly favorite: boolean;
  /** The room's last activity, for the empty query's fallback order; `null` when unknown. */
  readonly updatedAt: string | null;
}

/** The server's catalogue, as far as ranking needs it. */
export interface RemoteCatalogue {
  readonly rooms: readonly {
    readonly roomId: number;
    readonly name: string;
    readonly kind: SwitcherRoomKind;
    readonly unread: boolean;
    readonly muted: boolean;
    readonly favorite: boolean;
  }[];
  readonly people: readonly { readonly userId: number; readonly directRoomId: number | null }[];
  readonly threads: readonly {
    readonly threadId: number;
    readonly name: string;
    readonly roomId: number;
    readonly roomName: string | null;
  }[];
}

const BLANK = {
  detail: null,
  roomId: null,
  roomKind: null,
  userId: null,
  memberIds: [],
  threadId: null,
  unread: false,
  count: 0,
  muted: false,
  favorite: false,
  updatedAt: null,
} as const satisfies Partial<SwitcherItem>;

function localItem(row: SidebarRow, viewerId: number | null): SwitcherItem {
  const { room, membership } = row;

  const live = {
    ...BLANK,
    label: row.displayName,
    roomId: room.id,
    roomKind: room.kind,
    unread: rowUnread(row),
    count: rowPillCount(row),
    muted: membership.involvement === "muted",
    favorite: membership.favoritePosition !== null,
    updatedAt: room.updatedAt,
  };

  const [only] = row.directMemberIds;
  const oneToOne = room.kind === "direct" && room.name === null && row.directMemberIds.length === 1;

  if (oneToOne && only !== undefined && only !== viewerId) {
    return { ...live, key: `person:${only}`, kind: "person", userId: only };
  }

  const memberIds = room.kind === "direct" ? row.directMemberIds : [];

  return { ...live, key: `room:${room.id}`, kind: "room", memberIds };
}

/** The sidebar's rooms as switcher items: a one-to-one DM is its person. */
export function localItems(sidebar: SidebarState, viewerId: number | null): SwitcherItem[] {
  return sidebarSections(sidebar).flatMap((section) =>
    section.rows.map((row) => localItem(row, viewerId)),
  );
}

const REMOTE_ROOM_KIND = {
  channel: "open",
  dm: "direct",
  group: "direct",
  voice: "voice",
  stage: "stage",
  board: "board",
} as const satisfies Record<SwitcherRoomKind, RoomKind>;

/** The server's catalogue as switcher items (people own their DM rooms). */
export function remoteItems(catalogue: RemoteCatalogue): SwitcherItem[] {
  const personByRoom = new Map(
    catalogue.people.flatMap((person) =>
      person.directRoomId === null ? [] : [[person.directRoomId, person.userId] as const],
    ),
  );

  const roomNames = new Map(catalogue.rooms.map((room) => [room.roomId, room.name]));

  const rooms = catalogue.rooms.flatMap((room): SwitcherItem[] =>
    personByRoom.has(room.roomId)
      ? []
      : [
          {
            ...BLANK,
            key: `room:${room.roomId}`,
            kind: "room",
            label: room.name,
            roomId: room.roomId,
            roomKind: REMOTE_ROOM_KIND[room.kind],
            unread: room.unread,
            muted: room.muted,
            favorite: room.favorite,
          },
        ],
  );

  const roomState = new Map(catalogue.rooms.map((room) => [room.roomId, room]));

  const people = catalogue.people.map((person): SwitcherItem => {
    const room = person.directRoomId === null ? undefined : roomState.get(person.directRoomId);

    return {
      ...BLANK,
      key: `person:${person.userId}`,
      kind: "person",
      label: "",
      roomId: person.directRoomId,
      roomKind: person.directRoomId === null ? null : "direct",
      userId: person.userId,
      unread: room?.unread ?? false,
      muted: room?.muted ?? false,
      favorite: room?.favorite ?? false,
    };
  });

  const threads = catalogue.threads.map(
    (thread): SwitcherItem => ({
      ...BLANK,
      key: `thread:${thread.threadId}`,
      kind: "thread",
      label: thread.name,
      detail: thread.roomName ?? roomNames.get(thread.roomId) ?? null,
      roomId: thread.roomId,
      threadId: thread.threadId,
    }),
  );

  return [...rooms, ...people, ...threads];
}

/**
 * Local and remote items as one list. The sidebar's copy wins (it is live: unread state, the
 * pill), the server fills in what the sidebar lacks (a thread's room, people with no DM yet), and
 * a room shown as its person isn't listed twice. People get their names from `nameOf`.
 */
export function mergeItems(
  local: readonly SwitcherItem[],
  remote: readonly SwitcherItem[],
  nameOf: (userId: number) => string | undefined,
): SwitcherItem[] {
  const byKey = new Map<string, SwitcherItem>();

  for (const item of [...remote, ...local]) {
    const held = byKey.get(item.key);

    byKey.set(item.key, held === undefined ? item : { ...held, ...item });
  }

  const personRooms = new Set(
    [...byKey.values()].flatMap((item) =>
      item.kind === "person" && item.roomId !== null ? [item.roomId] : [],
    ),
  );

  return [...byKey.values()].flatMap((item): SwitcherItem[] => {
    if (item.kind === "room" && item.roomId !== null && personRooms.has(item.roomId)) {
      return [];
    }

    if (item.kind !== "person" || item.userId === null) {
      return [item];
    }

    const label = nameOf(item.userId) ?? item.label;

    return label === "" ? [] : [{ ...item, label }];
  });
}

/** One titled group of results. */
export interface SwitcherSection {
  readonly key: "recent" | "unread" | "channels" | "people" | "threads";
  readonly title: string;
  readonly items: readonly SwitcherItem[];
}

const SECTION_TITLES = {
  recent: "Recent",
  unread: "Unread",
  channels: "Channels",
  people: "People",
  threads: "Threads",
} as const satisfies Record<SwitcherSection["key"], string>;

const PER_SECTION = 8;

const EMPTY_RECENT = 6;

const EMPTY_THREADS = 3;

function sectionOf(item: SwitcherItem): "channels" | "people" | "threads" {
  if (item.kind === "thread") {
    return "threads";
  }

  return item.kind === "person" || item.roomKind === "direct" ? "people" : "channels";
}

function byNewest(left: SwitcherItem, right: SwitcherItem): number {
  return (right.updatedAt ?? "").localeCompare(left.updatedAt ?? "");
}

function section(key: SwitcherSection["key"], items: readonly SwitcherItem[]): SwitcherSection[] {
  return items.length === 0 ? [] : [{ key, title: SECTION_TITLES[key], items }];
}

/**
 * No query: your recent picks (topped up with the latest conversations), then whatever is
 * unread, then recent threads. Nothing shows twice.
 */
function emptyQuerySections(
  items: readonly SwitcherItem[],
  recents: readonly string[],
): SwitcherSection[] {
  const byKey = new Map(items.map((item) => [item.key, item]));
  const picked = recents.flatMap((key) => byKey.get(key) ?? []).slice(0, EMPTY_RECENT);
  const used = new Set(picked.map((item) => item.key));
  const conversations = items.filter((item) => item.kind !== "thread" && item.roomId !== null);

  const unread = conversations
    .filter((item) => item.unread && !item.muted && !used.has(item.key))
    .sort((left, right) => right.count - left.count || byNewest(left, right))
    .slice(0, PER_SECTION);

  for (const item of unread) {
    used.add(item.key);
  }

  const latest = conversations
    .filter((item) => !used.has(item.key))
    .sort(byNewest)
    .slice(0, Math.max(0, EMPTY_RECENT - picked.length));

  const recent = [...picked, ...latest];

  for (const item of latest) {
    used.add(item.key);
  }

  const threads = items
    .filter((item) => item.kind === "thread" && !used.has(item.key))
    .slice(0, EMPTY_THREADS);

  return [
    ...section("recent", recent),
    ...section("unread", unread),
    ...section("threads", threads),
  ];
}

interface Scored {
  readonly item: SwitcherItem;
  readonly score: number;
}

/** A match's score with the nudges: recent picks, unread and favourites up, muted down. */
function nudged(item: SwitcherItem, score: number, recents: readonly string[]): number {
  const recency = recents.indexOf(item.key);

  return (
    score +
    (recency === -1 ? 0 : 30 - Math.min(recency, 10) * 2) +
    (item.unread ? 12 : 0) +
    (item.favorite ? 8 : 0) -
    (item.muted ? 25 : 0)
  );
}

/**
 * The results for `query`: with no query, recent and unread conversations; otherwise every
 * match grouped into Channels, People and Threads, each best first, and the sections ordered by
 * their best match so the top result is always first.
 */
export function rankItems(
  items: readonly SwitcherItem[],
  query: string,
  recents: readonly string[],
): SwitcherSection[] {
  const needle = normalizeQuery(query);

  if (needle === "") {
    return emptyQuerySections(items, recents);
  }

  const groups = new Map<"channels" | "people" | "threads", Scored[]>();

  for (const item of items) {
    const score = matchScore(item.label, needle);

    if (score !== null) {
      const key = sectionOf(item);
      const list = groups.get(key) ?? [];

      list.push({ item, score: nudged(item, score, recents) });
      groups.set(key, list);
    }
  }

  return [...groups.entries()]
    .map(([key, list]) => ({
      key,
      list: list
        .sort(
          (left, right) =>
            right.score - left.score || left.item.label.localeCompare(right.item.label),
        )
        .slice(0, PER_SECTION),
    }))
    .sort((left, right) => (right.list[0]?.score ?? 0) - (left.list[0]?.score ?? 0))
    .map(({ key, list }) => ({
      key,
      title: SECTION_TITLES[key],
      items: list.map((entry) => entry.item),
    }));
}

/** The sections' items in display order, for ↑/↓. */
export function flattenSections(sections: readonly SwitcherSection[]): SwitcherItem[] {
  return sections.flatMap((entry) => entry.items);
}
