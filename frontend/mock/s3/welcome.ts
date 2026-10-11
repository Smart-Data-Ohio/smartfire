/**
 * The welcome card and server guide (`crates/api/src/welcome.rs`): administrators read the
 * configuration at `/admin/welcome` and `PATCH` one section (the welcome or the guide) at a time;
 * everyone reads `/welcome`, which keeps only the rooms the viewer belongs to (Open) or may join
 * (an alive open room: Join). An administrator's copy shows a saved room they can't reach as a
 * `keep` token, never its id or name. The server's rules apply on a write (lengths, counts, rooms
 * they can reach or tokens for rooms already saved), and every save publishes
 * `workspace.updated`. The configuration belongs to the world, so a reset clears it.
 * `/__mock/welcome` stands in for another administrator's save.
 */
import type { GuideEntry } from "../../src/gen/GuideEntry.ts";
import type { GuideResource } from "../../src/gen/GuideResource.ts";
import type { GuideResourceEdit } from "../../src/gen/GuideResourceEdit.ts";
import type { WelcomeAdmin } from "../../src/gen/WelcomeAdmin.ts";
import type { WelcomeRoom } from "../../src/gen/WelcomeRoom.ts";
import type { WelcomeRoomCard } from "../../src/gen/WelcomeRoomCard.ts";
import type { WelcomeRoomChoice } from "../../src/gen/WelcomeRoomChoice.ts";
import type { WelcomeRoomEdit } from "../../src/gen/WelcomeRoomEdit.ts";
import type { WelcomeSettings } from "../../src/gen/WelcomeSettings.ts";
import type { WorkspaceBranding } from "../../src/gen/WorkspaceBranding.ts";
import type { WorkspaceWelcome } from "../../src/gen/WorkspaceWelcome.ts";
import { forbidden, type MockResponse, ok, validation } from "../http.ts";
import { field, isNumber, isRecord, isString, type Json } from "../json.ts";
import { renderMarkdown } from "../markdown.ts";
import { type Route, route, type S2Context } from "../s2/context.ts";
import { JOINABLE_OPEN_ROOM, VIEWER_ID, type World } from "../seed.ts";

const MESSAGE_MAX = 1000;

const ROOMS_MAX = 5;

const DESCRIPTION_MAX = 100;

const GUIDE_MAX = 10;

const TITLE_MAX = 80;

const BODY_MAX = 1000;

const EMOJI_MAX = 16;

const EMPTY: WelcomeSettings = { message: "", rooms: [], guide: [] };

/** An open room outside the world (the join catalog's), by id. */
export type CatalogRoom = (roomId: number) => { readonly id: number; readonly name: string } | null;

export interface WelcomeModule {
  readonly routes: readonly Route[];
  /** `/__mock/welcome`: replaces the configuration unchecked and tells every client. */
  control(body: Json | undefined): WelcomeSettings;
}

interface Reachable {
  readonly name: string;
  readonly joined: boolean;
}

const length = (text: string) => [...text].length;

// biome-ignore lint/suspicious/noControlCharactersInRegex: control characters are what it finds
const CONTROL = /[\u0000-\u001f\u007f-\u009f]/u;

// biome-ignore lint/suspicious/noControlCharactersInRegex: control characters are what it finds
const CONTROL_EXCEPT_BREAKS = /[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f-\u009f]/u;

// biome-ignore lint/suspicious/noControlCharactersInRegex: ASCII is what a single emoji can't hold
const PLAIN = /[\u0000-\u007f\s]/u;

/** The settings a body carries, or `null` for one that isn't settings-shaped at all. */
function settingsOf(body: Json | undefined): WelcomeSettings | null {
  const message = field(body, "message");
  const rooms = field(body, "rooms");
  const guide = field(body, "guide");

  if (!isString(message) || !Array.isArray(rooms) || !Array.isArray(guide)) {
    return null;
  }

  const parsedRooms: WelcomeRoom[] = [];

  for (const room of rooms) {
    const roomId = field(room, "roomId");
    const emoji = field(room, "emoji") ?? null;
    const description = field(room, "description");

    if (!isNumber(roomId) || !isString(description) || !(emoji === null || isString(emoji))) {
      return null;
    }

    parsedRooms.push({ roomId, emoji, description });
  }

  const parsedGuide: GuideResource[] = [];

  for (const entry of guide) {
    const title = field(entry, "title");
    const emoji = field(entry, "emoji") ?? null;
    const roomId = field(entry, "roomId") ?? null;
    const text = field(entry, "body") ?? null;

    if (
      !isString(title) ||
      !(emoji === null || isString(emoji)) ||
      !(roomId === null || isNumber(roomId)) ||
      !(text === null || isString(text))
    ) {
      return null;
    }

    parsedGuide.push({ title, emoji, roomId, body: text });
  }

  return { message, rooms: parsedRooms, guide: parsedGuide };
}

const nullableString = (value: Json | undefined): value is string | null | undefined =>
  value === undefined || value === null || isString(value);

const nullableNumber = (value: Json | undefined): value is number | null | undefined =>
  value === undefined || value === null || isNumber(value);

/** The sections a `PATCH` names, or `null` for a body that isn't update-shaped. */
interface Update {
  message?: string;
  rooms?: readonly WelcomeRoomEdit[];
  guide?: readonly GuideResourceEdit[];
}

function updateOf(body: Json | undefined): Update | null {
  if (!isRecord(body)) return null;

  const message = field(body, "message");
  const rooms = field(body, "rooms");
  const guide = field(body, "guide");

  if (message !== undefined && !isString(message)) return null;

  if (rooms !== undefined && !Array.isArray(rooms)) return null;

  if (guide !== undefined && !Array.isArray(guide)) return null;

  const parsedRooms: WelcomeRoomEdit[] = [];

  for (const room of rooms ?? []) {
    const roomId = field(room, "roomId");
    const keep = field(room, "keep");
    const emoji = field(room, "emoji");
    const description = field(room, "description");

    if (
      !nullableNumber(roomId) ||
      !nullableString(keep) ||
      !nullableString(emoji) ||
      !isString(description)
    ) {
      return null;
    }

    parsedRooms.push({
      roomId: roomId ?? null,
      keep: keep ?? null,
      emoji: emoji ?? null,
      description,
    });
  }

  const parsedGuide: GuideResourceEdit[] = [];

  for (const entry of guide ?? []) {
    const title = field(entry, "title");
    const emoji = field(entry, "emoji");
    const roomId = field(entry, "roomId");
    const keep = field(entry, "keep");
    const text = field(entry, "body");

    if (
      !isString(title) ||
      !nullableString(emoji) ||
      !nullableNumber(roomId) ||
      !nullableString(keep) ||
      !nullableString(text)
    ) {
      return null;
    }

    parsedGuide.push({
      title,
      emoji: emoji ?? null,
      roomId: roomId ?? null,
      keep: keep ?? null,
      body: text ?? null,
    });
  }

  const update: Update = {};

  if (message !== undefined) update.message = message;

  if (rooms !== undefined) update.rooms = parsedRooms;

  if (guide !== undefined) update.guide = parsedGuide;

  return update;
}

/** A trimmed emoji, `null` for a blank one; throws for anything but a short non-ASCII run. */
function emojiOf(emoji: string | null, field: "rooms" | "guide", at: number): string | null {
  const trimmed = emoji?.trim() ?? "";

  if (trimmed === "") return null;

  if (length(trimmed) > EMOJI_MAX || PLAIN.test(trimmed)) {
    throw validation(field, `#${at}: emoji must be a single emoji`);
  }

  return trimmed;
}

/** The server's `normalize`: trimmed, blanks made `null`, the first broken rule thrown. */
function normalize(settings: WelcomeSettings): WelcomeSettings {
  const message = settings.message.trim();

  if (length(message) > MESSAGE_MAX) {
    throw validation("message", `must be ${MESSAGE_MAX} characters or fewer`);
  }

  if (CONTROL_EXCEPT_BREAKS.test(message)) {
    throw validation("message", "can't contain control characters");
  }

  if (settings.rooms.length > ROOMS_MAX) {
    throw validation("rooms", `can suggest at most ${ROOMS_MAX} rooms`);
  }

  const seen = new Set<number>();

  const rooms = settings.rooms.map((room, index) => {
    const at = index + 1;
    const description = room.description.trim();

    if (seen.has(room.roomId)) throw validation("rooms", `#${at}: suggests the same room twice`);

    seen.add(room.roomId);

    if (length(description) > DESCRIPTION_MAX) {
      throw validation(
        "rooms",
        `#${at}: description must be ${DESCRIPTION_MAX} characters or fewer`,
      );
    }

    if (CONTROL.test(description)) {
      throw validation("rooms", `#${at}: description must be one line of text`);
    }

    return { roomId: room.roomId, emoji: emojiOf(room.emoji, "rooms", at), description };
  });

  if (settings.guide.length > GUIDE_MAX) {
    throw validation("guide", `can hold at most ${GUIDE_MAX} resources`);
  }

  const guide = settings.guide.map((entry, index) => {
    const at = index + 1;
    const title = entry.title.trim();
    const text = entry.body?.trim() ?? "";

    if (title === "") throw validation("guide", `#${at}: title can't be blank`);

    if (length(title) > TITLE_MAX || CONTROL.test(title)) {
      throw validation(
        "guide",
        `#${at}: title must be one line of ${TITLE_MAX} characters or fewer`,
      );
    }

    const emoji = emojiOf(entry.emoji, "guide", at);

    if (entry.roomId !== null && text !== "") {
      throw validation("guide", `#${at}: links a room or holds a note, not both`);
    }

    if (entry.roomId === null && text === "") {
      throw validation("guide", `#${at}: needs a room or a note`);
    }

    if (length(text) > BODY_MAX) {
      throw validation("guide", `#${at}: note must be ${BODY_MAX} characters or fewer`);
    }

    if (CONTROL_EXCEPT_BREAKS.test(text)) {
      throw validation("guide", `#${at}: note can't contain control characters`);
    }

    return { title, emoji, roomId: entry.roomId, body: text === "" ? null : text };
  });

  return { message, rooms, guide };
}

export function createWelcome(
  ctx: S2Context,
  workspace: { readonly branding: () => WorkspaceBranding; readonly description: () => string },
  catalog: CatalogRoom,
): WelcomeModule {
  // The configuration lives with its world: a reset (a new world) starts without one.
  const saved = new WeakMap<World, WelcomeSettings>();

  const current = (): WelcomeSettings => saved.get(ctx.world()) ?? EMPTY;

  // `keep` tokens: opaque, each standing for a room id, per world.
  const tokens = new WeakMap<World, Map<string, number>>();
  let issued = 0;

  const keepToken = (roomId: number): string => {
    const known = tokens.get(ctx.world()) ?? new Map<string, number>();

    tokens.set(ctx.world(), known);

    issued += 1;

    const token = `keep-${issued.toString(36)}`;

    known.set(token, roomId);

    return token;
  };

  const keptRoom = (token: string): number | null => tokens.get(ctx.world())?.get(token) ?? null;

  const isAdministrator = () => ctx.world().users.get(VIEWER_ID)?.role === "administrator";

  const requireAdministrator = () => {
    if (!isAdministrator()) throw forbidden("Only administrators can change the welcome");
  };

  /** A room the viewer can open (joined) or join (an open room), by id; anything else is absent. */
  const reach = (roomId: number): Reachable | null => {
    const record = ctx.world().rooms.get(roomId);

    if (record === undefined) {
      const open = catalog(roomId);

      return open === null ? null : { name: open.name, joined: false };
    }

    if (record.room.kind === "direct") return null;

    const name = record.room.name ?? "";

    if (record.memberIds.includes(VIEWER_ID)) return { name, joined: true };

    return record.room.kind === "open" ? { name, joined: false } : null;
  };

  const choices = (): WelcomeRoomChoice[] => {
    const ids = [...ctx.world().rooms.keys()];
    const fromCatalog = catalog(JOINABLE_OPEN_ROOM.id);

    if (fromCatalog !== null && !ids.includes(fromCatalog.id)) ids.push(fromCatalog.id);

    return ids
      .flatMap((id) => {
        const reached = reach(id);

        return reached === null ? [] : [{ id, name: reached.name }];
      })
      .sort((left, right) => left.name.localeCompare(right.name) || left.id - right.id);
  };

  /** The configuration as the viewer may see it: a room they can't reach goes as a token. */
  const adminView = (): WelcomeAdmin => {
    const settings = current();

    const mask = (roomId: number) =>
      reach(roomId) === null ? { roomId: null, keep: keepToken(roomId) } : { roomId, keep: null };

    return {
      message: settings.message,
      rooms: settings.rooms.map((room) => ({
        ...mask(room.roomId),
        emoji: room.emoji,
        description: room.description,
      })),
      guide: settings.guide.map((entry) => ({
        title: entry.title,
        emoji: entry.emoji,
        ...(entry.roomId === null ? { roomId: null, keep: null } : mask(entry.roomId)),
        body: entry.body,
      })),
      choices: choices(),
    };
  };

  const render = (source: string) =>
    `<div class="markdown-body">${renderMarkdown(source, ctx.mentionables())}</div>`;

  const viewerView = (): WorkspaceWelcome => {
    const settings = current();

    const rooms: WelcomeRoomCard[] = settings.rooms.flatMap((room) => {
      const reached = reach(room.roomId);

      return reached === null
        ? []
        : [
            {
              roomId: room.roomId,
              name: reached.name,
              emoji: room.emoji,
              description: room.description,
              joined: reached.joined,
            },
          ];
    });

    const guide = settings.guide.flatMap((entry): GuideEntry[] => {
      if (entry.roomId === null) {
        return [
          {
            title: entry.title,
            emoji: entry.emoji,
            bodyHtml: render(entry.body ?? ""),
            room: null,
          },
        ];
      }

      const reached = reach(entry.roomId);

      return reached === null
        ? []
        : [
            {
              title: entry.title,
              emoji: entry.emoji,
              bodyHtml: null,
              room: { id: entry.roomId, name: reached.name, joined: reached.joined },
            },
          ];
    });

    return {
      name: workspace.branding().name,
      description: workspace.description(),
      messageHtml: settings.message === "" ? null : render(settings.message),
      rooms,
      guide,
      canAdminister: isAdministrator(),
    };
  };

  const store = (settings: WelcomeSettings) => {
    saved.set(ctx.world(), settings);
    ctx.publish([{ topic: "user", type: "workspace.updated", data: workspace.branding() }]);
  };

  const save = (body: Json | undefined): MockResponse => {
    requireAdministrator();

    const update = updateOf(body);

    if (update === null) throw validation("message", "isn't a welcome update");

    if ((update.rooms?.length ?? 0) > ROOMS_MAX) {
      throw validation("rooms", `can suggest at most ${ROOMS_MAX} rooms`);
    }

    if ((update.guide?.length ?? 0) > GUIDE_MAX) {
      throw validation("guide", `can hold at most ${GUIDE_MAX} resources`);
    }

    const previous = current();

    const saved = new Set([
      ...previous.rooms.map((room) => room.roomId),
      ...previous.guide.flatMap((entry) => (entry.roomId === null ? [] : [entry.roomId])),
    ]);

    // A room the administrator can reach, or a token for one already saved; nothing else.
    const pick = (
      section: "rooms" | "guide",
      at: number,
      row: { readonly roomId: number | null; readonly keep: string | null },
      unreachable: string,
    ): number | null => {
      if (row.roomId !== null && row.keep !== null) {
        throw validation(section, `#${at}: names a room and keeps one; send one or the other`);
      }

      if (row.roomId !== null) {
        if (reach(row.roomId) === null) throw validation(section, `#${at}: ${unreachable}`);

        return row.roomId;
      }

      if (row.keep !== null) {
        const kept = keptRoom(row.keep);

        if (kept === null || !saved.has(kept)) {
          throw validation(section, `#${at}: that room is no longer saved; choose another`);
        }

        return kept;
      }

      return null;
    };

    const rooms: WelcomeRoom[] | undefined = update.rooms?.map((room, index) => {
      const roomId = pick("rooms", index + 1, room, "isn't a room you can suggest");

      if (roomId === null) throw validation("rooms", `#${index + 1}: choose a room`);

      return { roomId, emoji: room.emoji, description: room.description };
    });

    const guide: GuideResource[] | undefined = update.guide?.map((entry, index) => ({
      title: entry.title,
      emoji: entry.emoji,
      roomId: pick("guide", index + 1, entry, "links to a room you can't choose"),
      body: entry.body,
    }));

    store(
      normalize({
        message: update.message ?? previous.message,
        rooms: rooms ?? previous.rooms,
        guide: guide ?? previous.guide,
      }),
    );

    return ok(adminView());
  };

  return {
    routes: [
      route("GET", /^\/welcome$/, () => ok(viewerView())),
      route("GET", /^\/admin\/welcome$/, () => {
        requireAdministrator();

        return ok(adminView());
      }),
      route("PATCH", /^\/admin\/welcome$/, (request) => save(request.body)),
    ],
    control(body) {
      const settings = (isRecord(body) ? settingsOf(body) : null) ?? EMPTY;

      store(settings);

      return settings;
    },
  };
}
