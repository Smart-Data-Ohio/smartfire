import type { BoardListing } from "../../src/gen/BoardListing.ts";
import type { BoardPostForm } from "../../src/gen/BoardPostForm.ts";
import type { BoardStatusFilter } from "../../src/gen/BoardStatusFilter.ts";
import type { WorkStatus } from "../../src/gen/WorkStatus.ts";
import { notFound, ok, validation } from "../http.ts";
import { field, intField, type Json, stringField } from "../json.ts";
import { firstId, route, type S2Context } from "../s2/context.ts";
import { iso, plainDraft, type ThreadRecord, threadDto } from "../s2/model.ts";
import { clientMessageIdOf, parseMessage } from "../s2/posting.ts";
import type { Threads } from "../s2/threads.ts";
import type { Uploads } from "../s2/uploads.ts";
import type { Work } from "../s4/work.ts";
import { rowTimestamp, VIEWER_ID } from "../seed.ts";
import { autoAssignedBoardOwner, createBoardAutomations } from "./automations.ts";
import { BOARD_ROOM_ID } from "./seed.ts";
import { emptyWorkDetail, newWorkFacts, ownerCandidates, tagsOf } from "./work.ts";

const STATUSES: readonly WorkStatus[] = ["planned", "in_progress", "blocked", "done"];

function statusOf(raw: Json | undefined, fallback: WorkStatus): WorkStatus {
  const status = STATUSES.find((value) => value === raw);

  if (raw !== undefined && status === undefined)
    throw validation("status", "Status is not included in the list");

  return status ?? fallback;
}

export function createBoards(ctx: S2Context, threads: Threads, uploads: Uploads, work: Work) {
  const boardRoom = (roomId: number) => {
    const room = ctx.roomOr404(roomId);

    const viewer = ctx.world().users.get(VIEWER_ID);

    if (room.room.kind !== "board" || viewer?.status !== "active" || viewer.role === "bot")
      throw notFound("Board not found");

    return room;
  };

  const posts = (roomId: number) =>
    [...ctx.world().threads.values()].filter((thread) => thread.roomId === roomId);

  const tagCounts = (roomId: number) => {
    const counts = new Map<string, number>();

    for (const post of posts(roomId)) {
      for (const tag of post.work?.tags ?? []) counts.set(tag, (counts.get(tag) ?? 0) + 1);
    }

    return [...counts]
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([name, count]) => ({ name, count }));
  };

  const postForm = (roomId: number): BoardPostForm => {
    boardRoom(roomId);
    const candidates = ownerCandidates(ctx.world(), roomId);

    return {
      ownerCandidates: candidates,
      tagSuggestions: tagCounts(roomId).map(({ name }) => name),
      users: ctx.usersFor(candidates.map(({ userId }) => userId)),
    };
  };

  const listing = (roomId: number, query: URLSearchParams): BoardListing => {
    const room = boardRoom(roomId);
    const rawStatus = query.get("status");

    const status: BoardStatusFilter =
      rawStatus === "done" || rawStatus === "all" ? rawStatus : "open";

    const rawOwner = query.get("owner") ?? "anyone";
    const owner = /^(anyone|me|agents|[1-9]\d*)$/.test(rawOwner) ? rawOwner : "anyone";
    const tag = (query.get("tag") ?? "").trim().toLowerCase();
    const rawPage = Number(query.get("page") ?? 1);
    const page = Number.isFinite(rawPage) ? Math.max(1, Math.min(20, Math.trunc(rawPage))) : 1;
    const all = posts(roomId);

    const matches = all
      .filter((thread) => {
        const work = thread.work;

        if (work == null) return false;

        if (status === "open" && work.status === "done") return false;

        if (status === "done" && work.status !== "done") return false;

        if (owner === "me" && work.owner?.id !== VIEWER_ID) return false;

        if (owner === "agents" && work.owner?.role !== "bot" && work.owner?.agent == null)
          return false;

        if (/^\d+$/.test(owner) && work.owner?.id !== Number(owner)) return false;

        return tag === "" || work.tags.includes(tag);
      })
      .sort((a, b) => Date.parse(b.lastActivityAt) - Date.parse(a.lastActivityAt) || b.id - a.id);

    const window = matches.slice(0, page * 50);

    const options = ctx
      .usersFor(room.memberIds)
      .filter((user) => user.status === "active")
      .sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()) || a.id - b.id);

    return {
      roomId,
      status,
      owner,
      tag,
      page,
      posts: window.map((thread) => ({
        thread: threadDto(thread, ctx.now()),
        membership: thread.viewerMembership,
      })),
      hasMore: matches.length > page * 50,
      anyPosts: all.length > 0,
      ownerOptions: options.map((user) => ({
        userId: user.id,
        agent: user.role === "bot" || user.agent !== null,
      })),
      tagCounts: tagCounts(roomId),
      digest: room.boardDigest ?? null,
      canAdminister:
        room.room.creatorId === VIEWER_ID ||
        ctx.world().users.get(VIEWER_ID)?.role === "administrator",
      users: ctx.usersFor([
        ...window.map((thread) => thread.creatorId),
        ...options.map((user) => user.id),
      ]),
    };
  };

  const checkedOwner = (roomId: number, body: Json | undefined): number | null => {
    const raw = field(body, "ownerId");

    if (raw == null) return null;
    const id = intField(body, "ownerId");

    if (
      id === null ||
      !ownerCandidates(ctx.world(), roomId).some((candidate) => candidate.userId === id)
    ) {
      const agent = id !== null && ctx.world().users.get(id)?.role === "bot";
      throw validation(
        "ownerId",
        agent
          ? "must be an active agent member of the parent room with permission to post"
          : "must be an active human member of the parent room",
      );
    }

    return id;
  };

  /** Posts by `room:creator:clientPostId`, so a retry answers the post its first attempt made. */
  const postsByClientId = new Map<string, number>();

  const createPost = (roomId: number, body: Json | undefined, actorId = VIEWER_ID) => {
    const room = boardRoom(roomId);

    if (!room.memberIds.includes(actorId)) throw notFound("Board not found");
    const clientPostId = stringField(body, "clientPostId") ?? null;
    const postKey = clientPostId === null ? null : `${roomId}:${actorId}:${clientPostId}`;
    const made = postKey === null ? undefined : postsByClientId.get(postKey);

    if (made !== undefined && ctx.world().threads.has(made))
      return ok(threads.detail(threads.threadOr404(made)));
    const messageBody = field(body, "message");
    const clientMessageId = messageBody == null ? null : clientMessageIdOf(messageBody);
    const key = `${roomId}:${actorId}:${clientMessageId}`;
    const existing = clientMessageId === null ? undefined : ctx.world().sentByClientId.get(key);

    if (existing?.threadId != null)
      return ok(threads.detail(threads.threadOr404(existing.threadId)));
    const name = stringField(body, "name")?.trim() ?? "";

    if (name === "") throw validation("name", "Name can't be blank");

    if ([...name].length > 100)
      throw validation("name", "Name is too long (maximum is 100 characters)");
    const status = statusOf(field(body, "status"), "planned");
    const tags = tagsOf(body);
    const ownerId = checkedOwner(roomId, body);

    const owner =
      ownerId === null
        ? autoAssignedBoardOwner(ctx.world(), roomId, tags)
        : (ctx.world().users.get(ownerId) ?? null);

    const source = stringField(messageBody, "markdownSource") ?? "";

    if ([...source].length > 50000)
      throw validation("message", "Message is too long (maximum is 50000 characters)");

    const parsed =
      messageBody == null ||
      (source.trim() === "" && field(messageBody, "attachmentSignedId") == null)
        ? null
        : parseMessage(messageBody, uploads.attachment);

    const at = iso(ctx.now());

    const thread: ThreadRecord = {
      id: ctx.world().nextThreadId++,
      roomId,
      isBoard: true,
      parentMessageId: null,
      creatorId: actorId,
      name,
      closed: false,
      locked: false,
      lastActivityAt: at,
      autoArchiveAfterMinutes: 1440,
      createdAt: at,
      updatedAt: at,
      messages: [],
      memberIds: new Set([actorId]),
      viewerMembership:
        actorId === VIEWER_ID
          ? { threadId: 0, involvement: "mentions", unreadAt: null, joinedAt: at }
          : null,
      work: {
        ...newWorkFacts(status, rowTimestamp(ctx.now())),
        ...emptyWorkDetail,
        ownerId: owner?.id ?? null,
        owner,
        ownerActive: owner !== null,
        tags,
      },
    };

    if (thread.viewerMembership !== null)
      thread.viewerMembership = { ...thread.viewerMembership, threadId: thread.id };
    ctx.world().threads.set(thread.id, thread);

    if (postKey !== null) postsByClientId.set(postKey, thread.id);

    if (parsed !== null && clientMessageId !== null) {
      const message = threads.postReply(
        thread,
        {
          ...plainDraft(actorId, parsed.markdown, clientMessageId),
          attachment: parsed.attachment,
        },
        true,
      );

      ctx.world().sentByClientId.set(key, message);
    }

    ctx.publish([
      { topic: `room:${roomId}`, type: "thread.created", data: threadDto(thread, ctx.now()) },
    ]);

    return ok(threads.detail(thread), 201);
  };

  return {
    routes: [
      ...createBoardAutomations(ctx).routes,
      route("GET", /^\/rooms\/(\d+)\/board$/, (request) =>
        ok(listing(firstId(request), request.query)),
      ),
      route("GET", /^\/rooms\/(\d+)\/posts\/new$/, (request) => ok(postForm(firstId(request)))),
      route("POST", /^\/rooms\/(\d+)\/posts$/, (request) =>
        createPost(firstId(request), request.body),
      ),
    ],
    control(action: string, body: Json | undefined) {
      const actorId = intField(body, "userId") ?? 2;

      if (action === "board-update")
        return ok(work.updateAs(intField(body, "threadId") ?? 9001, body, actorId));

      if (action === "board-create")
        return createPost(
          intField(body, "roomId") ?? BOARD_ROOM_ID,
          {
            name: stringField(body, "name") ?? "Live post from Maya",
            status: field(body, "status") ?? "planned",
            ownerId: field(body, "ownerId") ?? null,
            tags: field(body, "tags") ?? [],
            message: stringField(body, "brief")
              ? {
                  clientMessageId: ctx.uuid(),
                  markdownSource: stringField(body, "brief"),
                  replyToMessageId: null,
                  replyNotifyAuthor: null,
                  attachmentSignedId: null,
                }
              : null,
          },
          actorId,
        );

      return null;
    },
  };
}
