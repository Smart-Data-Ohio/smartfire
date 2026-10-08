import type { BoardListing } from "../../src/gen/BoardListing.ts";
import type { BoardPostForm } from "../../src/gen/BoardPostForm.ts";
import type { BoardStatusFilter } from "../../src/gen/BoardStatusFilter.ts";
import type { ThreadDetail } from "../../src/gen/ThreadDetail.ts";
import type { WorkHistoryEntry } from "../../src/gen/WorkHistoryEntry.ts";
import type { WorkList } from "../../src/gen/WorkList.ts";
import type { WorkStatus } from "../../src/gen/WorkStatus.ts";
import { forbidden, notFound, ok, validation } from "../http.ts";
import { field, intField, type Json, stringArrayField, stringField } from "../json.ts";
import { renderMarkdown } from "../markdown.ts";
import { firstId, route, type S2Context } from "../s2/context.ts";
import { iso, plainDraft, type ThreadRecord, threadDto, touched } from "../s2/model.ts";
import { clientMessageIdOf, parseMessage } from "../s2/posting.ts";
import type { Threads } from "../s2/threads.ts";
import type { Uploads } from "../s2/uploads.ts";
import { VIEWER_ID } from "../seed.ts";
import { autoAssignedBoardOwner, createBoardAutomations } from "./automations.ts";
import { BOARD_ROOM_ID } from "./seed.ts";
import {
  emptyWorkDetail,
  newWorkFacts,
  ownerCandidates,
  threadPermissions,
  workDetail,
} from "./work.ts";

const STATUSES: readonly WorkStatus[] = ["planned", "in_progress", "blocked", "done"];

function statusOf(raw: Json | undefined, fallback: WorkStatus): WorkStatus {
  const status = STATUSES.find((value) => value === raw);

  if (raw !== undefined && status === undefined)
    throw validation("status", "Status is not included in the list");

  return status ?? fallback;
}

function tagsOf(body: Json | undefined): string[] {
  const input = stringArrayField(body, "tags");

  if (input === null && field(body, "tags") !== undefined)
    throw validation("tags", "Tags must be a list");

  const tags = [
    ...new Set((input ?? []).map((tag) => tag.trim().toLowerCase()).filter((tag) => tag !== "")),
  ].sort();

  if (tags.length > 5) throw validation("tags", "Tags must have at most 5 entries");

  if (tags.some((tag) => [...tag].length > 30))
    throw validation("tags", "Tags are too long (maximum is 30 characters)");

  if (tags.some((tag) => !/^[a-z0-9][a-z0-9-]*$/.test(tag)))
    throw validation("tags", "Tags must contain only letters, numbers and hyphens");

  return tags;
}

export function createBoards(ctx: S2Context, threads: Threads, uploads: Uploads) {
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

  const publishUpdate = (thread: ThreadRecord) => {
    const data = threadDto(thread, ctx.now());
    ctx.publish([
      { topic: `room:${thread.roomId}`, type: "thread.updated", data },
      { topic: `thread:${thread.id}`, type: "thread.updated", data },
    ]);
  };

  const createPost = (roomId: number, body: Json | undefined, actorId = VIEWER_ID) => {
    const room = boardRoom(roomId);

    if (!room.memberIds.includes(actorId)) throw notFound("Board not found");
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
      work: { ...newWorkFacts(status), owner, ownerActive: owner !== null, tags },
      workDetail: { ...emptyWorkDetail },
    };

    if (thread.viewerMembership !== null)
      thread.viewerMembership = { ...thread.viewerMembership, threadId: thread.id };
    ctx.world().threads.set(thread.id, thread);

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

  const historyEntry = (
    thread: ThreadRecord,
    kind: WorkHistoryEntry["kind"],
    actorId: number,
  ): WorkHistoryEntry => {
    const work = thread.work;
    const owner = work?.owner;

    return {
      id: Math.max(0, ...(thread.workDetail?.history ?? []).map(({ id }) => id)) + 1,
      kind,
      createdAt: touched(ctx.now(), thread.updatedAt ?? thread.createdAt),
      actorId,
      fromStatus: work?.status ?? null,
      toStatus: work?.status ?? null,
      fromOwner: owner == null ? null : { userId: owner.id, name: owner.name },
      toOwner: owner == null ? null : { userId: owner.id, name: owner.name },
      note: null,
      handoff: null,
    };
  };

  /** Validate every requested change before mutating, so a rejected multi-field write is atomic. */
  const updateWork = (
    threadId: number,
    body: Json | undefined,
    actorId = VIEWER_ID,
    remote = false,
  ): ThreadDetail => {
    const thread = threads.threadOr404(threadId);
    const viewer = ctx.world().users.get(VIEWER_ID);

    if (viewer?.status !== "active" || viewer.role === "bot") throw notFound();
    const allowed = threadPermissions(ctx, thread);
    const rawStatus = field(body, "status");
    const rawOwner = field(body, "ownerId");
    const rawResult = field(body, "resultMarkdown");
    const rawTags = field(body, "tags");

    const status =
      rawStatus === undefined
        ? (thread.work?.status ?? null)
        : rawStatus === null
          ? null
          : statusOf(rawStatus, "planned");

    if (!remote) {
      if (
        rawStatus !== undefined &&
        !(thread.work == null
          ? allowed.canConvertWork
          : rawStatus === null
            ? allowed.canAssignWork
            : allowed.canUpdateWorkStatus)
      )
        throw forbidden();

      if (rawOwner !== undefined && !allowed.canAssignWork) throw forbidden();

      if ((rawResult !== undefined || rawTags !== undefined) && !allowed.canManageWork)
        throw forbidden();
    }

    const ownerId =
      rawOwner === undefined ? (thread.work?.owner?.id ?? null) : checkedOwner(thread.roomId, body);

    if (status === null && ownerId !== null) throw validation("ownerId", "requires work tracking");

    if (
      rawResult !== undefined &&
      rawResult !== null &&
      stringField(body, "resultMarkdown") === null
    )
      throw validation("resultMarkdown", "Result must be Markdown text");

    const source = stringField(body, "resultMarkdown");

    const result =
      rawResult === undefined
        ? (thread.workDetail?.resultMarkdown ?? null)
        : source === null || source.trim() === ""
          ? null
          : source;

    if (result !== null && [...result].length > 20000)
      throw validation("resultMarkdown", "Result is too long (maximum is 20000 characters)");

    if (rawTags !== undefined && !thread.isBoard)
      throw validation("tags", "Tags are only available on board posts");
    const tags = rawTags === undefined ? (thread.work?.tags ?? []) : tagsOf(body);

    const addedTags = tags.filter((tag) => !(thread.work?.tags ?? []).includes(tag));

    const owner =
      ownerId === null
        ? rawTags === undefined
          ? null
          : autoAssignedBoardOwner(ctx.world(), thread.roomId, addedTags)
        : (ctx.world().users.get(ownerId) ?? null);

    const before = thread.work;
    const detail = thread.workDetail ?? emptyWorkDetail;
    const statusChanged = status !== (before?.status ?? null);
    const ownerChanged = (owner?.id ?? null) !== (before?.owner?.id ?? null);
    const resultChanged = result !== detail.resultMarkdown;
    const tagsChanged = JSON.stringify(tags) !== JSON.stringify(before?.tags ?? []);

    if (!statusChanged && !ownerChanged && !resultChanged && !tagsChanged)
      return threads.detail(thread);
    const entry = historyEntry(thread, statusChanged ? "update" : "assignment", actorId);
    entry.toStatus = status;
    entry.toOwner = owner === null ? null : { userId: owner.id, name: owner.name };
    const at = entry.createdAt;
    thread.work =
      status === null
        ? null
        : {
            ...(before ?? newWorkFacts()),
            status,
            owner,
            ownerActive:
              owner !== null &&
              ownerCandidates(ctx.world(), thread.roomId).some(({ userId }) => userId === owner.id),
            tags,
            resultUpdatedAt: resultChanged
              ? result === null
                ? null
                : at
              : (before?.resultUpdatedAt ?? null),
          };
    thread.workDetail = {
      ...detail,
      resultMarkdown: result,
      resultHtml: result === null ? null : renderMarkdown(result, ctx.mentionables()),
      resultUpdatedById: resultChanged
        ? result === null
          ? null
          : actorId
        : detail.resultUpdatedById,
      history: [...(statusChanged || ownerChanged ? [entry] : []), ...detail.history],
    };

    if (resultChanged) {
      const resultEntry = historyEntry(thread, "result", actorId);
      thread.workDetail.history.unshift(resultEntry);
    }

    thread.updatedAt = at;
    publishUpdate(thread);

    return threads.detail(thread);
  };

  const handoff = (threadId: number, body: Json | undefined): ThreadDetail => {
    const thread = threads.threadOr404(threadId);

    const viewer = ctx.world().users.get(VIEWER_ID);

    if (viewer?.status !== "active" || viewer.role === "bot") throw notFound();

    if (thread.work == null) throw validation("base", "This thread is not tracked as work");

    if (!threadPermissions(ctx, thread).canManageWork) throw forbidden();

    if (thread.work.owner?.agent?.agentId === intField(body, "receiverAgentId"))
      throw validation("receiverAgentId", "Receiver is already the owner of this work");

    const receiver = workDetail(ctx, thread)?.handoffReceivers.find(
      ({ agentId }) => agentId === intField(body, "receiverAgentId"),
    );

    if (receiver === undefined)
      throw validation(
        "receiverAgentId",
        "Receiver must be an active agent member of this room with permission to post",
      );
    const summary = stringField(body, "summary")?.trim() ?? "";

    if (summary === "" || [...summary].length > 2000)
      throw validation(
        "summary",
        summary === ""
          ? "Summary can't be blank"
          : "Summary is too long (maximum is 2000 characters)",
      );

    const packageList = (key: string) => {
      const values = stringArrayField(body, key);

      if (values === null) throw validation(key, `${key} must be a list`);

      const normalized = [
        ...new Set(values.map((value) => value.trim()).filter((value) => value !== "")),
      ];

      if (normalized.length > 10 || normalized.some((value) => [...value].length > 500))
        throw validation(key, `${key} must have at most 10 entries of 500 characters`);

      if (key === "links" && normalized.some((value) => !/^https?:\/\//i.test(value)))
        throw validation(key, "Links must be HTTP or HTTPS URLs");

      return normalized;
    };

    const links = packageList("links");
    const questions = packageList("openQuestions");
    const owner = ctx.world().users.get(receiver.userId);

    if (owner === undefined) throw notFound();
    const entry = historyEntry(thread, "handoff", VIEWER_ID);
    entry.toOwner = { userId: owner.id, name: owner.name };
    entry.handoff = {
      summary: summary.length > 200 ? `${summary.slice(0, 197)}...` : summary,
      linkCount: links.length,
      questionCount: questions.length,
    };
    thread.work = { ...thread.work, owner, ownerActive: true };
    thread.workDetail = {
      ...(thread.workDetail ?? emptyWorkDetail),
      history: [entry, ...(thread.workDetail?.history ?? [])],
    };
    thread.updatedAt = entry.createdAt;
    publishUpdate(thread);

    return threads.detail(thread);
  };

  const workList = (query: URLSearchParams): WorkList => {
    const viewer = ctx.world().users.get(VIEWER_ID);

    if (viewer?.status !== "active" || viewer.role === "bot") throw notFound();
    const raw = query.get("state");

    const state =
      raw === "done" || raw === "all" || raw === "agents" || raw === "boards" ? raw : "open";

    const rows = [...ctx.world().threads.values()]
      .filter((thread) => {
        const room = ctx.world().rooms.get(thread.roomId);

        if (room === undefined || !room.memberIds.includes(VIEWER_ID) || thread.work == null)
          return false;

        if (state === "open") return thread.work.status !== "done";

        if (state === "done") return thread.work.status === "done";

        if (state === "agents")
          return thread.work.owner?.role === "bot" || thread.work.owner?.agent != null;

        return state !== "boards" || room.room.kind === "board";
      })
      .sort(
        (a, b) =>
          Date.parse(b.updatedAt ?? b.lastActivityAt) -
            Date.parse(a.updatedAt ?? a.lastActivityAt) || b.id - a.id,
      );

    return {
      threads: rows.map((thread) => ({
        thread: threadDto(thread, ctx.now()),
        roomName: ctx.world().rooms.get(thread.roomId)?.room.name ?? "",
        board: !!thread.isBoard,
        updatedAt: thread.updatedAt ?? thread.lastActivityAt,
      })),
      users: ctx.usersFor(rows.map((thread) => thread.creatorId)),
    };
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
      route("PATCH", /^\/threads\/(\d+)\/work$/, (request) =>
        ok(updateWork(firstId(request), request.body)),
      ),
      route("POST", /^\/threads\/(\d+)\/work\/handoff$/, (request) =>
        ok(handoff(firstId(request), request.body), 201),
      ),
      route("GET", /^\/work$/, (request) => ok(workList(request.query))),
    ],
    control(action: string, body: Json | undefined) {
      const actorId = intField(body, "userId") ?? 2;

      if (action === "board-update")
        return ok(updateWork(intField(body, "threadId") ?? 9001, body, actorId, true));

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
