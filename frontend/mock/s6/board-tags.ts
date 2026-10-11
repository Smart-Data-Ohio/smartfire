import type { BoardTag } from "../../src/gen/BoardTag.ts";
import type { BoardTagCatalog } from "../../src/gen/BoardTagCatalog.ts";
import { caselessEqual } from "../../src/lib/case-fold.ts";
import { forbidden, HttpError, notFound, ok, validation } from "../http.ts";
import {
  booleanField,
  field,
  intField,
  isNumber,
  isRecord,
  type Json,
  stringField,
} from "../json.ts";
import { firstId, route, type S2Context } from "../s2/context.ts";
import { VIEWER_ID, type World } from "../seed.ts";

/** A board's curated tags and policy as the mock keeps them (`board_tags`, rooms columns). */
export interface BoardTagState {
  tags: BoardTag[];
  tagsRequired: boolean;
  defaultBoardTagId: number | null;
  nextId: number;
}

const VALIDATION = "Validation";

/** The server's name comparison: trimmed, under full Unicode case folding. */
const same = (left: string, right: string) => caselessEqual(left.trim(), right.trim());

function tagState(world: World, roomId: number): BoardTagState | null {
  const room = world.rooms.get(roomId);

  if (room?.room.kind !== "board") return null;

  room.boardTags ??= { tags: [], tagsRequired: false, defaultBoardTagId: null, nextId: 9501 };

  return room.boardTags;
}

/** The board's catalog in display order (`BoardTag::for_room`: position, then id). */
export function boardTagCatalog(world: World, roomId: number): BoardTagCatalog {
  const state = tagState(world, roomId);

  return {
    roomId,
    tags: [...(state?.tags ?? [])]
      .sort((a, b) => a.position - b.position || a.id - b.id)
      .map((tag) => ({ ...tag })),
    tagsRequired: state?.tagsRequired ?? false,
    defaultBoardTagId: state?.defaultBoardTagId ?? null,
  };
}

/** A filter's tag as the listing answers it: a catalog name's stored label, else as given. */
export function canonicalBoardTag(world: World, roomId: number, tag: string): string {
  return (
    boardTagCatalog(world, roomId).tags.find((curated) => same(curated.name, tag))?.name ?? tag
  );
}

/**
 * `BoardTagPolicy::post_tags`: catalog names resolve to their label, the rest are lower-cased
 * free text under the classic grammar; a board that requires a tag gets its default, or refuses.
 */
export function boardPostTags(world: World, roomId: number, input: readonly string[]): string[] {
  const catalog = boardTagCatalog(world, roomId);
  const tags: string[] = [];

  for (const raw of input) {
    const trimmed = raw.trim();

    if (trimmed === "") continue;

    const label =
      catalog.tags.find((tag) => same(tag.name, trimmed))?.name ?? trimmed.toLowerCase();

    if (!tags.includes(label)) tags.push(label);
  }

  const free = tags.filter((name) => !catalog.tags.some((tag) => tag.name === name));

  if (free.some((tag) => [...tag].length > 30))
    throw validation("tags", "Tags are too long (maximum is 30 characters)");

  if (free.some((tag) => !/^[a-z0-9][a-z0-9-]*$/.test(tag)))
    throw validation("tags", "Tags must contain only letters, numbers and hyphens");

  if (catalog.tagsRequired && !tags.some((name) => catalog.tags.some((tag) => tag.name === name))) {
    const fallback = catalog.tags.find((tag) => tag.id === catalog.defaultBoardTagId);

    if (fallback === undefined) throw validation("tags", "must include a catalog tag");

    tags.push(fallback.name);
  }

  if (tags.length > 5) throw validation("tags", "Tags must have at most 5 entries");

  return tags;
}

type TagContext = Pick<S2Context, "world" | "roomOr404" | "publish">;

export function createBoardTags(ctx: TagContext) {
  const readable = (roomId: number) => {
    const room = ctx.roomOr404(roomId);
    const viewer = ctx.world().users.get(VIEWER_ID);

    if (
      room.room.kind !== "board" ||
      !room.memberIds.includes(VIEWER_ID) ||
      viewer?.status !== "active" ||
      viewer.role === "bot"
    )
      throw notFound("Board not found");

    return room;
  };

  const writable = (roomId: number): BoardTagState => {
    const room = readable(roomId);

    if (
      room.room.creatorId !== VIEWER_ID &&
      ctx.world().users.get(VIEWER_ID)?.role !== "administrator"
    )
      throw forbidden("Forbidden");

    const state = tagState(ctx.world(), roomId);

    if (state === null) throw notFound("Board not found");

    return state;
  };

  /** `board.automations.changed`: catalog readers refetch. */
  const changed = (roomId: number) =>
    ctx.publish([{ topic: `room:${roomId}`, type: "board.automations.changed", data: { roomId } }]);

  const answer = (roomId: number, status = 200) => {
    changed(roomId);

    return ok(boardTagCatalog(ctx.world(), roomId), status);
  };

  const checkName = (state: BoardTagState, name: string, id: number | null) => {
    const fields: Record<string, string[]> = {};

    const add = (key: string, message: string) => {
      fields[key] = [...(fields[key] ?? []), message];
    };

    if (name === "") add("name", "can't be blank");

    if ([...name].length > 20) add("name", "is too long (maximum is 20 characters)");

    if (id === null && state.tags.length >= 20) add("tags", "are limited to 20 per board");

    if (state.tags.some((tag) => tag.id !== id && same(tag.name, name)))
      add("name", "has already been taken");

    if (Object.keys(fields).length > 0) {
      const messages = Object.entries(fields).flatMap(([key, list]) =>
        list.map((message) => `${key === "name" ? "Name" : "Tags"} ${message}`),
      );

      throw new HttpError(422, {
        _tag: VALIDATION,
        message: `Validation failed: ${messages.join(", ")}`,
        fields,
      });
    }
  };

  const emojiOf = (body: Json | undefined): string | null => {
    const emoji = stringField(body, "emoji");

    return emoji === null || emoji.trim() === "" ? null : emoji;
  };

  /** Posts on the board, for a rename or delete to follow through. */
  const posts = (roomId: number) =>
    [...ctx.world().threads.values()].filter((thread) => thread.roomId === roomId && thread.work);

  const create = (roomId: number, body: Json | undefined) => {
    const state = writable(roomId);
    const name = (stringField(body, "name") ?? "").trim();

    checkName(state, name, null);

    const position = Math.max(-1, ...state.tags.map((tag) => tag.position)) + 1;

    state.tags.push({ id: state.nextId++, name, emoji: emojiOf(body), position });

    return answer(roomId, 201);
  };

  const update = (roomId: number, tagId: number, body: Json | undefined) => {
    const state = writable(roomId);
    const tag = state.tags.find((candidate) => candidate.id === tagId);

    if (tag === undefined) throw notFound("Tag not found");

    const name = (stringField(body, "name") ?? "").trim();

    checkName(state, name, tagId);

    const prior = tag.name;

    tag.name = name;
    tag.emoji = emojiOf(body);

    if (prior !== name) {
      for (const post of posts(roomId)) {
        const work = post.work;

        if (work == null || !work.tags.some((held) => same(held, prior))) continue;

        const kept = work.tags.filter((held) => !same(held, prior) && !same(held, name));

        work.tags = [...kept, name];
      }
    }

    return answer(roomId);
  };

  const destroy = (roomId: number, tagId: number) => {
    const state = writable(roomId);
    const tag = state.tags.find((candidate) => candidate.id === tagId);

    if (tag === undefined) throw notFound("Tag not found");

    state.tags = state.tags.filter((candidate) => candidate.id !== tagId);

    if (state.defaultBoardTagId === tagId) state.defaultBoardTagId = null;

    // `BoardTag::destroy`: the last tag takes a requirement with it, so the board stays postable.
    if (state.tags.length === 0) state.tagsRequired = false;

    for (const post of posts(roomId)) {
      if (post.work != null)
        post.work.tags = post.work.tags.filter((held) => !same(held, tag.name));
    }

    return answer(roomId);
  };

  const reorder = (roomId: number, body: Json | undefined) => {
    const state = writable(roomId);
    const raw = field(body, "tagIds");
    const ids = Array.isArray(raw) ? raw.filter(isNumber) : [];
    const unique = new Set(ids);

    if (
      ids.length !== state.tags.length ||
      unique.size !== state.tags.length ||
      state.tags.some((tag) => !unique.has(tag.id))
    ) {
      throw validation("tag_ids", "must contain every board tag exactly once");
    }

    for (const tag of state.tags) tag.position = ids.indexOf(tag.id);

    return answer(roomId);
  };

  const policy = (roomId: number, body: Json | undefined) => {
    const state = writable(roomId);
    const required = booleanField(body, "tagsRequired") ?? false;
    const defaultId = intField(body, "defaultBoardTagId");

    if (defaultId !== null && !state.tags.some((tag) => tag.id === defaultId))
      throw validation("default_board_tag_id", "must belong to this board");

    if (required && state.tags.length === 0)
      throw validation("tags_required", "needs at least one board tag");

    state.tagsRequired = required;
    state.defaultBoardTagId = defaultId;

    return answer(roomId);
  };

  return {
    routes: [
      route("GET", /^\/rooms\/(\d+)\/board\/tags$/, (request) => {
        readable(firstId(request));

        return ok(boardTagCatalog(ctx.world(), firstId(request)));
      }),
      route("POST", /^\/rooms\/(\d+)\/board\/tags$/, (request) =>
        create(firstId(request), request.body),
      ),
      route("PUT", /^\/rooms\/(\d+)\/board\/tags\/order$/, (request) =>
        reorder(firstId(request), request.body),
      ),
      route("PATCH", /^\/rooms\/(\d+)\/board\/tags\/(\d+)$/, (request) =>
        update(firstId(request), request.ids[1] ?? 0, request.body),
      ),
      route("DELETE", /^\/rooms\/(\d+)\/board\/tags\/(\d+)$/, (request) =>
        destroy(firstId(request), request.ids[1] ?? 0),
      ),
      route("PATCH", /^\/rooms\/(\d+)\/board$/, (request) =>
        policy(firstId(request), request.body),
      ),
    ],
    /**
     * `/__mock/board-tags`: someone else replaces the board's catalog, `{ roomId?, tags: [{ name,
     * emoji? }], tagsRequired?, defaultTag? }` (the default by name), and the room hears about it.
     */
    control(roomId: number, body: Json | undefined) {
      const state = tagState(ctx.world(), roomId);

      if (state === null) throw notFound("Board not found");

      const raw = field(body, "tags");
      const entries = Array.isArray(raw) ? raw.filter(isRecord) : [];

      state.tags = entries.map((entry, position) => ({
        id: state.nextId++,
        name: stringField(entry, "name") ?? `Tag ${position + 1}`,
        emoji: stringField(entry, "emoji"),
        position,
      }));

      const named = stringField(body, "defaultTag");

      state.tagsRequired = booleanField(body, "tagsRequired") ?? false;
      state.defaultBoardTagId =
        named === null ? null : (state.tags.find((tag) => same(tag.name, named))?.id ?? null);

      return answer(roomId);
    },
  };
}
