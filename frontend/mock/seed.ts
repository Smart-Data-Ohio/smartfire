/**
 * The mock workspace: people, rooms, the viewer's memberships and a realistic message history,
 * all generated from a seed and placed relative to `now`, so day dividers and "5 minutes ago"
 * look real on any day.
 */
import type { Membership } from "../src/gen/Membership.ts";
import type { MessageDTO } from "../src/gen/MessageDTO.ts";
import type { Room } from "../src/gen/Room.ts";
import type { RoomCategory } from "../src/gen/RoomCategory.ts";
import type { RoomKind } from "../src/gen/RoomKind.ts";
import type { User } from "../src/gen/User.ts";
import type { UserPresence } from "../src/gen/UserPresence.ts";
import {
  ANNOUNCEMENTS,
  DESIGN_LINES,
  EMBER_DM,
  ENGINEERING_LINES,
  GENERAL_LINES,
  GROUP_DM_LINES,
  LAUNCH_LINES,
  MAYA_DM_LINES,
  RANDOM_LINES,
  VOICE_LINES,
} from "./corpus.ts";
import { type Mentionable, mentionsUser, renderMarkdown } from "./markdown.ts";
import type { Random } from "./random.ts";
import { emptyS2World, type S2World } from "./s2/model.ts";
import { emptyS3World, type S3World } from "./s3/model.ts";

const MINUTE = 60_000;

const HOUR = 60 * MINUTE;

const DAY = 24 * HOUR;

/** The signed-in person. */
export const VIEWER_ID = 1;

/** The team's AI agent (`role: "bot"`). */
export const BOT_ID = 9;

/** A deactivated person who still authors old messages in #general. */
export const DEACTIVATED_ID = 10;

/** The seeded people, by name. */
export const USER_IDS = {
  riel: 1,
  maya: 2,
  jonah: 3,
  priya: 4,
  sam: 5,
  lucia: 6,
  theo: 7,
  grace: 8,
  ember: BOT_ID,
  dana: DEACTIVATED_ID,
} as const;

/** The seeded rooms, by what they're for in screenshots and tests. */
export const ROOM_IDS = {
  /** #general: ~400 messages over ~10 days, 52 unread (more than a page) with a mention. */
  general: 1,
  /** #design: in the "Launch" category, 4 unread. */
  design: 2,
  /** #engineering: a favourite; long, code-heavy messages; read. */
  engineering: 3,
  /** #random: muted, 3 unread (dimmed but bold). */
  random: 4,
  /** #announcements: six messages, read. */
  announcements: 5,
  /** #quiet: no messages at all. */
  quiet: 6,
  /** #launch-planning: a closed (private) channel in the "Launch" category, 1 unread mention. */
  launchPlanning: 7,
  /** "Lounge": a voice room. */
  lounge: 8,
  /** Direct message with Maya: a favourite, 2 unread. */
  dmMaya: 9,
  /** Direct message with Ember, the bot. */
  dmEmber: 10,
  /** Group direct message with Jonah and Priya. */
  groupDm: 11,
  /** "Town Hall": a stage room the viewer hosts; no messages. */
  townHall: 12,
} as const;

/** The viewer's sidebar category. */
export const CATEGORY_IDS = { launch: 1 } as const;

/**
 * Seeded message ids are stable whatever the seed and clock: a room's root messages are
 * `roomId * MESSAGE_ID_BLOCK + index` (oldest first), so the newest of #general's 400 is 10399.
 */
export const MESSAGE_ID_BLOCK = 10_000;

/** Messages created at run time count up from here, above every seeded id. */
export const FIRST_LIVE_MESSAGE_ID = 1_000_000;

/** The seeded id of a room's root message at `index` (oldest first). */
export function seededMessageId(roomId: number, index: number): number {
  return roomId * MESSAGE_ID_BLOCK + index;
}

/** A room as the mock keeps it: the DTO plus what the viewer's sidebar row and detail need. */
export interface RoomRecord {
  room: Room;
  /** Everyone in the room, in membership order (oldest first), viewer included. */
  memberIds: number[];
  /** The viewer's membership. */
  membership: Membership;
  /** The root timeline, ascending by id (and so by `createdAt`). */
  readonly messages: MessageDTO[];
  /** The viewer's unread mentions here. */
  mentionCount: number;
}

/** Everything the mock knows. Mutated in place by the server. */
export interface World extends S2World, S3World {
  readonly users: Map<number, User>;
  readonly presence: Map<number, UserPresence>;
  readonly rooms: Map<number, RoomRecord>;
  readonly categories: readonly RoomCategory[];
  nextMessageId: number;
}

/** RFC 3339 with milliseconds and `Z`, the server's fixed form. */
export function timestamp(ms: number): string {
  return new Date(ms).toISOString();
}

/** A random UUID-formatted id from the seeded PRNG (the client's ids are UUID v7). */
export function seededUuid(random: Random): string {
  const hex = Array.from({ length: 32 }, () => random.int(0, 15).toString(16)).join("");

  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-7${hex.slice(13, 16)}-a${hex.slice(17, 20)}-${hex.slice(20)}`;
}

interface PersonSeed {
  readonly id: number;
  readonly name: string;
  readonly role: User["role"];
  readonly status: User["status"];
  readonly bio: string | null;
  readonly joinedDaysAgo: number;
  readonly presence: UserPresence["presence"];
  readonly statusText: string | null;
  readonly customStatus: { readonly emoji: string; readonly text: string } | null;
}

const PEOPLE: readonly PersonSeed[] = [
  {
    id: USER_IDS.riel,
    name: "Riel St. Amand",
    role: "administrator",
    status: "active",
    bio: "Building Smartfire.",
    joinedDaysAgo: 400,
    presence: "online",
    statusText: null,
    customStatus: null,
  },
  {
    id: USER_IDS.maya,
    name: "Maya Okafor",
    role: "member",
    status: "active",
    bio: "Customer success lead.",
    joinedDaysAgo: 380,
    presence: "online",
    statusText: null,
    customStatus: null,
  },
  {
    id: USER_IDS.jonah,
    name: "Jonah Lindqvist",
    role: "member",
    status: "active",
    bio: "Backend, databases, the occasional sourdough.",
    joinedDaysAgo: 360,
    presence: "idle",
    statusText: null,
    customStatus: null,
  },
  {
    id: USER_IDS.priya,
    name: "Priya Raman",
    role: "administrator",
    status: "active",
    bio: "Engineering manager.",
    joinedDaysAgo: 340,
    presence: "dnd",
    statusText: "🎧 Heads down until 3pm",
    customStatus: { emoji: "🎧", text: "Heads down until 3pm" },
  },
  {
    id: USER_IDS.sam,
    name: "Sam Whitfield",
    role: "member",
    status: "active",
    bio: null,
    joinedDaysAgo: 300,
    presence: "offline",
    statusText: null,
    customStatus: null,
  },
  {
    id: USER_IDS.lucia,
    name: "Lucía Fernández",
    role: "member",
    status: "active",
    bio: "Product design.",
    joinedDaysAgo: 250,
    presence: "offline",
    statusText: "🌴 Out until Monday",
    customStatus: { emoji: "🌴", text: "Out until Monday" },
  },
  {
    id: USER_IDS.theo,
    name: "Theo Nakamura",
    role: "member",
    status: "active",
    bio: "Front end and motion.",
    joinedDaysAgo: 120,
    presence: "online",
    statusText: null,
    customStatus: null,
  },
  {
    id: USER_IDS.grace,
    name: "Grace Adeyemi",
    role: "member",
    status: "active",
    bio: "Marketing.",
    joinedDaysAgo: 30,
    presence: "offline",
    statusText: null,
    customStatus: null,
  },
  {
    id: USER_IDS.ember,
    name: "Ember",
    role: "bot",
    status: "active",
    bio: "The team's AI agent. Mention @Ember in any room or send a direct message.",
    joinedDaysAgo: 90,
    presence: "online",
    statusText: null,
    customStatus: null,
  },
  {
    id: USER_IDS.dana,
    name: "Dana Kowalski",
    role: "member",
    status: "deactivated",
    bio: null,
    joinedDaysAgo: 390,
    presence: "offline",
    statusText: null,
    customStatus: null,
  },
];

/** Avatar paths: the Vite plugin serves a picture for these two and 404s the rest (initials). */
export const USERS_WITH_PHOTOS: ReadonlySet<number> = new Set([USER_IDS.maya, BOT_ID]);

function seedUsers(now: number): Map<number, User> {
  const users = new Map<number, User>();

  for (const person of PEOPLE) {
    users.set(person.id, {
      id: person.id,
      name: person.name,
      role: person.role,
      status: person.status,
      bio: person.bio,
      avatarUrl: `/users/${person.id}/avatar`,
      hasAvatar: USERS_WITH_PHOTOS.has(person.id),
      customStatus:
        person.customStatus === null
          ? null
          : {
              emoji: person.customStatus.emoji,
              text: person.customStatus.text,
              expiresAt: timestamp(now + 3 * DAY),
            },
      avatarIcon: null,
      agent:
        person.role === "bot"
          ? // One agent per bot; the mock reuses the bot user's id as the agent id.
            { agentId: person.id, kind: "workspace", status: "idle", suspended: false }
          : null,
      createdAt: timestamp(now - person.joinedDaysAgo * DAY),
    });
  }

  return users;
}

function seedPresence(): Map<number, UserPresence> {
  const presence = new Map<number, UserPresence>();

  for (const person of PEOPLE) {
    if (person.role === "bot" || person.status !== "active") continue;
    presence.set(person.id, {
      userId: person.id,
      presence: person.presence,
      statusText: person.statusText,
    });
  }

  return presence;
}

const HUMANS = [1, 2, 3, 4, 5, 6, 7, 8];

interface RoomSeed {
  readonly id: number;
  readonly kind: RoomKind;
  readonly name: string | null;
  readonly memberIds: readonly number[];
  readonly createdDaysAgo: number;
  /** How many root messages to generate (ignored for scripted rooms). */
  readonly count: number;
  readonly lines: readonly string[];
  /** Chance that a gap between messages is a break between conversations. */
  readonly breakChance: number;
  /** How long ago the newest message was posted. */
  readonly lastAgoMs: number;
  readonly unread: number;
  /** How many of the unread messages mention the viewer. */
  readonly unreadMentions: number;
  readonly involvement: Membership["involvement"];
  readonly categoryId: number | null;
  readonly favoritePosition: number | null;
}

const ROOMS: readonly RoomSeed[] = [
  {
    id: ROOM_IDS.general,
    kind: "open",
    name: "general",
    memberIds: [...HUMANS, BOT_ID],
    createdDaysAgo: 400,
    count: 400,
    lines: GENERAL_LINES,
    breakChance: 0.14,
    lastAgoMs: 4 * MINUTE,
    unread: 52,
    unreadMentions: 1,
    involvement: "everything",
    categoryId: null,
    favoritePosition: null,
  },
  {
    id: ROOM_IDS.design,
    kind: "open",
    name: "design",
    memberIds: [1, 2, 6, 7, 8],
    createdDaysAgo: 300,
    count: 70,
    lines: DESIGN_LINES,
    breakChance: 0.2,
    lastAgoMs: 40 * MINUTE,
    unread: 4,
    unreadMentions: 0,
    involvement: "mentions",
    categoryId: CATEGORY_IDS.launch,
    favoritePosition: null,
  },
  {
    id: ROOM_IDS.engineering,
    kind: "open",
    name: "engineering",
    memberIds: [1, 3, 4, 5, 7, BOT_ID],
    createdDaysAgo: 390,
    count: 140,
    lines: ENGINEERING_LINES,
    breakChance: 0.18,
    lastAgoMs: 25 * MINUTE,
    unread: 0,
    unreadMentions: 0,
    involvement: "mentions",
    categoryId: null,
    favoritePosition: 1,
  },
  {
    id: ROOM_IDS.random,
    kind: "open",
    name: "random",
    memberIds: HUMANS,
    createdDaysAgo: 400,
    count: 80,
    lines: RANDOM_LINES,
    breakChance: 0.3,
    lastAgoMs: 2 * HOUR,
    unread: 3,
    unreadMentions: 0,
    involvement: "muted",
    categoryId: null,
    favoritePosition: null,
  },
  {
    id: ROOM_IDS.announcements,
    kind: "open",
    name: "announcements",
    memberIds: [...HUMANS, BOT_ID],
    createdDaysAgo: 400,
    count: ANNOUNCEMENTS.length,
    lines: ANNOUNCEMENTS,
    breakChance: 1,
    lastAgoMs: 26 * HOUR,
    unread: 0,
    unreadMentions: 0,
    involvement: "everything",
    categoryId: null,
    favoritePosition: null,
  },
  {
    id: ROOM_IDS.quiet,
    kind: "open",
    name: "quiet",
    memberIds: [1, 5],
    createdDaysAgo: 2,
    count: 0,
    lines: [],
    breakChance: 0,
    lastAgoMs: 0,
    unread: 0,
    unreadMentions: 0,
    involvement: "mentions",
    categoryId: null,
    favoritePosition: null,
  },
  {
    id: ROOM_IDS.launchPlanning,
    kind: "closed",
    name: "launch-planning",
    memberIds: [1, 2, 3, 6],
    createdDaysAgo: 40,
    count: 30,
    lines: LAUNCH_LINES,
    breakChance: 0.25,
    lastAgoMs: 15 * MINUTE,
    unread: 1,
    unreadMentions: 1,
    involvement: "mentions",
    categoryId: CATEGORY_IDS.launch,
    favoritePosition: null,
  },
  {
    id: ROOM_IDS.lounge,
    kind: "voice",
    name: "Lounge",
    memberIds: HUMANS,
    createdDaysAgo: 200,
    count: VOICE_LINES.length,
    lines: VOICE_LINES,
    breakChance: 1,
    lastAgoMs: 5 * HOUR,
    unread: 0,
    unreadMentions: 0,
    involvement: "mentions",
    categoryId: null,
    favoritePosition: null,
  },
  {
    id: ROOM_IDS.dmMaya,
    kind: "direct",
    name: null,
    memberIds: [1, 2],
    createdDaysAgo: 200,
    count: 24,
    lines: MAYA_DM_LINES,
    breakChance: 0.25,
    lastAgoMs: 9 * MINUTE,
    unread: 2,
    unreadMentions: 0,
    involvement: "everything",
    categoryId: null,
    favoritePosition: 2,
  },
  {
    id: ROOM_IDS.dmEmber,
    kind: "direct",
    name: null,
    memberIds: [1, BOT_ID],
    createdDaysAgo: 60,
    count: EMBER_DM.length,
    lines: [],
    breakChance: 0.3,
    lastAgoMs: 3 * HOUR,
    unread: 0,
    unreadMentions: 0,
    involvement: "everything",
    categoryId: null,
    favoritePosition: null,
  },
  {
    id: ROOM_IDS.groupDm,
    kind: "direct",
    name: null,
    memberIds: [1, 3, 4],
    createdDaysAgo: 100,
    count: 14,
    lines: GROUP_DM_LINES,
    breakChance: 0.3,
    lastAgoMs: 28 * HOUR,
    unread: 0,
    unreadMentions: 0,
    involvement: "everything",
    categoryId: null,
    favoritePosition: null,
  },
  {
    id: ROOM_IDS.townHall,
    kind: "stage",
    name: "Town Hall",
    memberIds: HUMANS,
    createdDaysAgo: 30,
    count: 0,
    lines: [],
    breakChance: 0,
    lastAgoMs: 0,
    unread: 0,
    unreadMentions: 0,
    involvement: "mentions",
    categoryId: null,
    favoritePosition: null,
  },
];

/** A message before it has an id: ids are handed out in global time order afterwards. */
interface Draft {
  readonly roomId: number;
  creatorId: number;
  markdown: string;
  readonly createdAt: number;
  readonly editedAfterMs: number | null;
}

/** UTC hours 3 to 12 are the night in North America: no one posts then. */
function isNight(ms: number): boolean {
  const hour = new Date(ms).getUTCHours();

  return hour >= 3 && hour < 12;
}

/** `count` timestamps ending `lastAgoMs` before now, oldest first, in bursts with breaks. */
function messageTimes(now: number, seed: RoomSeed, random: Random): number[] {
  const times: number[] = [];
  let at = now - seed.lastAgoMs;

  for (let index = 0; index < seed.count; index++) {
    times.push(at);

    const gap = random.chance(seed.breakChance)
      ? random.int(25 * MINUTE, 5 * HOUR)
      : random.int(15_000, 4 * MINUTE);

    at -= gap;

    if (isNight(at)) {
      // Skip back over the night to the previous evening, 10pm to midnight Eastern.
      const evening = new Date(at);

      evening.setUTCHours(random.int(0, 2), random.int(0, 59), random.int(0, 59), 0);
      at = evening.getTime();
    }
  }

  return times.reverse();
}

function firstName(name: string): string {
  return name.split(/\s+/)[0] ?? name;
}

/** Fills `{name}` / `{@name}` with another member of the room. */
function fillLine(
  line: string,
  authorId: number,
  memberIds: readonly number[],
  users: Map<number, User>,
  random: Random,
): string {
  const others = memberIds.filter((id) => id !== authorId && id !== BOT_ID);

  return line.replace(/\{(@?)name\}/g, (_match, at: string) => {
    const other = users.get(others.length > 0 ? random.pick(others) : VIEWER_ID);
    const name = other === undefined ? "everyone" : firstName(other.name);

    return `${at}${name}`;
  });
}

function pickLine(lines: readonly string[], previous: string | null, random: Random): string {
  let line = random.pick(lines);

  for (let attempt = 0; attempt < 3 && line === previous; attempt++) {
    line = random.pick(lines);
  }

  return line;
}

const MENTION_LINES = [
  "@Riel can you take a look at this when you get a sec?",
  "@Riel, the numbers you asked for are in the doc 👍",
  "Looping in @Riel for a decision on this one",
];

function draftRoom(now: number, seed: RoomSeed, users: Map<number, User>, random: Random): Draft[] {
  const times = messageTimes(now, seed, random);
  const authors = seed.memberIds.filter((id) => id !== BOT_ID);
  const drafts: Draft[] = [];
  let author = random.pick(authors);
  let previous: string | null = null;

  times.forEach((createdAt, index) => {
    let creatorId: number;
    let markdown: string;

    if (seed.id === ROOM_IDS.dmEmber) {
      const scripted = EMBER_DM[index];

      creatorId = scripted?.bot === true ? BOT_ID : VIEWER_ID;
      markdown = scripted?.text ?? "";
    } else if (seed.lines === ANNOUNCEMENTS || seed.lines === VOICE_LINES) {
      creatorId = seed.lines === ANNOUNCEMENTS ? USER_IDS.priya : random.pick(authors);
      markdown = seed.lines[index] ?? "";
    } else {
      if (!random.chance(0.55)) author = random.pick(authors);

      const oldEnough = createdAt < now - 6 * DAY;

      creatorId =
        seed.id === ROOM_IDS.general && oldEnough && random.chance(0.05) ? DEACTIVATED_ID : author;

      const line = pickLine(seed.lines, previous, random);

      previous = line;
      markdown = fillLine(line, creatorId, seed.memberIds, users, random);
    }

    drafts.push({
      roomId: seed.id,
      creatorId,
      markdown,
      createdAt,
      editedAfterMs:
        seed.lines !== ANNOUNCEMENTS && random.chance(0.04)
          ? random.int(MINUTE, 10 * MINUTE)
          : null,
    });
  });

  if (seed.id === ROOM_IDS.engineering && drafts.length > 20) {
    // Ember chimes in a couple of times with a CI summary.
    for (const index of [drafts.length - 12, drafts.length - 60]) {
      const draft = drafts[index];

      if (draft === undefined) continue;
      draft.creatorId = BOT_ID;
      draft.markdown =
        "**CI summary:** 214 tests passed, 0 failed. The slowest job was `rust-port (shard 3)` at 6m 12s.";
    }
  }

  // The unread tail is from other people, and some of it mentions the viewer.
  const tail = drafts.slice(drafts.length - seed.unread);

  for (const draft of tail) {
    if (draft.creatorId !== VIEWER_ID) continue;
    draft.creatorId = seed.memberIds.find((id) => id !== VIEWER_ID && id !== BOT_ID) ?? BOT_ID;
  }

  for (let mention = 0; mention < seed.unreadMentions; mention++) {
    const draft = tail[Math.floor(((mention + 1) * tail.length) / (seed.unreadMentions + 1))];

    if (draft !== undefined) draft.markdown = MENTION_LINES[mention % MENTION_LINES.length] ?? "";
  }

  return drafts;
}

function mentionables(users: Map<number, User>): Mentionable[] {
  const people: Mentionable[] = [];

  for (const user of users.values()) {
    if (user.status === "active") people.push({ id: user.id, name: user.name });
  }

  return people;
}

/** The viewer's membership in a room, read up to just before its unread tail. */
function viewerMembership(seed: RoomSeed, messages: readonly MessageDTO[]): Membership {
  const firstUnread = seed.unread > 0 ? messages[messages.length - seed.unread] : undefined;
  const lastRead = messages[messages.length - seed.unread - 1];

  return {
    id: 100 + seed.id,
    roomId: seed.id,
    userId: VIEWER_ID,
    involvement: seed.involvement,
    unreadAt: firstUnread === undefined ? null : firstUnread.createdAt,
    lastReadMessageId: lastRead === undefined ? null : lastRead.id,
    roomCategoryId: seed.categoryId,
    favoritePosition: seed.favoritePosition,
    // Every stage membership has a role; the viewer hosts the seeded stage.
    stageRole: seed.kind === "stage" ? "host" : null,
  };
}

/** Builds the whole workspace. Same `now` and same random sequence, same workspace. */
export function seedWorld(now: number, random: Random): World {
  const users = seedUsers(now);
  const people = mentionables(users);
  const drafts: Draft[] = [];

  for (const seed of ROOMS) drafts.push(...draftRoom(now, seed, users, random));

  drafts.sort((a, b) => a.createdAt - b.createdAt || a.roomId - b.roomId);

  const messagesByRoom = new Map<number, MessageDTO[]>();

  for (const draft of drafts) {
    const createdAt = timestamp(draft.createdAt);

    const editedAt =
      draft.editedAfterMs === null
        ? null
        : timestamp(Math.min(now - 1000, draft.createdAt + draft.editedAfterMs));

    const list = messagesByRoom.get(draft.roomId) ?? [];

    list.push({
      id: seededMessageId(draft.roomId, list.length),
      roomId: draft.roomId,
      threadId: null,
      creatorId: draft.creatorId,
      clientMessageId: seededUuid(random),
      bodyHtml: renderMarkdown(draft.markdown, people),
      markdownSource: draft.markdown,
      systemNote: false,
      action: false,
      streaming: false,
      embedsSuppressed: false,
      replyToMessageId: null,
      forwardedFromMessageId: null,
      forwardedAt: null,
      forwardNote: null,
      editedAt,
      attachment: null,
      reactions: [],
      boosts: [],
      pinned: false,
      thread: null,
      poll: null,
      cards: [],
      cardsAsOf: createdAt,
      steps: [],
      createdAt,
      updatedAt: editedAt ?? createdAt,
    });
    messagesByRoom.set(draft.roomId, list);
  }

  const rooms = new Map<number, RoomRecord>();

  for (const seed of ROOMS) {
    const messages = messagesByRoom.get(seed.id) ?? [];
    const membership = viewerMembership(seed, messages);
    const createdAt = timestamp(now - seed.createdDaysAgo * DAY);
    const newest = messages.at(-1);
    let mentionCount = 0;

    for (const message of messages.slice(messages.length - seed.unread)) {
      if (seed.unread > 0 && mentionsUser(message.bodyHtml, VIEWER_ID)) mentionCount++;
    }

    rooms.set(seed.id, {
      room: {
        id: seed.id,
        kind: seed.kind,
        name: seed.name,
        iconName: null,
        creatorId: seed.kind === "direct" ? VIEWER_ID : USER_IDS.priya,
        createdAt,
        updatedAt: newest === undefined ? createdAt : newest.createdAt,
      },
      memberIds: [...seed.memberIds],
      membership,
      messages,
      mentionCount,
    });
  }

  return {
    ...emptyS2World(),
    ...emptyS3World(),
    users,
    presence: seedPresence(),
    rooms,
    categories: [{ id: CATEGORY_IDS.launch, name: "Launch", collapsed: false, position: 0 }],
    nextMessageId: FIRST_LIVE_MESSAGE_ID,
  };
}
