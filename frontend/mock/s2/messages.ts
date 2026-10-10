/**
 * Message actions: edit, edit source, delete, reactions and boosts, pins, saved items and
 * forwards, each publishing what the contract says on the message's conversation topic or the
 * viewer's `user` topic.
 */

import type { ForwardDestinationList } from "../../src/gen/ForwardDestinationList.ts";
import type { ForwardResult } from "../../src/gen/ForwardResult.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { MessageReactions } from "../../src/gen/MessageReactions.ts";
import type { MessageRead } from "../../src/gen/MessageRead.ts";
import type { MessageSource } from "../../src/gen/MessageSource.ts";
import type { PinList } from "../../src/gen/PinList.ts";
import type { PinState } from "../../src/gen/PinState.ts";
import type { Reaction } from "../../src/gen/Reaction.ts";
import type { SavedItem } from "../../src/gen/SavedItem.ts";
import { forbidden, type MockResponse, noContent, notFound, ok, validation } from "../http.ts";
import { field, intField, isRecord, type Json, stringField } from "../json.ts";
import { renderMarkdown } from "../markdown.ts";
import { VIEWER_ID } from "../seed.ts";
import { mockSound } from "../sounds.ts";
import type { Outgoing } from "../sync.ts";
import { firstId, type Route, route, type S2Context } from "./context.ts";
import { reactionContent } from "./emoji.ts";
import {
  iso,
  locate,
  MAX_PINS_PER_ROOM,
  type MessageLocation,
  pinCount,
  SOURCE_LIMIT,
  type ThreadRecord,
  threadStatus,
  topicOf,
  touched,
} from "./model.ts";
import { checkMarkdown } from "./posting.ts";
import type { Threads } from "./threads.ts";

/** A boost is at most this many characters. */
export const BOOST_LIMIT = 16;

/** A message can go to at most this many places at once. */
export const MAX_FORWARD_DESTINATIONS = 5;

/** The messages module. */
export interface Messages {
  readonly routes: readonly Route[];
  /** `userId` reacts or boosts, as `POST /messages/:id/boosts` would for them. */
  react(messageId: number, userId: number, content: string): MessageReactions;
  /** `userId` pins or unpins a message. */
  pin(messageId: number, userId: number, pinned: boolean): PinState;
  /** Saves a message for the viewer (or updates its reminder). */
  save(messageId: number, remindAt: string | null): SavedItem;
}

/** Tells another module about a saved item that changed: saved, re-saved, unsaved or deleted. */
export type SavedHook = (
  messageId: number,
  before: SavedItem | null,
  after: SavedItem | null,
) => void;

/** Creates the messages module. `savedChanged` hears about every saved item change. */
export function createMessages(
  ctx: S2Context,
  threads: Threads,
  savedChanged: SavedHook = () => undefined,
): Messages {
  const messageOr404 = (messageId: number): MessageLocation => {
    const location = locate(ctx.world(), messageId);

    if (location === null) throw notFound("Message not found");

    try {
      ctx.roomOr404(location.room.room.id);
    } catch {
      throw notFound("Message not found");
    }

    return location;
  };

  const replace = (location: MessageLocation, next: MessageDTO) => {
    location.list[location.index] = next;
  };

  const isAdmin = () => ctx.world().users.get(VIEWER_ID)?.role === "administrator";

  const ensureCanEdit = (location: MessageLocation) => {
    if (location.message.creatorId !== VIEWER_ID || location.message.systemNote) {
      throw forbidden("You can only edit your own messages");
    }

    if (location.thread?.locked === true) throw forbidden("This thread is locked");
  };

  // --- edit and delete ---

  const edit = (messageId: number, body: Json | undefined): MessageDTO => {
    const location = messageOr404(messageId);

    ensureCanEdit(location);

    const current = location.message;

    const markdown = checkMarkdown(
      stringField(body, "markdownSource"),
      current.attachment !== null,
    );

    const updatedAt = touched(ctx.now(), current.updatedAt);
    const changed = markdown !== current.markdownSource;

    const next: MessageDTO = {
      ...current,
      bodyHtml: renderMarkdown(markdown, ctx.mentionables()),
      markdownSource: markdown,
      sound: current.attachment === null ? mockSound(markdown) : null,
      editedAt: changed ? updatedAt : current.editedAt,
      updatedAt,
    };

    replace(location, next);
    ctx.publish([{ topic: topicOf(next), type: "message.updated", data: next }]);

    return next;
  };

  const source = (messageId: number): MessageSource => {
    const location = messageOr404(messageId);

    ensureCanEdit(location);

    const markdown =
      location.message.markdownSource ??
      location.message.bodyHtml.replace(/<br\s*\/?>/g, "\n").replace(/<[^>]+>/g, "");

    return { messageId, markdownSource: markdown };
  };

  const remove = (messageId: number): MockResponse => {
    const world = ctx.world();
    const location = messageOr404(messageId);
    const message = location.message;

    if (message.systemNote) throw forbidden("System notes can't be deleted");

    if (message.creatorId !== VIEWER_ID && !isAdmin()) {
      throw forbidden("You can only delete your own messages");
    }

    location.list.splice(location.index, 1);

    const events: Outgoing[] = [
      {
        topic: topicOf(message),
        type: "message.removed",
        data: { id: message.id, roomId: message.roomId, threadId: message.threadId },
      },
    ];

    if (world.pins.delete(message.id)) {
      events.push({
        topic: `room:${message.roomId}`,
        type: "message.pinned",
        data: {
          messageId: message.id,
          roomId: message.roomId,
          pinned: false,
          pinCount: pinCount(world, message.roomId),
        },
      });
    }

    const savedBefore = world.saved.get(message.id) ?? null;

    if (world.saved.delete(message.id)) {
      savedChanged(message.id, savedBefore, null);
      events.push({
        topic: "user",
        type: "saved.changed",
        data: { messageId: message.id, item: null },
      });
    }

    // A hard delete: replies lose their target and forwards their source, quietly.
    for (const list of [location.list, ...[...world.rooms.values()].map((room) => room.messages)]) {
      list.forEach((candidate, index) => {
        if (candidate.replyToMessageId === message.id) {
          list[index] = { ...candidate, replyToMessageId: null };
        } else if (candidate.forwardedFromMessageId === message.id) {
          list[index] = { ...candidate, forwardedFromMessageId: null };
        }
      });
    }

    if (location.thread === null && location.room.memberIds.includes(VIEWER_ID)) {
      events.push({
        topic: "user",
        type: "sidebar.row.upserted",
        data: ctx.sidebarRow(location.room),
      });
    }

    ctx.publish(events);

    if (location.thread === null) {
      threads.parentRemoved(message.id);
    } else {
      threads.replyRemoved(location.thread);
    }

    return noContent();
  };

  // --- reactions and boosts ---

  const reactionsOf = (message: MessageDTO): MessageReactions => ({
    messageId: message.id,
    roomId: message.roomId,
    threadId: message.threadId,
    reactions: message.reactions,
    boosts: message.boosts,
    updatedAt: message.updatedAt,
  });

  const changeReactions = (
    location: MessageLocation,
    reactions: Reaction[],
    boosts: MessageDTO["boosts"],
  ): MessageReactions => {
    const next: MessageDTO = {
      ...location.message,
      reactions,
      boosts,
      updatedAt: touched(ctx.now(), location.message.updatedAt),
    };

    replace(location, next);

    const data = reactionsOf(next);

    ctx.publish([{ topic: topicOf(next), type: "message.reactions", data }]);

    return data;
  };

  const react = (messageId: number, userId: number, raw: string): MessageReactions => {
    const location = messageOr404(messageId);
    const content = raw.trim();

    if (content === "") throw validation("content", "Content can't be blank");

    if ([...content].length > BOOST_LIMIT) {
      throw validation("content", `Content is too long (maximum is ${BOOST_LIMIT} characters)`);
    }

    const reaction = reactionContent(content);
    const message = location.message;

    if (reaction === null) {
      const boost = {
        id: ctx.world().nextBoostId++,
        boosterId: userId,
        content,
        createdAt: iso(ctx.now()),
      };

      return changeReactions(location, message.reactions, [...message.boosts, boost]);
    }

    const existing = message.reactions.find((pill) => pill.content === reaction.content);
    let reactions: Reaction[];

    if (existing === undefined) {
      reactions = [
        ...message.reactions,
        {
          content: reaction.content,
          title: reaction.title,
          imageUrl: reaction.imageUrl,
          reactorIds: [userId],
        },
      ];
    } else if (existing.reactorIds.includes(userId)) {
      reactions = message.reactions.flatMap((pill) => {
        if (pill !== existing) return [pill];

        const reactorIds = pill.reactorIds.filter((id) => id !== userId);

        return reactorIds.length === 0 ? [] : [{ ...pill, reactorIds }];
      });
    } else {
      reactions = message.reactions.map((pill) =>
        pill === existing ? { ...pill, reactorIds: [...pill.reactorIds, userId] } : pill,
      );
    }

    return changeReactions(location, reactions, message.boosts);
  };

  const unboost = (messageId: number, boostId: number): MessageReactions => {
    const location = messageOr404(messageId);
    const boost = location.message.boosts.find((candidate) => candidate.id === boostId);

    if (boost === undefined) throw notFound("Boost not found");

    if (boost.boosterId !== VIEWER_ID) throw forbidden("You can only remove your own boosts");

    return changeReactions(
      location,
      location.message.reactions,
      location.message.boosts.filter((candidate) => candidate.id !== boostId),
    );
  };

  // --- pins ---

  const pin = (messageId: number, userId: number, pinned: boolean): PinState => {
    const world = ctx.world();
    const location = messageOr404(messageId);
    const roomId = location.room.room.id;
    const existing = world.pins.has(messageId);

    if (pinned && !existing) {
      if (pinCount(world, roomId) >= MAX_PINS_PER_ROOM) {
        throw validation("base", `A room can have at most ${MAX_PINS_PER_ROOM} pinned messages`);
      }

      world.pins.set(messageId, {
        roomId,
        pin: { messageId, pinnerId: userId, pinnedAt: iso(ctx.now()) },
      });
    } else if (!pinned && existing) {
      world.pins.delete(messageId);
    }

    const state: PinState = { messageId, roomId, pinned, pinCount: pinCount(world, roomId) };

    if (pinned !== existing) {
      replace(location, {
        ...location.message,
        pinned,
        updatedAt: touched(ctx.now(), location.message.updatedAt),
      });
      ctx.publish([{ topic: `room:${roomId}`, type: "message.pinned", data: state }]);
    }

    return state;
  };

  const pins = (roomId: number): PinList => {
    const world = ctx.world();

    ctx.roomOr404(roomId);

    const rows = [...world.pins.values()]
      .flatMap((record) => (record.roomId === roomId ? [record.pin] : []))
      .sort((a, b) => Date.parse(b.pinnedAt) - Date.parse(a.pinnedAt) || b.messageId - a.messageId)
      .slice(0, MAX_PINS_PER_ROOM);

    const messages: MessageDTO[] = [];

    for (const row of rows) {
      const location = locate(world, row.messageId);

      if (location !== null) messages.push(location.message);
    }

    return {
      pins: rows,
      messages,
      users: ctx.usersFor([
        ...rows.map((row) => row.pinnerId),
        ...messages.map((message) => message.creatorId),
      ]),
    };
  };

  // --- saved items ---

  const save = (messageId: number, remindAt: string | null): SavedItem => {
    const world = ctx.world();

    messageOr404(messageId);

    if (remindAt !== null) {
      const at = Date.parse(remindAt);

      if (Number.isNaN(at) || at <= ctx.now()) {
        throw validation("remindAt", "Remind at must be in the future");
      }
    }

    const at = remindAt === null ? null : iso(Date.parse(remindAt));
    const current = world.saved.get(messageId);

    const item: SavedItem =
      current === undefined
        ? {
            id: world.nextSavedId++,
            messageId,
            status: "in_progress",
            remindAt: at,
            remindedAt: null,
            createdAt: iso(ctx.now()),
          }
        : { ...current, remindAt: at, remindedAt: null };

    world.saved.set(messageId, item);
    ctx.publish([{ topic: "user", type: "saved.changed", data: { messageId, item } }]);
    savedChanged(messageId, current ?? null, item);

    return item;
  };

  const unsave = (savedItemId: number): MockResponse => {
    const world = ctx.world();

    for (const item of world.saved.values()) {
      if (item.id !== savedItemId) continue;

      world.saved.delete(item.messageId);
      savedChanged(item.messageId, item, null);
      ctx.publish([
        {
          topic: "user",
          type: "saved.changed",
          data: { messageId: item.messageId, item: null },
        },
      ]);

      return noContent();
    }

    throw notFound("Saved item not found");
  };

  // --- forwards ---

  const forwardableThreads = (roomId: number): ThreadRecord[] =>
    [...ctx.world().threads.values()]
      .filter((thread) => thread.roomId === roomId && !thread.locked)
      .sort((a, b) => Date.parse(b.lastActivityAt) - Date.parse(a.lastActivityAt) || b.id - a.id);

  const destinations = (): ForwardDestinationList => {
    const rooms = [...ctx.world().rooms.values()]
      .flatMap((record) =>
        record.membership.involvement !== "invisible" && record.room.kind !== "board"
          ? [{ record, name: ctx.displayName(record) }]
          : [],
      )
      .sort((a, b) => {
        const [left, right] = [a.name.toLowerCase(), b.name.toLowerCase()];

        return left < right ? -1 : left > right ? 1 : a.record.room.id - b.record.room.id;
      });

    return {
      destinations: rooms.map(({ record, name }) => {
        const direct = record.room.kind === "direct";

        return {
          roomId: record.room.id,
          name,
          direct,
          threads: direct
            ? []
            : forwardableThreads(record.room.id).map((thread) => ({
                id: thread.id,
                name: thread.name,
                status: threadStatus(thread, ctx.now()),
              })),
        };
      }),
    };
  };

  const forward = (messageId: number, body: Json | undefined): MockResponse => {
    const world = ctx.world();
    const original = messageOr404(messageId).message;
    const rawNote = stringField(body, "note");
    const note = rawNote === null || rawNote.trim() === "" ? null : rawNote;

    if (note !== null && note.length > SOURCE_LIMIT) {
      throw validation("note", `Note is too long (maximum is ${SOURCE_LIMIT} characters)`);
    }

    const raw = field(body, "destinations");
    const targets = Array.isArray(raw) ? raw : [];

    if (targets.length < 1 || targets.length > MAX_FORWARD_DESTINATIONS) {
      throw validation("destinations", `Choose 1 to ${MAX_FORWARD_DESTINATIONS} destinations`);
    }

    const seen = new Set<string>();

    const resolved = targets.map((target: Json) => {
      const roomId = isRecord(target) ? intField(target, "roomId") : null;
      const threadId = isRecord(target) ? intField(target, "threadId") : null;
      const record = roomId === null ? undefined : world.rooms.get(roomId);

      if (
        record === undefined ||
        record.membership.involvement === "invisible" ||
        record.room.kind === "board"
      ) {
        throw validation("destinations", "You can't forward to that room");
      }

      const key = `${roomId}:${threadId ?? "root"}`;

      if (seen.has(key)) throw validation("destinations", "Each destination can be chosen once");

      seen.add(key);

      if (threadId === null) return { record, thread: null };

      const thread = world.threads.get(threadId);

      if (thread === undefined || thread.roomId !== roomId || record.room.kind === "direct") {
        throw validation("destinations", "That thread isn't in that room");
      }

      if (thread.locked) throw validation("destinations", "That thread is locked");

      return { record, thread };
    });

    const forwardedAt = iso(ctx.now());

    const forwards = resolved.map(({ record, thread }) => {
      const draft = {
        creatorId: VIEWER_ID,
        markdown: original.markdownSource ?? "",
        clientMessageId: ctx.uuid(),
        replyToMessageId: null,
        streaming: false,
        attachment: original.attachment,
        action: false,
        systemNote: false,
        forward: { messageId: original.id, at: forwardedAt, note, bodyHtml: original.bodyHtml },
      };

      return thread === null ? ctx.postToRoom(record, draft) : threads.postReply(thread, draft);
    });

    const result: ForwardResult = { forwards };

    return ok(result, 201);
  };

  // --- one message ---

  /** `GET /messages/:id`: the message with its conversation; a 404 when it isn't reachable. */
  const read = (messageId: number): MessageRead => {
    const { room, thread, message } = messageOr404(messageId);
    const item = ctx.world().saved.get(message.id);

    return {
      message,
      users: ctx.usersFor([message.creatorId, ...(message.thread?.replierIds ?? [])]),
      conversation: {
        roomId: room.room.id,
        threadId: thread?.id ?? null,
        roomKind: room.room.kind,
        roomName: ctx.displayName(room),
        roomIconName: room.room.iconName,
        threadName: thread?.name ?? null,
      },
      saved: item === undefined ? null : { messageId: message.id, savedItemId: item.id },
    };
  };

  return {
    routes: [
      route("GET", /^\/messages\/(\d+)$/, (request) => ok(read(firstId(request)))),
      route("PATCH", /^\/messages\/(\d+)$/, (request) => ok(edit(firstId(request), request.body))),
      route("GET", /^\/messages\/(\d+)\/source$/, (request) => ok(source(firstId(request)))),
      route("DELETE", /^\/messages\/(\d+)$/, (request) => remove(firstId(request))),
      route("POST", /^\/messages\/(\d+)\/boosts$/, (request) =>
        ok(react(firstId(request), VIEWER_ID, stringField(request.body, "content") ?? "")),
      ),
      route("DELETE", /^\/messages\/(\d+)\/boosts\/(\d+)$/, (request) =>
        ok(unboost(firstId(request), request.ids[1] ?? 0)),
      ),
      route("POST", /^\/messages\/(\d+)\/pin$/, (request) =>
        ok(pin(firstId(request), VIEWER_ID, true), 201),
      ),
      route("DELETE", /^\/messages\/(\d+)\/pin$/, (request) =>
        ok(pin(firstId(request), VIEWER_ID, false)),
      ),
      route("GET", /^\/rooms\/(\d+)\/pins$/, (request) => ok(pins(firstId(request)))),
      route("POST", /^\/saved$/, (request) => {
        const messageId = intField(request.body, "messageId");

        if (messageId === null) throw notFound("Message not found");

        return ok(save(messageId, stringField(request.body, "remindAt")), 201);
      }),
      route("DELETE", /^\/saved\/(\d+)$/, (request) => unsave(firstId(request))),
      route("GET", /^\/forward_destinations$/, () => ok(destinations())),
      route("POST", /^\/messages\/(\d+)\/forwards$/, (request) =>
        forward(firstId(request), request.body),
      ),
    ],
    react,
    pin,
    save,
  };
}
