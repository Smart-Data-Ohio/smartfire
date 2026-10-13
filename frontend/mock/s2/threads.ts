/**
 * Channel threads: listing, opening, starting, replying, renaming, closing, locking, joining,
 * reading and deleting, with the sync events each one publishes on `room:<id>`,
 * `thread:<id>` and the viewer's `user` topic.
 */

import { ThreadAccess } from "../../src/api/errors.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { MessageSticker } from "../../src/gen/MessageSticker.ts";
import type { ThreadAccessMembers } from "../../src/gen/ThreadAccessMembers.ts";
import type { ThreadAccessRefusal } from "../../src/gen/ThreadAccessRefusal.ts";
import type { ThreadCreated } from "../../src/gen/ThreadCreated.ts";
import type { ThreadDetail } from "../../src/gen/ThreadDetail.ts";
import type { ThreadInvolvement } from "../../src/gen/ThreadInvolvement.ts";
import type { ThreadList } from "../../src/gen/ThreadList.ts";
import type { ThreadMembershipState } from "../../src/gen/ThreadMembershipState.ts";
import type { ThreadPermissions } from "../../src/gen/ThreadPermissions.ts";
import type { ThreadStatus } from "../../src/gen/ThreadStatus.ts";
import {
  conflict,
  forbidden,
  HttpError,
  type MockResponse,
  noContent,
  notFound,
  ok,
  validation,
} from "../http.ts";
import { booleanField, field, intField, isRecord, type Json, stringField } from "../json.ts";
import { workDetail, workUserIds } from "../s4/work-model.ts";
import { threadPermissions } from "../s6/work.ts";
import { VIEWER_ID } from "../seed.ts";
import type { Outgoing } from "../sync.ts";
import { firstId, type Route, route, type S2Context } from "./context.ts";
import {
  buildMessage,
  canAccessThread,
  DEFAULT_AUTO_ARCHIVE_MINUTES,
  defaultThreadName,
  indicatorOf,
  inviteThreadMentionees,
  iso,
  type MessageDraft,
  pageOf,
  plainDraft,
  THREAD_NAME_LIMIT,
  type ThreadRecord,
  threadDto,
  threadStatus,
  touched,
} from "./model.ts";
import {
  clientMessageIdOf,
  draftFiles,
  driveCards,
  nestedMessage,
  parseMessage,
} from "./posting.ts";
import type { Uploads } from "./uploads.ts";

const INVOLVEMENTS: readonly ThreadInvolvement[] = ["nothing", "mentions", "everything"];

const STATUSES: readonly ThreadStatus[] = ["active", "closed", "locked"];

/** The threads module. */
export interface Threads {
  readonly routes: readonly Route[];
  /** The thread, if the viewer belongs to its room; 404 otherwise. */
  threadOr404(threadId: number): ThreadRecord;
  /** Posts a reply with everything that follows: indicator, thread events, unread. */
  postReply(thread: ThreadRecord, draft: MessageDraft, fresh?: boolean): MessageDTO;
  finalizeReply(thread: ThreadRecord, message: MessageDTO): void;
  /** After a reply was deleted: the recount and the refresh events. */
  replyRemoved(thread: ThreadRecord): void;
  /** After a root message was deleted: a thread it started stays, with no parent. */
  parentRemoved(messageId: number): void;
  /** Starts or stops `userId` typing in a thread (fanned out to `thread:<id>`). */
  typing(threadId: number, userId: number, on: boolean): void;
  /** The viewer's `ThreadDetail`, as `GET /threads/:id` answers it. */
  detail(thread: ThreadRecord): ThreadDetail;
  /** The `thread.updated` events for the thread, on `room:<id>` and `thread:<id>`. */
  updated(thread: ThreadRecord): Outgoing[];
  /**
   * The private-thread controls, as other people's clients would act: `private-thread` starts
   * one, `thread-access` adds or removes someone, `thread-readable` says whether someone can see
   * a thread. `null` for any other control.
   */
  control(action: string, body: Json | undefined): MockResponse | null;
}

/** Creates the threads module. `held` applies the send hold to posting requests. */
export function createThreads(
  ctx: S2Context,
  uploads: Uploads,
  held: (run: () => MockResponse) => MockResponse | Promise<MockResponse>,
  sticker: ((id: number) => MessageSticker) | undefined = undefined,
): Threads {
  const threadOr404 = (threadId: number): ThreadRecord => {
    const thread = ctx.world().threads.get(threadId);

    if (thread === undefined) throw notFound("Thread not found");

    ctx.roomOr404(thread.roomId);

    if (!canAccessThread(ctx.world(), thread)) throw notFound("Thread not found");

    return thread;
  };

  const permissions = (thread: ThreadRecord): ThreadPermissions => threadPermissions(ctx, thread);

  const parentOf = (thread: ThreadRecord): MessageDTO | null => {
    if (thread.parentMessageId === null) return null;

    const room = ctx.world().rooms.get(thread.roomId);

    const parent = room?.messages.find((message) => message.id === thread.parentMessageId);

    if (parent === undefined) return null;

    // Marked like a page's messages: the server flags an ignored author's parent too.
    return ctx.world().ignored.has(parent.creatorId) ? { ...parent, ignored: true } : parent;
  };

  const detail = (thread: ThreadRecord): ThreadDetail => {
    const parent = parentOf(thread);
    const allowed = permissions(thread);
    const work = workDetail(ctx.world(), thread, allowed);

    return {
      thread: threadDto(thread, ctx.now()),
      membership: thread.viewerMembership,
      parentMessage: parent,
      permissions: allowed,
      work,
      users: ctx.usersFor([
        thread.creatorId,
        ...(parent === null ? [] : [parent.creatorId]),
        ...workUserIds(thread, work),
      ]),
    };
  };

  /** Keeps the parent's `thread` current; the `thread.indicator` event, if there's a parent. */
  const syncIndicator = (thread: ThreadRecord, removed = false): Outgoing[] => {
    const room = ctx.world().rooms.get(thread.roomId);

    const index =
      room?.messages.findIndex((message) => message.id === thread.parentMessageId) ?? -1;

    const parent = room?.messages[index];

    if (room === undefined || parent === undefined) return [];

    const indicator = removed ? null : indicatorOf(thread);

    room.messages[index] = {
      ...parent,
      thread: indicator,
      updatedAt: touched(ctx.now(), parent.updatedAt),
    };

    return [
      {
        topic: `room:${thread.roomId}`,
        type: "thread.indicator",
        data: { roomId: thread.roomId, parentMessageId: parent.id, thread: indicator },
      },
    ];
  };

  /**
   * `thread_changed`: a public thread's record on its room and thread topics; a private one's only
   * to its current readers, on their own `user` topic (the hub keeps it from anyone else).
   */
  const updated = (thread: ThreadRecord): Outgoing[] => {
    const data = threadDto(thread, ctx.now());

    if (thread.visibility === "private") return [{ topic: "user", type: "thread.updated", data }];

    return [
      { topic: `room:${thread.roomId}`, type: "thread.updated", data },
      { topic: `thread:${thread.id}`, type: "thread.updated", data },
    ];
  };

  /** `thread_changed` for a new thread: as `updated`, as `thread.created` and without the thread topic. */
  const createdEvent = (thread: ThreadRecord): Outgoing => ({
    topic: thread.visibility === "private" ? "user" : `room:${thread.roomId}`,
    type: "thread.created",
    data: threadDto(thread, ctx.now()),
  });

  /**
   * `Broadcasts::thread_access_changed` as the viewer sees it. Removed, the viewer gets
   * `thread.removed` on their `user` topic, which also ends their subscription to the thread; added,
   * they hear of it from the thread's record, which every current reader gets. The viewer's sidebar
   * row is sent again either way.
   */
  const accessChanged = (thread: ThreadRecord, userId: number, added: boolean): Outgoing[] => {
    const room = ctx.world().rooms.get(thread.roomId);
    const events: Outgoing[] = [];

    if (!added && userId === VIEWER_ID) {
      events.push({
        topic: "user",
        type: "thread.removed",
        data: { threadId: thread.id, roomId: thread.roomId },
        unsubscribe: `thread:${thread.id}`,
      });
    }

    if (userId === VIEWER_ID && room !== undefined) {
      events.push({ topic: "user", type: "sidebar.row.upserted", data: ctx.sidebarRow(room) });
    }

    return [...events, ...updated(thread)];
  };

  const join = (thread: ThreadRecord, userId: number, involvement: ThreadInvolvement | null) => {
    thread.memberIds.add(userId);

    if (userId !== VIEWER_ID) return;

    const current = thread.viewerMembership;

    thread.viewerMembership = {
      threadId: thread.id,
      involvement: involvement ?? current?.involvement ?? "mentions",
      unreadAt: current?.unreadAt ?? null,
      joinedAt: current?.joinedAt ?? iso(ctx.now()),
    };
  };

  const postReply = (thread: ThreadRecord, draft: MessageDraft, fresh = false): MessageDTO => {
    const world = ctx.world();

    if (!canAccessThread(world, thread, draft.creatorId)) throw notFound("Thread not found");
    const parent = parentOf(thread);

    const createdAt = iso(
      Math.max(
        ctx.now(),
        Date.parse(thread.lastActivityAt) + 1,
        Date.parse(parent?.createdAt ?? thread.createdAt) + 1,
      ),
    );

    join(thread, draft.creatorId, null);
    thread.closed = false;
    thread.lastActivityAt = createdAt;
    thread.updatedAt = createdAt;

    const message = buildMessage(
      world.nextMessageId++,
      thread.roomId,
      thread.id,
      draft,
      createdAt,
      ctx.mentionables(),
    );

    thread.messages.push(message);
    const notifications = receiveReply(thread, message);

    const events: Outgoing[] = [
      { topic: `thread:${thread.id}`, type: "message.created", data: message },
      ...filesChanged(thread),
      ...syncIndicator(thread),
      ...(fresh ? [] : updated(thread)),
      ...notifications,
    ];

    ctx.publish(events);

    return message;
  };

  const receiveReply = (thread: ThreadRecord, message: MessageDTO): Outgoing[] => {
    inviteThreadMentionees(ctx.world(), thread, message, message.creatorId, message.updatedAt);
    const membership = thread.viewerMembership;

    if (
      message.streaming ||
      message.systemNote ||
      message.creatorId === VIEWER_ID ||
      membership === null ||
      !canAccessThread(ctx.world(), thread)
    )
      return [];
    thread.viewerMembership = { ...membership, unreadAt: message.updatedAt };

    return [
      {
        topic: "user",
        type: "thread.unread",
        data: { threadId: thread.id, roomId: thread.roomId, refreshOnly: false },
      },
    ];
  };

  const refresh = (thread: ThreadRecord): Outgoing => ({
    topic: "user",
    type: "thread.unread",
    data: { threadId: thread.id, roomId: thread.roomId, refreshOnly: true },
  });

  // --- endpoints ---

  const list = (roomId: number, query: URLSearchParams): ThreadList => {
    const record = ctx.roomOr404(roomId);
    const state = query.get("state") ?? "active";
    const now = ctx.now();

    const keep = (thread: ThreadRecord) => {
      const status = threadStatus(thread, now);

      switch (state) {
        case "all":
          return true;
        case "closed":
          return status === "closed";
        case "locked":
          return thread.locked;
        default:
          return status === "active";
      }
    };

    const threads =
      record.room.kind === "direct"
        ? []
        : [...ctx.world().threads.values()]
            .filter(
              (thread) =>
                thread.roomId === roomId && canAccessThread(ctx.world(), thread) && keep(thread),
            )
            .sort(
              (a, b) => Date.parse(b.lastActivityAt) - Date.parse(a.lastActivityAt) || b.id - a.id,
            );

    return {
      threads: threads.map((thread) => ({
        thread: threadDto(thread, now),
        membership: thread.viewerMembership,
      })),
      users: ctx.usersFor(threads.map((thread) => thread.creatorId)),
    };
  };

  const create = (roomId: number, body: Json | undefined): MockResponse => {
    const world = ctx.world();
    const record = ctx.roomOr404(roomId);

    if (record.room.kind === "board") throw forbidden("Board rooms take posts, not threads");

    if (record.room.kind === "direct") throw forbidden("Direct messages don't have threads");

    const messageBody = nestedMessage(body);
    const clientMessageId = clientMessageIdOf(messageBody);
    const key = `${roomId}:${VIEWER_ID}:${clientMessageId}`;
    const duplicate = world.sentByClientId.get(key);

    const existing =
      duplicate?.threadId == null ? undefined : world.threads.get(duplicate.threadId);

    if (duplicate !== undefined && existing !== undefined) {
      if (!canAccessThread(world, existing)) throw notFound("Thread not found");
      const replay: ThreadCreated = { detail: detail(existing), message: duplicate };

      return ok(replay);
    }

    const parentId = intField(body, "parentMessageId");
    const parent = record.messages.find((message) => message.id === parentId);

    if (parent === undefined) throw notFound("Message not found");

    for (const thread of world.threads.values()) {
      if (thread.parentMessageId === parent.id) {
        if (!canAccessThread(world, thread)) throw notFound("Thread not found");
        throw conflict("That message already has a thread; open it instead");
      }
    }

    const rawVisibility = field(body, "visibility");

    if (
      rawVisibility !== undefined &&
      rawVisibility !== null &&
      rawVisibility !== "public" &&
      rawVisibility !== "private"
    ) {
      throw validation("visibility", "Visibility is not included in the list");
    }

    const visibility = rawVisibility === "private" ? "private" : "public";

    const parsed = parseMessage(messageBody, uploads.attachment, sticker);
    const requested = stringField(body, "name")?.trim() ?? "";

    if ([...requested].length > THREAD_NAME_LIMIT) {
      throw validation("name", `Name is too long (maximum is ${THREAD_NAME_LIMIT} characters)`);
    }

    const createdAt = iso(Math.max(ctx.now(), Date.parse(parent.createdAt) + 1));

    const thread: ThreadRecord = {
      id: world.nextThreadId++,
      roomId,
      parentMessageId: parent.id,
      creatorId: VIEWER_ID,
      name: requested === "" ? defaultThreadName(parent) : requested,
      closed: false,
      locked: false,
      lastActivityAt: createdAt,
      autoArchiveAfterMinutes: DEFAULT_AUTO_ARCHIVE_MINUTES,
      visibility,
      accessMembers:
        visibility === "private"
          ? new Map([[VIEWER_ID, { userId: VIEWER_ID, addedById: VIEWER_ID, createdAt }]])
          : new Map(),
      createdAt,
      messages: [],
      memberIds: new Set(),
      viewerMembership: null,
    };

    world.threads.set(thread.id, thread);
    ctx.publish([createdEvent(thread)]);

    const message = postReply(
      thread,
      {
        creatorId: VIEWER_ID,
        markdown: parsed.markdown,
        clientMessageId,
        replyToMessageId: null,
        streaming: false,
        ...draftFiles(parsed),
        action: false,
        systemNote: false,
        forward: null,
        cards: driveCards(parsed.driveFileIds),
      },
      true,
    );

    world.sentByClientId.set(key, message);

    const created: ThreadCreated = { detail: detail(thread), message };

    return ok(created, 201);
  };

  const filesChanged = (thread: ThreadRecord): Outgoing[] =>
    canAccessThread(ctx.world(), thread)
      ? [
          {
            topic: thread.visibility === "private" ? "user" : `room:${thread.roomId}`,
            type: "room.files.changed",
            data: { roomId: thread.roomId, threadId: thread.id },
          },
        ]
      : [];

  const reply = (threadId: number, body: Json | undefined): MockResponse => {
    const world = ctx.world();
    const thread = threadOr404(threadId);

    if (thread.locked) throw forbidden("This thread is locked");

    const clientMessageId = clientMessageIdOf(body);
    const key = `${thread.roomId}:${VIEWER_ID}:${clientMessageId}`;
    const duplicate = world.sentByClientId.get(key);

    if (duplicate !== undefined) return ok(duplicate);

    const parsed = parseMessage(body, uploads.attachment, sticker);
    const replyTo = parsed.replyToMessageId;

    if (replyTo !== null && !thread.messages.some((message) => message.id === replyTo)) {
      throw validation("replyToMessageId", "Reply to message must be on this timeline");
    }

    const message = postReply(thread, {
      creatorId: VIEWER_ID,
      markdown: parsed.markdown,
      clientMessageId,
      replyToMessageId: replyTo,
      streaming: false,
      ...draftFiles(parsed),
      action: false,
      systemNote: false,
      forward: null,
      cards: driveCards(parsed.driveFileIds),
    });

    world.sentByClientId.set(key, message);

    return ok(message, 201);
  };

  const update = (threadId: number, body: Json | undefined): ThreadDetail => {
    const thread = threadOr404(threadId);

    if (isRecord(body) && "visibility" in body) {
      throw accessRefused("visibilityImmutable", "Thread visibility cannot be changed");
    }

    const allowed = permissions(thread);
    const name = stringField(body, "name");
    const status = stringField(body, "status");

    if (name !== null) {
      if (!allowed.canRename) throw forbidden("You can't rename this thread");

      const trimmed = name.trim();

      if (trimmed === "") throw validation("name", "Name can't be blank");

      if ([...trimmed].length > THREAD_NAME_LIMIT) {
        throw validation("name", `Name is too long (maximum is ${THREAD_NAME_LIMIT} characters)`);
      }

      thread.name = trimmed;
    }

    if (status !== null) {
      const target = STATUSES.find((candidate) => candidate === status);

      if (target === undefined) throw validation("status", "Status is not included in the list");

      const current = threadStatus(thread, ctx.now());

      if (target === "closed" && current !== "closed") {
        if (!allowed.canClose) throw forbidden("You can't close this thread");

        thread.closed = true;
      } else if (target === "locked" && current !== "locked") {
        if (!allowed.canLock) throw forbidden("You can't lock this thread");

        thread.locked = true;
      } else if (target === "active" && current === "locked") {
        if (!allowed.canUnlock) throw forbidden("You can't unlock this thread");

        thread.locked = false;
      } else if (target === "active" && current === "closed") {
        if (!allowed.canReopen) throw forbidden("You can't reopen this thread");

        thread.closed = false;

        if (threadStatus(thread, ctx.now()) === "closed") thread.lastActivityAt = iso(ctx.now());
      }
    }

    ctx.publish(updated(thread));

    return detail(thread);
  };

  const remove = (threadId: number): MockResponse => {
    const world = ctx.world();
    const thread = threadOr404(threadId);

    if (!permissions(thread).canDelete) throw forbidden("You can't delete this thread");

    const events = syncIndicator(thread, true);

    world.threads.delete(thread.id);

    for (const message of thread.messages) {
      world.pins.delete(message.id);
      world.saved.delete(message.id);
    }

    const data = { threadId: thread.id, roomId: thread.roomId };

    ctx.publish([
      ...events,
      { topic: `room:${thread.roomId}`, type: "thread.removed", data },
      { topic: `thread:${thread.id}`, type: "thread.removed", data },
    ]);

    return noContent();
  };

  const joinThread = (threadId: number, body: Json | undefined): ThreadMembershipState => {
    const thread = threadOr404(threadId);
    const raw = stringField(body, "involvement");
    const involvement = INVOLVEMENTS.find((candidate) => candidate === raw) ?? null;

    if (raw !== null && involvement === null) {
      throw validation("involvement", "Involvement is not included in the list");
    }

    join(thread, VIEWER_ID, involvement);

    if (thread.viewerMembership === null) throw notFound("Thread membership not found");

    return { membership: thread.viewerMembership };
  };

  const leave = (threadId: number): MockResponse => {
    const thread = threadOr404(threadId);

    thread.memberIds.delete(VIEWER_ID);
    thread.viewerMembership = null;

    return noContent();
  };

  const read = (threadId: number): ThreadMembershipState => {
    const thread = threadOr404(threadId);
    const membership = thread.viewerMembership;

    if (membership === null) throw notFound("Thread membership not found");

    thread.viewerMembership = { ...membership, unreadAt: null };
    ctx.publish([
      { topic: "user", type: "thread.read", data: { threadId: thread.id, roomId: thread.roomId } },
    ]);

    return { membership: thread.viewerMembership };
  };

  const accessRefused = (reason: ThreadAccessRefusal, message: string): HttpError =>
    new HttpError(
      reason === "inviteForbidden" || reason === "removeForbidden" ? 403 : 422,
      new ThreadAccess({ reason, message }),
    );

  const privateThread = (threadId: number, creatorMayManage = false): ThreadRecord => {
    const world = ctx.world();
    const candidate = world.threads.get(threadId);

    const creator =
      creatorMayManage &&
      candidate?.creatorId === VIEWER_ID &&
      world.users.get(VIEWER_ID)?.status === "active" &&
      (world.rooms.get(candidate.roomId)?.memberIds.includes(VIEWER_ID) ?? false);

    const thread = creator && candidate !== undefined ? candidate : threadOr404(threadId);

    if (thread.visibility !== "private") throw accessRefused("notPrivate", "This thread is public");

    return thread;
  };

  const accessMembers = (thread: ThreadRecord): ThreadAccessMembers => {
    const members = [...thread.accessMembers.values()].sort(
      (left, right) => left.userId - right.userId,
    );

    return { members, users: ctx.usersFor(members.map((member) => member.userId)) };
  };

  const addAccessMember = (threadId: number, body: Json | undefined): ThreadAccessMembers => {
    const thread = privateThread(threadId, true);
    const world = ctx.world();
    const administrator = world.users.get(VIEWER_ID)?.role === "administrator";

    if (!administrator && thread.creatorId !== VIEWER_ID && !thread.accessMembers.has(VIEWER_ID)) {
      throw accessRefused("inviteForbidden", "You can't invite people to this thread");
    }

    const userId = intField(body, "userId");

    if (userId === null) throw validation("userId", "User is required");
    const user = world.users.get(userId);

    if (
      user?.status !== "active" ||
      !(world.rooms.get(thread.roomId)?.memberIds.includes(userId) ?? false)
    ) {
      throw accessRefused(
        "parentRoomRequired",
        "The person must be an active member of the parent room",
      );
    }

    if (!thread.accessMembers.has(userId)) {
      thread.accessMembers.set(userId, { userId, addedById: VIEWER_ID, createdAt: iso(ctx.now()) });
      join(thread, userId, null);
      ctx.publish(accessChanged(thread, userId, true));
    }

    if (canAccessThread(world, thread)) return accessMembers(thread);
    const member = thread.accessMembers.get(userId);

    if (member === undefined) throw notFound("Thread member not found");

    return { members: [member], users: ctx.usersFor([userId]) };
  };

  const removeAccessMember = (threadId: number, userId: number): MockResponse => {
    const thread = privateThread(threadId, true);
    const administrator = ctx.world().users.get(VIEWER_ID)?.role === "administrator";

    if (!administrator && thread.creatorId !== VIEWER_ID && userId !== VIEWER_ID) {
      throw accessRefused("removeForbidden", "You can't remove people from this thread");
    }

    if (thread.accessMembers.delete(userId)) {
      if (userId === VIEWER_ID && thread.viewerMembership !== null)
        thread.viewerMembership = { ...thread.viewerMembership, unreadAt: null };
      ctx.publish(accessChanged(thread, userId, false));
    }

    return noContent();
  };

  /**
   * `creatorId` starts a private thread on the newest root message in `roomId` that has none,
   * with a first reply, sharing it with the viewer when `withViewer`. Like the server, the
   * thread's events reach only the people who can see it.
   */
  const startPrivate = (body: Json | undefined): MockResponse => {
    const world = ctx.world();
    const roomId = intField(body, "roomId");
    const creatorId = intField(body, "creatorId");
    const record = roomId === null ? undefined : world.rooms.get(roomId);

    if (record === undefined || creatorId === null || !record.memberIds.includes(creatorId)) {
      throw validation("roomId", "The creator must be a member of the room");
    }

    const taken = new Set([...world.threads.values()].map((thread) => thread.parentMessageId));

    const parent = record.messages.findLast(
      (message) => message.threadId === null && !message.systemNote && !taken.has(message.id),
    );

    if (parent === undefined) throw notFound("No message to start a thread on");

    const createdAt = iso(Math.max(ctx.now(), Date.parse(parent.createdAt) + 1));

    const accessMembers = new Map([
      [creatorId, { userId: creatorId, addedById: creatorId, createdAt }],
    ]);

    if (booleanField(body, "withViewer") === true) {
      accessMembers.set(VIEWER_ID, { userId: VIEWER_ID, addedById: creatorId, createdAt });
    }

    const thread: ThreadRecord = {
      id: world.nextThreadId++,
      roomId: record.room.id,
      parentMessageId: parent.id,
      creatorId,
      name: stringField(body, "name") ?? defaultThreadName(parent),
      closed: false,
      locked: false,
      lastActivityAt: createdAt,
      autoArchiveAfterMinutes: DEFAULT_AUTO_ARCHIVE_MINUTES,
      visibility: "private",
      accessMembers,
      createdAt,
      messages: [],
      memberIds: new Set(accessMembers.keys()),
      viewerMembership: accessMembers.has(VIEWER_ID)
        ? { threadId: 0, involvement: "mentions", unreadAt: null, joinedAt: createdAt }
        : null,
    };

    if (thread.viewerMembership !== null) {
      thread.viewerMembership = { ...thread.viewerMembership, threadId: thread.id };
    }

    world.threads.set(thread.id, thread);
    ctx.publish([createdEvent(thread)]);
    postReply(
      thread,
      plainDraft(creatorId, stringField(body, "markdown") ?? "Just between us.", ctx.uuid()),
      true,
    );

    return ok({ threadId: thread.id, parentMessageId: parent.id }, 201);
  };

  /** `actorId` adds (`added`) or removes `userId`; the viewer's own events follow. */
  const changeAccess = (body: Json | undefined): MockResponse => {
    const world = ctx.world();
    const thread = world.threads.get(intField(body, "threadId") ?? 0);
    const userId = intField(body, "userId");
    const actorId = intField(body, "actorId") ?? thread?.creatorId ?? VIEWER_ID;

    if (thread === undefined || thread.visibility !== "private" || userId === null) {
      throw notFound("Private thread not found");
    }

    if (booleanField(body, "added") === false) {
      if (thread.accessMembers.delete(userId)) {
        if (userId === VIEWER_ID && thread.viewerMembership !== null)
          thread.viewerMembership = { ...thread.viewerMembership, unreadAt: null };
        ctx.publish(accessChanged(thread, userId, false));
      }

      return noContent();
    }

    if (!(world.rooms.get(thread.roomId)?.memberIds.includes(userId) ?? false)) {
      throw accessRefused("parentRoomRequired", "The person must be a member of the room");
    }

    if (!thread.accessMembers.has(userId)) {
      thread.accessMembers.set(userId, { userId, addedById: actorId, createdAt: iso(ctx.now()) });
      join(thread, userId, null);
      ctx.publish(accessChanged(thread, userId, true));
    }

    return ok(accessMembers(thread));
  };

  const lookups = () => ({
    usersFor: ctx.usersFor,
    saved: ctx.world().saved,
    ignored: ctx.world().ignored,
  });

  return {
    control(action, body) {
      if (action === "private-thread") return startPrivate(body);

      if (action === "thread-access") return changeAccess(body);

      if (action === "thread-readable") {
        const thread = ctx.world().threads.get(intField(body, "threadId") ?? 0);

        if (thread === undefined) throw notFound("Thread not found");

        return ok({
          readable: canAccessThread(ctx.world(), thread, intField(body, "userId") ?? VIEWER_ID),
        });
      }

      return null;
    },
    finalizeReply(thread, message) {
      const notifications = receiveReply(thread, message);

      ctx.publish([
        ...notifications,
        { topic: `thread:${thread.id}`, type: "message.updated", data: message },
        ...filesChanged(thread),
        ...syncIndicator(thread),
        ...updated(thread),
      ]);
    },
    routes: [
      route("GET", /^\/rooms\/(\d+)\/threads$/, (request) =>
        ok(list(firstId(request), request.query)),
      ),
      route("POST", /^\/rooms\/(\d+)\/threads$/, (request) =>
        held(() => create(firstId(request), request.body)),
      ),
      route("GET", /^\/threads\/(\d+)$/, (request) => ok(detail(threadOr404(firstId(request))))),
      route("PATCH", /^\/threads\/(\d+)$/, (request) => ok(update(firstId(request), request.body))),
      route("DELETE", /^\/threads\/(\d+)$/, (request) => remove(firstId(request))),
      route("GET", /^\/threads\/(\d+)\/messages$/, (request) =>
        ok(pageOf(threadOr404(firstId(request)).messages, request.query, lookups())),
      ),
      route("POST", /^\/threads\/(\d+)\/messages$/, (request) => {
        threadOr404(firstId(request));

        return held(() => reply(firstId(request), request.body));
      }),
      route("POST", /^\/threads\/(\d+)\/join$/, (request) =>
        ok(joinThread(firstId(request), request.body)),
      ),
      route("DELETE", /^\/threads\/(\d+)\/join$/, (request) => leave(firstId(request))),
      route("POST", /^\/threads\/(\d+)\/read$/, (request) => ok(read(firstId(request)))),
      route("GET", /^\/threads\/(\d+)\/members$/, (request) =>
        ok(accessMembers(privateThread(firstId(request)))),
      ),
      route("POST", /^\/threads\/(\d+)\/members$/, (request) =>
        ok(addAccessMember(firstId(request), request.body)),
      ),
      route("DELETE", /^\/threads\/(\d+)\/members\/(\d+)$/, (request) =>
        removeAccessMember(firstId(request), request.ids[1] ?? 0),
      ),
    ],
    threadOr404,
    postReply: (thread, draft, fresh) => postReply(thread, draft, fresh),
    replyRemoved(thread) {
      const events = [...syncIndicator(thread), ...updated(thread)];

      if (thread.memberIds.size > 0 && canAccessThread(ctx.world(), thread))
        events.push(refresh(thread));

      ctx.publish(events);
    },
    parentRemoved(messageId) {
      for (const thread of ctx.world().threads.values()) {
        if (thread.parentMessageId !== messageId) continue;

        thread.parentMessageId = null;
        ctx.publish([
          ...updated(thread),
          ...(canAccessThread(ctx.world(), thread) ? [refresh(thread)] : []),
        ]);
      }
    },
    typing(threadId, userId, on) {
      ctx.publish([{ topic: `thread:${threadId}`, type: "typing", data: { userId, on } }]);
    },
    detail,
    updated,
  };
}
