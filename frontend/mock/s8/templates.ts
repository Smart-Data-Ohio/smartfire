/** File templates mirror the admin API, using the mock's normal room creator. */
import type { TemplateBoard } from "../../src/gen/TemplateBoard.ts";
import type { TemplatePreview } from "../../src/gen/TemplatePreview.ts";
import type { TemplateRoom } from "../../src/gen/TemplateRoom.ts";
import type { TemplateRoomKind } from "../../src/gen/TemplateRoomKind.ts";
import type { WorkspaceTemplate } from "../../src/gen/WorkspaceTemplate.ts";
import { caselessEqual } from "../../src/lib/case-fold.ts";
import { conflict, forbidden, ok, validation } from "../http.ts";
import { field, isBoolean, isNumber, isRecord, isString, type Json, parseJson } from "../json.ts";
import type { AdminModule } from "../s2/admin.ts";
import { route, type S2Context } from "../s2/context.ts";
import { lookupIcon } from "../s2/emoji.ts";
import { workspaceLayoutOf } from "../s3/workspace-layout.ts";
import { boardTagCatalog } from "../s6/board-tags.ts";
import { VIEWER_ID } from "../seed.ts";
import type { createRoomManagement } from "./rooms.ts";

const FILE_LIMIT = 1024 * 1024;

const KINDS: readonly TemplateRoomKind[] = ["open", "closed", "voice", "stage", "board"];

const BITRATES = [48000, 64000, 96000, 128000, 256000, 384000];

function fileError(message: string) {
  return validation("file", message, message);
}

function object(value: Json | undefined, keys: readonly string[]) {
  if (!isRecord(value) || Object.keys(value).some((key) => !keys.includes(key)))
    throw fileError("Invalid template object or unknown field.");

  return value;
}

function list(value: Json | undefined): readonly Json[] {
  if (!Array.isArray(value)) throw fileError("Expected a template array.");

  return value;
}

function text(value: Json | undefined): string {
  if (!isString(value)) throw fileError("Expected a template string.");

  return value;
}

function nullableText(value: Json | undefined): string | null {
  return value === undefined || value === null ? null : text(value);
}

function nullableInteger(value: Json | undefined): number | null {
  if (value === undefined || value === null) return null;

  if (!isNumber(value) || !Number.isSafeInteger(value))
    throw fileError("Expected an integer or null.");

  return value;
}

function boolean(value: Json | undefined): boolean {
  if (!isBoolean(value)) throw fileError("Expected a template boolean.");

  return value;
}

function boardOf(value: Json | undefined): TemplateBoard | null {
  if (value === undefined || value === null) return null;
  const board = object(value, ["tags", "tagsRequired", "defaultTag"]);

  return {
    tags: list(board.tags).map((value) => {
      const tag = object(value, ["name", "emoji"]);

      return { name: text(tag.name).trim(), emoji: nullableText(tag.emoji) };
    }),
    tagsRequired: boolean(board.tagsRequired),
    defaultTag: nullableInteger(board.defaultTag),
  };
}

function roomOf(value: Json): TemplateRoom {
  const room = object(value, [
    "kind",
    "name",
    "iconName",
    "topic",
    "announcement",
    "voiceBitrate",
    "voiceUserLimit",
    "board",
  ]);

  const kind = KINDS.find((kind) => kind === room.kind);

  if (kind === undefined)
    throw fileError("Unknown room kind. Expected open, closed, voice, stage or board.");

  const iconName =
    nullableText(room.iconName)
      ?.replace(/^[\0\t\n\v\f\r :]+|[\0\t\n\v\f\r :]+$/g, "")
      .toLowerCase() || null;

  return {
    kind,
    name: nullableText(room.name),
    iconName: iconName,
    topic: nullableText(room.topic)?.trim() || null,
    announcement: boolean(room.announcement),
    voiceBitrate: nullableInteger(room.voiceBitrate),
    voiceUserLimit: nullableInteger(room.voiceUserLimit),
    board: boardOf(room.board),
  };
}

function documentOf(value: Json | undefined): WorkspaceTemplate {
  const doc = object(value, ["format", "version", "categories", "rooms"]);

  if (doc.format !== "smartfire-template") throw fileError("Expected smartfire-template format.");

  if (doc.version !== 1)
    throw fileError(`Unsupported template version ${doc.version}. Expected version 1.`);

  return {
    format: "smartfire-template",
    version: 1,
    rooms: list(doc.rooms).map(roomOf),
    categories: list(doc.categories).map((value) => {
      const category = object(value, ["name", "rooms"]);

      return { name: text(category.name), rooms: list(category.rooms).map(roomOf) };
    }),
  };
}

function uniqueName(name: string | null, used: Set<string | null>, limit?: number): string | null {
  if (!used.has(name)) {
    used.add(name);

    return name;
  }

  for (let n = 2; ; n++) {
    const suffix = ` (${n})`;
    const base = name ?? "Unnamed room";

    const candidate = `${
      limit === undefined
        ? base
        : Array.from(base)
            .slice(0, limit - suffix.length)
            .join("")
    }${suffix}`;

    if (!used.has(candidate)) {
      used.add(candidate);

      return candidate;
    }
  }
}

export function createTemplates(
  ctx: S2Context,
  admin: Pick<AdminModule, "hasIcon">,
  management: ReturnType<typeof createRoomManagement>,
) {
  const authorize = () => {
    if (ctx.world().users.get(VIEWER_ID)?.role !== "administrator") throw forbidden();
  };

  const exportRoom = (id: number): TemplateRoom => {
    const record = ctx.world().rooms.get(id);

    if (record === undefined || record.room.kind === "direct")
      throw new Error("Missing template room");
    const room = record.room;
    const catalog = room.kind === "board" ? boardTagCatalog(ctx.world(), id) : null;
    const index = catalog?.tags.findIndex((tag) => tag.id === catalog.defaultBoardTagId) ?? -1;

    return {
      kind: record.room.kind,
      name: room.name,
      iconName: room.iconName,
      topic: room.topic ?? null,
      announcement: room.announcement ?? false,
      voiceBitrate: room.voiceBitrate ?? null,
      voiceUserLimit: room.voiceUserLimit ?? null,
      board:
        catalog === null
          ? null
          : {
              tags: catalog.tags.map((tag) => ({ name: tag.name, emoji: tag.emoji })),
              tagsRequired: catalog.tagsRequired,
              defaultTag: index < 0 ? null : index,
            },
    };
  };

  const group = (categoryId: number | null) =>
    [...ctx.world().rooms.values()]
      .filter(
        (r) =>
          r.room.kind !== "direct" &&
          (ctx.world().workspacePlacements.get(r.room.id)?.categoryId ?? null) === categoryId,
      )
      .sort((a, b) => {
        const left = ctx.world().workspacePlacements.get(a.room.id)?.position ?? null;
        const right = ctx.world().workspacePlacements.get(b.room.id)?.position ?? null;

        return (
          Number(left === null) - Number(right === null) ||
          (left ?? 0) - (right ?? 0) ||
          (a.room.name ?? "").toLowerCase().localeCompare((b.room.name ?? "").toLowerCase()) ||
          a.room.id - b.room.id
        );
      })
      .map((r) => exportRoom(r.room.id));

  const exported = (): WorkspaceTemplate => ({
    format: "smartfire-template",
    version: 1,
    categories: [...ctx.world().workspaceCategories]
      .sort((a, b) => a.position - b.position || a.id - b.id)
      .map((c) => ({ name: c.name, rooms: group(c.id) })),
    rooms: group(null),
  });

  const plan = (contents: string): TemplatePreview => {
    const result: TemplatePreview = {
      template: null,
      changes: [],
      errors: [],
      categoryCount: 0,
      roomCount: 0,
    };

    try {
      if (new TextEncoder().encode(contents).length > FILE_LIMIT)
        throw new Error("Template files must be at most 1 MiB.");
      const doc = documentOf(parseJson(contents));
      result.categoryCount = doc.categories.length;
      result.roomCount = doc.rooms.length + doc.categories.reduce((n, c) => n + c.rooms.length, 0);

      if (result.categoryCount > 100 || result.roomCount > 1000)
        throw new Error("Templates allow at most 100 categories and 1000 rooms.");
      const categoryNames = new Set(ctx.world().workspaceCategories.map((c) => c.name));

      const roomNames = new Set(
        [...ctx.world().rooms.values()].flatMap((r) =>
          r.room.kind === "direct" ? [] : [r.room.name],
        ),
      );

      const planRoom = (room: TemplateRoom) => {
        if (room.topic !== null && Array.from(room.topic).length > 1024)
          result.errors.push(`Room ${room.name}: topic is too long (maximum is 1024 characters).`);

        if (room.voiceBitrate !== null && !BITRATES.includes(room.voiceBitrate))
          result.errors.push(`Room ${room.name}: unsupported voice bitrate.`);

        if (room.voiceUserLimit !== null && (room.voiceUserLimit < 1 || room.voiceUserLimit > 99))
          result.errors.push(`Room ${room.name}: voice user limit must be between 1 and 99.`);

        if (
          room.iconName !== null &&
          lookupIcon(room.iconName) === null &&
          !admin.hasIcon(room.iconName)
        )
          result.errors.push(
            `Room ${room.name}: icon ${room.iconName} is not available in this workspace.`,
          );

        if ((room.kind === "board") !== (room.board !== null))
          result.errors.push(
            `Room ${room.name}: board settings must be present only for board rooms.`,
          );
        const board = room.board;

        if (board !== null) {
          if (
            board.tags.length > 20 ||
            (board.tagsRequired && board.tags.length === 0) ||
            (board.defaultTag !== null &&
              (board.defaultTag < 0 || board.defaultTag >= board.tags.length))
          )
            result.errors.push(
              `Room ${room.name}: invalid board tag count, required tags or default tag index.`,
            );

          if (
            board.tags.some(
              (tag, i) =>
                tag.name === "" ||
                Array.from(tag.name).length > 20 ||
                board.tags.slice(0, i).some((prior) => caselessEqual(prior.name, tag.name)),
            )
          )
            result.errors.push(
              `Room ${room.name}: board tag names must be unique and contain 1 to 20 characters.`,
            );
        }

        const createdName = uniqueName(room.name, roomNames);
        result.changes.push({ entity: "room", name: room.name, createdName });
        room.name = createdName;
      };

      for (const category of doc.categories) {
        if (category.name.trim() === "" || Array.from(category.name).length > 50)
          result.errors.push(`Category ${category.name}: name must contain 1 to 50 characters.`);
        const createdName = uniqueName(category.name, categoryNames, 50);
        result.changes.push({ entity: "category", name: category.name, createdName });
        category.name = createdName ?? "";

        for (const room of category.rooms) planRoom(room);
      }

      for (const room of doc.rooms) planRoom(room);

      result.template = doc;
    } catch (error) {
      result.errors.push(error instanceof Error ? error.message : "Invalid template JSON.");
    }

    return result;
  };

  const contentsOf = (body: Json | undefined): string => {
    const contents = field(body, "contents");

    if (!isString(contents))
      throw fileError("Malformed template upload: expected contents string.");

    return contents;
  };

  return {
    routes: [
      route("GET", /^\/admin\/templates\/export$/, () => {
        authorize();
        const doc = exported();

        const roomCount = doc.rooms.length + doc.categories.reduce((n, c) => n + c.rooms.length, 0);

        if (
          doc.categories.length > 100 ||
          roomCount > 1000 ||
          new TextEncoder().encode(JSON.stringify(doc, null, 2)).length > FILE_LIMIT
        )
          throw validation(
            "file",
            "Workspace structure exceeds the template limits (1 MiB, 100 categories, 1000 rooms)",
          );

        return {
          ...ok(doc),
          headers: { "content-disposition": 'attachment; filename="smartfire-template-v1.json"' },
        };
      }),
      route("POST", /^\/admin\/templates\/preview$/, ({ body }) => {
        authorize();
        object(body, ["contents"]);

        return ok(plan(contentsOf(body)));
      }),
      route("POST", /^\/admin\/templates\/import$/, ({ body }) => {
        authorize();
        object(body, ["contents", "expected"]);
        const preview = plan(contentsOf(body));

        if (preview.errors.length > 0 || preview.template === null)
          throw fileError(preview.errors.join("; "));
        const expected = documentOf(field(body, "expected"));

        if (JSON.stringify(preview.template) !== JSON.stringify(expected))
          throw conflict("Workspace structure changed. Preview this file again before importing.");
        const world = ctx.world();

        const create = (room: TemplateRoom, categoryId: number | null, position: number) => {
          const body =
            room.kind === "open"
              ? {
                  type: room.kind,
                  clientRoomId: ctx.uuid(),
                  name: room.name,
                  iconName: room.iconName,
                }
              : {
                  type: room.kind,
                  clientRoomId: ctx.uuid(),
                  name: room.name,
                  iconName: room.iconName,
                  userIds: [VIEWER_ID],
                };

          const reply = management.create(body);
          const id = field(field(reply.json, "room"), "id");
          const record = isNumber(id) ? world.rooms.get(id) : undefined;

          if (record === undefined) throw new Error("Expected a created room");
          record.room = {
            ...record.room,
            topic: room.topic,
            announcement: room.announcement,
            voiceBitrate: room.voiceBitrate,
            voiceUserLimit: room.voiceUserLimit,
          };

          if (room.board !== null)
            record.boardTags = {
              tags: room.board.tags.map((tag, position) => ({
                ...tag,
                id: position + 1,
                position,
              })),
              tagsRequired: room.board.tagsRequired,
              defaultBoardTagId: room.board.defaultTag === null ? null : room.board.defaultTag + 1,
              nextId: room.board.tags.length + 1,
            };
          world.workspacePlacements.set(record.room.id, { categoryId, position });
          ctx.publish([
            { topic: "user", type: "sidebar.row.upserted", data: ctx.sidebarRow(record) },
          ]);
        };

        for (const category of preview.template.categories) {
          const id = Math.max(0, ...world.workspaceCategories.map((c) => c.id)) + 1;
          world.workspaceCategories.push({
            id,
            name: category.name,
            position: world.workspaceCategories.length,
          });
          category.rooms.forEach((room, position) => {
            create(room, id, position);
          });
        }

        const offset = group(null).length;
        preview.template.rooms.forEach((room, position) => {
          create(room, null, offset + position);
        });
        ctx.publish([
          { topic: "user", type: "workspace.layout.updated", data: workspaceLayoutOf(world) },
        ]);

        return { ...ok(preview), status: 201 };
      }),
    ],
  };
}
