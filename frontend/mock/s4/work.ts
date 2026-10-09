/**
 * Work tracking: the work list, the work fields of a thread (status, owner, result) and the
 * human handoff to an agent, with the server's permission checks, validation messages and the
 * `thread.updated` each change publishes (`work_threads#index`, `#create_handoff`,
 * `channel_threads#update`).
 */
import type { ThreadDetail } from "../../src/gen/ThreadDetail.ts";
import type { WorkFilter } from "../../src/gen/WorkFilter.ts";
import type { WorkHistoryEntry } from "../../src/gen/WorkHistoryEntry.ts";
import type { WorkList } from "../../src/gen/WorkList.ts";
import type { WorkListRow } from "../../src/gen/WorkListRow.ts";
import type { WorkStatus } from "../../src/gen/WorkStatus.ts";
import { forbidden, HttpError, type MockResponse, notFound, ok } from "../http.ts";
import {
  field,
  intField,
  isRecord,
  isString,
  type Json,
  stringArrayField,
  stringField,
} from "../json.ts";
import { renderMarkdown } from "../markdown.ts";
import { firstId, type Route, route, type S2Context } from "../s2/context.ts";
import { iso, type ThreadRecord, threadDto } from "../s2/model.ts";
import type { Threads } from "../s2/threads.ts";
import { autoAssignedBoardOwner } from "../s6/automations.ts";
import { tagsOf } from "../s6/work.ts";
import { rowTimestamp, touchedRow, VIEWER_ID } from "../seed.ts";
import { invalid, invalidBody, sentence } from "./http.ts";
import { workStateOf } from "./seed.ts";
import {
  emptyWork,
  HANDOFF_COLLECTION_LIMIT,
  HANDOFF_ENTRY_LIMIT,
  HANDOFF_SUMMARY_LIMIT,
  HISTORY_SUMMARY_LIMIT,
  isActiveHumanMember,
  isAgentOwner,
  isPostingAgentMember,
  isTracked,
  normalizeList,
  ownerActive,
  ownerSnapshot,
  RESULT_LIMIT,
  receiverError,
  setOwner,
  truncate,
  WORK_STATUSES,
  type WorkRecord,
} from "./work-model.ts";

const FILTERS: readonly WorkFilter[] = ["open", "done", "all", "agents", "boards"];

/** What a work change asks for: each key is `undefined` when the request left it out. */
interface WorkChange {
  readonly status: WorkStatus | null | undefined;
  readonly ownerId: number | null | undefined;
  readonly resultMarkdown: string | null | undefined;
  readonly tags: string[] | undefined;
}

/** The work module. */
export interface Work {
  readonly routes: readonly Route[];
  updateAs(threadId: number, body: Json | undefined, actorId: number): ThreadDetail;
  /**
   * Someone else moves a thread's work (`status: null` stops tracking it), skipping the
   * viewer's permissions: the mock control behind live-update tests.
   */
  setStatusAs(threadId: number, status: WorkStatus | null, actorId: number): ThreadDetail;
}

/** Creates the work module on top of the threads module. */
export function createWork(ctx: S2Context, threads: Threads): Work {
  const scope = (threadId: number): ThreadRecord => {
    const viewer = ctx.world().users.get(VIEWER_ID);

    if (viewer?.role === "bot" || viewer?.status !== "active") throw notFound();

    try {
      return threads.threadOr404(threadId);
    } catch (error) {
      if (error instanceof HttpError && error.status === 404) throw notFound();

      throw error;
    }
  };

  /** The thread's `updated_at` for the list: the later of its last work change and reply. */
  const updatedAt = (thread: ThreadRecord): string => {
    const work = thread.work?.updatedAt ?? thread.lastActivityAt;

    return Date.parse(work) >= Date.parse(thread.lastActivityAt) ? work : thread.lastActivityAt;
  };

  const keep = (filter: WorkFilter, thread: ThreadRecord, board: boolean): boolean => {
    const status = thread.work?.status ?? null;

    switch (filter) {
      case "open":
        return status !== "done";
      case "done":
        return status === "done";
      case "all":
        return true;
      case "agents":
        return isAgentOwner(ctx.world(), thread.work?.ownerId ?? null);
      case "boards":
        return board;
    }
  };

  const list = (query: URLSearchParams): WorkList => {
    const world = ctx.world();

    if (world.users.get(VIEWER_ID)?.role === "bot") return { threads: [], users: [] };

    const raw = query.get("state");
    const filter = FILTERS.find((candidate) => candidate === raw) ?? "open";
    const rows: WorkListRow[] = [];
    const userIds: number[] = [];

    const add = (thread: ThreadRecord, roomName: string, board: boolean) => {
      if (!isTracked(thread) || !keep(filter, thread, board)) return;

      rows.push({
        thread: threadDto(thread, ctx.now()),
        roomName,
        board,
        updatedAt: updatedAt(thread),
      });
      userIds.push(thread.creatorId);
    };

    for (const thread of world.threads.values()) {
      const record = world.rooms.get(thread.roomId);

      if (record === undefined || !record.memberIds.includes(VIEWER_ID)) continue;

      add(thread, ctx.displayName(record), record.room.kind === "board");
    }

    rows.sort(
      (a, b) => Date.parse(b.updatedAt) - Date.parse(a.updatedAt) || b.thread.id - a.thread.id,
    );

    return { threads: rows, users: ctx.usersFor(userIds) };
  };

  const record = (
    thread: ThreadRecord,
    entry: Omit<WorkHistoryEntry, "id" | "createdAt">,
    at: string,
  ) => {
    const work = thread.work;

    if (work === undefined) return;

    work.history.unshift({ id: workStateOf(ctx.world()).nextHistoryId++, createdAt: at, ...entry });
  };

  /**
   * Applies a change that already passed the permission checks: validates it, writes it,
   * records the history and publishes `thread.updated`. Answers whether anything changed.
   */
  const apply = (thread: ThreadRecord, change: WorkChange, actorId: number): boolean => {
    const world = ctx.world();
    const work: WorkRecord = thread.work ?? emptyWork(rowTimestamp(ctx.now()));
    const status = change.status === undefined ? work.status : change.status;
    const requestedOwnerId = change.ownerId === undefined ? work.ownerId : change.ownerId;
    const addedTags = (change.tags ?? []).filter((tag) => !work.tags.includes(tag));

    const ownerId =
      requestedOwnerId ??
      (status === null
        ? null
        : (autoAssignedBoardOwner(world, thread.roomId, addedTags)?.id ?? null));

    const ownerChanged = ownerId !== work.ownerId;
    const errors: [string, string][] = [];

    if (ownerId !== null && status === null) {
      errors.push(["work_owner", "requires work tracking"]);
    } else if (ownerChanged && ownerId !== null) {
      if (world.users.get(ownerId)?.role === "bot") {
        if (!isPostingAgentMember(world, thread.roomId, ownerId)) {
          errors.push([
            "work_owner",
            "must be an active agent member of the parent room with permission to post",
          ]);
        }
      } else if (!isActiveHumanMember(world, thread.roomId, ownerId)) {
        errors.push(["work_owner", "must be an active human member of the parent room"]);
      }
    }

    // A blank result clears it, as `null` does.
    const result =
      change.resultMarkdown === undefined
        ? undefined
        : change.resultMarkdown === null || change.resultMarkdown.trim() === ""
          ? null
          : change.resultMarkdown;

    if (result !== undefined && result !== null && [...result].length > RESULT_LIMIT) {
      errors.push(["result_markdown", `is too long (maximum is ${RESULT_LIMIT} characters)`]);
    }

    if (errors.length > 0) throw invalid(errors, { work_owner: "ownerId" });

    const at = iso(ctx.now());
    const statusChanged = status !== work.status;
    const resultChanged = result !== undefined && result !== work.resultMarkdown;

    const tagsChanged =
      change.tags !== undefined && JSON.stringify(change.tags) !== JSON.stringify(work.tags);

    if (!statusChanged && !ownerChanged && !resultChanged && !tagsChanged) return false;

    thread.work = work;

    if (statusChanged || ownerChanged) {
      record(
        thread,
        {
          kind: statusChanged ? "update" : "assignment",
          actorId,
          fromStatus: work.status,
          toStatus: status,
          fromOwner: ownerSnapshot(world, work.ownerId),
          toOwner: ownerSnapshot(world, ownerId),
          note: null,
          handoff: null,
        },
        at,
      );
      work.status = status;
      setOwner(world, work, ownerId);
      work.ownerActive = ownerActive(world, thread.roomId, ownerId);
    }

    if (resultChanged) {
      work.resultMarkdown = result;
      work.resultHtml = result === null ? null : renderMarkdown(result, ctx.mentionables());
      work.resultUpdatedById = actorId;
      work.resultUpdatedAt = at;

      const owner = ownerSnapshot(world, work.ownerId);

      record(
        thread,
        {
          kind: "result",
          actorId,
          fromStatus: work.status,
          toStatus: work.status,
          fromOwner: owner,
          toOwner: owner,
          note: null,
          handoff: null,
        },
        at,
      );
    }

    if (change.tags !== undefined) work.tags = change.tags;

    if (statusChanged || ownerChanged || resultChanged)
      work.updatedAt = touchedRow(ctx.now(), work.updatedAt);
    ctx.publish(threads.updated(thread));

    return true;
  };

  const parseChange = (body: Json | undefined): WorkChange => {
    if (!isRecord(body)) throw invalidBody("expected an object");

    // `undefined` when the body leaves a key out, `null` when it sends `null`.
    const rawStatus = field(body, "status");
    const rawOwner = field(body, "ownerId");
    const rawResult = field(body, "resultMarkdown");
    const status = WORK_STATUSES.find((candidate) => candidate === rawStatus);

    if (rawStatus !== undefined && rawStatus !== null && status === undefined) {
      throw invalidBody("status must be a work status or null");
    }

    const ownerId = intField(body, "ownerId");

    if (rawOwner !== undefined && rawOwner !== null && ownerId === null) {
      throw invalidBody("ownerId must be an integer or null");
    }

    if (rawResult !== undefined && rawResult !== null && !isString(rawResult)) {
      throw invalidBody("resultMarkdown must be a string or null");
    }

    return {
      status: rawStatus === undefined ? undefined : (status ?? null),
      ownerId: rawOwner === undefined ? undefined : ownerId,
      resultMarkdown: rawResult === undefined ? undefined : stringField(body, "resultMarkdown"),
      tags: field(body, "tags") === undefined ? undefined : tagsOf(body),
    };
  };

  const update = (threadId: number, body: Json | undefined): ThreadDetail => {
    const thread = scope(threadId);
    const change = parseChange(body);
    const allowed = threads.detail(thread).permissions;
    const manager = allowed.canManageWork || allowed.canConvertWork;
    const assignment = allowed.canAssignWork || allowed.canConvertWork;

    if (
      (change.resultMarkdown !== undefined && !manager) ||
      (change.tags !== undefined && !(thread.isBoard ? manager : assignment)) ||
      ((change.status !== undefined || change.ownerId !== undefined) && !manager) ||
      (change.ownerId !== undefined && !assignment) ||
      (change.status !== undefined && (change.status !== null) !== isTracked(thread) && !assignment)
    ) {
      throw forbidden("You can't make that change to this thread");
    }

    apply(thread, change, VIEWER_ID);

    return threads.detail(thread);
  };

  const handoff = (threadId: number, body: Json | undefined): MockResponse => {
    const world = ctx.world();
    const thread = scope(threadId);
    const work = thread.work;

    if (work === undefined || work.status === null) {
      throw sentence("base", "This thread isn't tracked as work");
    }

    if (!threads.detail(thread).permissions.canManageWork) {
      throw forbidden("You cannot manage work in this thread");
    }

    const receiverAgentId = intField(body, "receiverAgentId");
    const summary = stringField(body, "summary");
    const rawLinks = stringArrayField(body, "links");
    const rawQuestions = stringArrayField(body, "openQuestions");

    if (
      receiverAgentId === null ||
      summary === null ||
      rawLinks === null ||
      rawQuestions === null
    ) {
      throw invalidBody(
        "receiverAgentId, summary, links and openQuestions must match the contract",
      );
    }

    const error = receiverError(world, thread.roomId, work.ownerId, receiverAgentId);

    if (error !== null) throw sentence("receiverAgentId", error);

    const receiver = [...world.users.values()].find(
      (user) => user.agent?.agentId === receiverAgentId,
    );

    if (receiver === undefined) throw notFound();

    const links = normalizeList(rawLinks);
    const questions = normalizeList(rawQuestions);
    const errors: [string, string][] = [];

    if (summary.trim() === "") errors.push(["summary", "can't be blank"]);

    if ([...summary].length > HANDOFF_SUMMARY_LIMIT) {
      errors.push(["summary", `is too long (maximum is ${HANDOFF_SUMMARY_LIMIT} characters)`]);
    }

    if (links.length > HANDOFF_COLLECTION_LIMIT) {
      errors.push(["links", `are limited to ${HANDOFF_COLLECTION_LIMIT} per handoff`]);
    }

    for (const link of links) {
      if ([...link].length > HANDOFF_ENTRY_LIMIT) {
        errors.push(["links", `must be at most ${HANDOFF_ENTRY_LIMIT} characters each`]);
        break;
      }

      if (!/^https?:\/\//i.test(link)) {
        errors.push(["links", "must be http(s) URLs"]);
        break;
      }
    }

    if (questions.length > HANDOFF_COLLECTION_LIMIT) {
      errors.push(["open_questions", `are limited to ${HANDOFF_COLLECTION_LIMIT} per handoff`]);
    }

    if (questions.some((question) => [...question].length > HANDOFF_ENTRY_LIMIT)) {
      errors.push(["open_questions", `must be at most ${HANDOFF_ENTRY_LIMIT} characters each`]);
    }

    if (errors.length > 0) throw invalid(errors);

    const at = iso(ctx.now());

    record(
      thread,
      {
        kind: "handoff",
        actorId: VIEWER_ID,
        fromStatus: work.status,
        toStatus: work.status,
        fromOwner: ownerSnapshot(world, work.ownerId),
        toOwner: ownerSnapshot(world, receiver.id),
        note: null,
        handoff: {
          summary: truncate(summary, HISTORY_SUMMARY_LIMIT),
          linkCount: links.length,
          questionCount: questions.length,
        },
      },
      at,
    );
    setOwner(world, work, receiver.id);
    work.ownerActive = true;
    work.updatedAt = touchedRow(ctx.now(), work.updatedAt);
    ctx.publish(threads.updated(thread));

    return ok(threads.detail(thread), 201);
  };

  return {
    routes: [
      route("GET", /^\/work$/, (request) => ok(list(request.query))),
      route("PATCH", /^\/threads\/(\d+)\/work$/, (request) =>
        ok(update(firstId(request), request.body)),
      ),
      route("POST", /^\/threads\/(\d+)\/work\/handoff$/, (request) =>
        handoff(firstId(request), request.body),
      ),
    ],
    updateAs(threadId, body, actorId) {
      const thread = threads.threadOr404(threadId);

      apply(thread, parseChange(body), actorId);

      return threads.detail(thread);
    },
    setStatusAs(threadId, status, actorId) {
      const thread = threads.threadOr404(threadId);

      apply(
        thread,
        {
          status,
          ownerId: status === null ? null : undefined,
          resultMarkdown: undefined,
          tags: undefined,
        },
        actorId,
      );

      return threads.detail(thread);
    },
  };
}
