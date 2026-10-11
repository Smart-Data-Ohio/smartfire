/**
 * The welcome card and server guide (`crates/api/src/welcome.rs`): administrators read and
 * replace the configuration at `/admin/welcome`; everyone reads `/welcome`, which keeps only the
 * rooms the viewer belongs to (Open) or may join (an alive open room: Join). The server's rules
 * apply on a write (lengths, counts, rooms that exist and aren't direct messages), and every save
 * publishes `workspace.updated`, as the server's `write_workspace` does. The configuration
 * belongs to the world, so a reset clears it. `/__mock/welcome` stands in for another
 * administrator's save.
 */
import type { GuideEntry } from "../../src/gen/GuideEntry.ts";
import type { GuideResource } from "../../src/gen/GuideResource.ts";
import type { WelcomeAdmin } from "../../src/gen/WelcomeAdmin.ts";
import type { WelcomeRoom } from "../../src/gen/WelcomeRoom.ts";
import type { WelcomeRoomCard } from "../../src/gen/WelcomeRoomCard.ts";
import type { WelcomeRoomChoice } from "../../src/gen/WelcomeRoomChoice.ts";
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

  const adminView = (): WelcomeAdmin => ({ settings: current(), choices: choices() });

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

    const given = settingsOf(body);

    if (given === null) throw validation("message", "isn't a welcome configuration");

    const settings = normalize(given);

    const previous = new Set([
      ...current().rooms.map((room) => room.roomId),
      ...current().guide.flatMap((entry) => (entry.roomId === null ? [] : [entry.roomId])),
    ]);

    // A room saved before stays saveable while it exists and isn't a direct message.
    const allowed = (roomId: number) => {
      if (reach(roomId) !== null) return true;

      const record = ctx.world().rooms.get(roomId);

      return previous.has(roomId) && record !== undefined && record.room.kind !== "direct";
    };

    settings.rooms.forEach((room, index) => {
      if (!allowed(room.roomId)) {
        throw validation("rooms", `#${index + 1}: isn't a room you can suggest`);
      }
    });

    settings.guide.forEach((entry, index) => {
      if (entry.roomId !== null && !allowed(entry.roomId)) {
        throw validation("guide", `#${index + 1}: links to a room you can't choose`);
      }
    });

    store(settings);

    return ok(adminView());
  };

  return {
    routes: [
      route("GET", /^\/welcome$/, () => ok(viewerView())),
      route("GET", /^\/admin\/welcome$/, () => {
        requireAdministrator();

        return ok(adminView());
      }),
      route("PUT", /^\/admin\/welcome$/, (request) => save(request.body)),
    ],
    control(body) {
      const settings = (isRecord(body) ? settingsOf(body) : null) ?? EMPTY;

      store(settings);

      return settings;
    },
  };
}
