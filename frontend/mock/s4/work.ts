/**
 * Work tracking: the work list, the work fields of a thread (status, owner, result) and the
 * human handoff to an agent, with the server's permission checks, validation messages and the
 * `thread.updated` each change publishes (`work_threads#index`, `#create_handoff`,
 * `channel_threads#update`).
 */
import type { ApiError } from "../../src/gen/ApiError.ts";
import type { ThreadDetail } from "../../src/gen/ThreadDetail.ts";
import type { WorkFilter } from "../../src/gen/WorkFilter.ts";
import type { WorkHistoryEntry } from "../../src/gen/WorkHistoryEntry.ts";
import type { WorkList } from "../../src/gen/WorkList.ts";
import type { WorkListRow } from "../../src/gen/WorkListRow.ts";
import type { WorkStatus } from "../../src/gen/WorkStatus.ts";
import { forbidden, HttpError, type MockResponse, ok, validation } from "../http.ts";
import { field, intField, isString, type Json, stringArrayField, stringField } from "../json.ts";
import { renderMarkdown } from "../markdown.ts";
import { firstId, type Route, route, type S2Context } from "../s2/context.ts";
import { iso, type ThreadRecord, threadDto, touched } from "../s2/model.ts";
import type { Threads } from "../s2/threads.ts";
import { VIEWER_ID } from "../seed.ts";
import { S4_BOARD, workStateOf } from "./seed.ts";
import {
  emptyWork,
  HANDOFF_COLLECTION_LIMIT,
  HANDOFF_ENTRY_LIMIT,
  HANDOFF_SUMMARY_LIMIT,
  HISTORY_SUMMARY_LIMIT,
  handoffReceivers,
  isActiveHumanMember,
  isPostingAgentMember,
  isTracked,
  normalizeList,
  ownerActive,
  ownerSnapshot,
  RESULT_LIMIT,
  setOwner,
  truncate,
  WORK_STATUSES,
  type WorkRecord,
} from "./work-model.ts";

const FILTERS: readonly WorkFilter[] = ["open", "done", "all", "agents", "boards"];

/** The wire tag, as http.ts spells it (the mock has no Effect tagged constructors). */
const VALIDATION: ApiError["_tag"] = "Validation";

/** A 422 with every message the record collected, as Rails' `errors.full_messages` reads. */
const invalid = (errors: readonly (readonly [string, string])[]): HttpError => {
  const fields: Record<string, string[]> = {};

  for (const [key, message] of errors) fields[key] = [...(fields[key] ?? []), message];

  return new HttpError(422, {
    _tag: VALIDATION,
    message: `Validation failed: ${errors.map(([, message]) => message).join(", ")}`,
    fields,
  });
};

/** What a work change asks for: each key is `undefined` when the request left it out. */
interface WorkChange {
  readonly status: WorkStatus | null | undefined;
  readonly ownerId: number | null | undefined;
  readonly resultMarkdown: string | null | undefined;
}

/** The work module. */
export interface Work {
  readonly routes: readonly Route[];
  /**
   * Someone else moves a thread's work (`status: null` stops tracking it), skipping the
   * viewer's permissions: the mock control behind live-update tests.
   */
  setStatusAs(threadId: number, status: WorkStatus | null, actorId: number): ThreadDetail;
}

/** Creates the work module on top of the threads module. */
export function createWork(ctx: S2Context, threads: Threads): Work {
  const isAgentUser = (userId: number | null): boolean =>
    userId !== null && ctx.world().users.get(userId)?.role === "bot";

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
        return isAgentUser(thread.work?.ownerId ?? null);
      case "boards":
        return board;
    }
  };

  const list = (query: URLSearchParams): WorkList => {
    const world = ctx.world();
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

      add(thread, ctx.displayName(record), false);
    }

    for (const post of workStateOf(world).boardPosts) add(post, S4_BOARD.name, true);

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
    const work: WorkRecord = thread.work ?? emptyWork(iso(ctx.now()));
    const status = change.status === undefined ? work.status : change.status;
    const ownerId = change.ownerId === undefined ? work.ownerId : change.ownerId;
    const ownerChanged = ownerId !== work.ownerId;
    const errors: [string, string][] = [];

    if (ownerId !== null && status === null) {
      errors.push(["ownerId", "Work owner requires work tracking"]);
    } else if (ownerChanged && ownerId !== null) {
      if (isAgentUser(ownerId)) {
        if (!isPostingAgentMember(world, thread.roomId, ownerId)) {
          errors.push([
            "ownerId",
            "Work owner must be an active agent member of the parent room with permission to post",
          ]);
        }
      } else if (!isActiveHumanMember(world, thread.roomId, ownerId)) {
        errors.push(["ownerId", "Work owner must be an active human member of the parent room"]);
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
      errors.push([
        "resultMarkdown",
        `Result markdown is too long (maximum is ${RESULT_LIMIT} characters)`,
      ]);
    }

    if (errors.length > 0) throw invalid(errors);

    const at = iso(ctx.now());
    const statusChanged = status !== work.status;
    const resultChanged = result !== undefined && result !== work.resultMarkdown;

    if (!statusChanged && !ownerChanged && !resultChanged) return false;

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

    work.updatedAt = touched(ctx.now(), work.updatedAt);
    ctx.publish(threads.updated(thread));

    return true;
  };

  const parseChange = (body: Json | undefined): WorkChange => {
    // `undefined` when the body leaves a key out, `null` when it sends `null`.
    const rawStatus = field(body, "status");
    const rawOwner = field(body, "ownerId");
    const rawResult = field(body, "resultMarkdown");
    const status = WORK_STATUSES.find((candidate) => candidate === rawStatus);

    if (rawStatus !== undefined && rawStatus !== null && status === undefined) {
      throw validation("status", "Work status is invalid");
    }

    const ownerId = intField(body, "ownerId");

    if (rawOwner !== undefined && rawOwner !== null && ownerId === null) {
      throw validation("ownerId", "Work owner must be an active human member of the parent room");
    }

    if (rawResult !== undefined && rawResult !== null && !isString(rawResult)) {
      throw validation("resultMarkdown", "Result markdown is invalid");
    }

    return {
      status: rawStatus === undefined ? undefined : (status ?? null),
      ownerId: rawOwner === undefined ? undefined : ownerId,
      resultMarkdown: rawResult === undefined ? undefined : stringField(body, "resultMarkdown"),
    };
  };

  const update = (threadId: number, body: Json | undefined): ThreadDetail => {
    const thread = threads.threadOr404(threadId);
    const change = parseChange(body);
    const allowed = threads.detail(thread).permissions;
    const tracked = isTracked(thread);
    const converting = !tracked && change.status !== undefined && change.status !== null;

    const refuse = () => {
      throw forbidden("You can't change this work");
    };

    if (change.status !== undefined) {
      if (converting) {
        if (!allowed.canConvertWork) refuse();
      } else if (change.status === null) {
        if (!allowed.canAssignWork) refuse();
      } else if (!allowed.canUpdateWorkStatus) {
        refuse();
      }
    }

    if (
      change.ownerId !== undefined &&
      !(allowed.canAssignWork || (converting && allowed.canConvertWork))
    ) {
      refuse();
    }

    if (
      change.resultMarkdown !== undefined &&
      !(allowed.canManageWork || (converting && allowed.canConvertWork))
    ) {
      refuse();
    }

    apply(thread, change, VIEWER_ID);

    return threads.detail(thread);
  };

  const handoff = (threadId: number, body: Json | undefined): MockResponse => {
    const world = ctx.world();
    const thread = threads.threadOr404(threadId);
    const work = thread.work;

    if (work === undefined || work.status === null) {
      throw validation("base", "This thread isn't tracked as work");
    }

    if (!threads.detail(thread).permissions.canManageWork) {
      throw forbidden("You can't hand off this work");
    }

    const receiverAgentId = intField(body, "receiverAgentId");
    const receivers = handoffReceivers(world, thread.roomId, work.ownerId);
    const receiver = receivers.find((candidate) => candidate.agentId === receiverAgentId);

    if (receiver === undefined) {
      const owner = work.ownerId === null ? undefined : world.users.get(work.ownerId);

      throw validation(
        "receiverAgentId",
        owner?.agent?.agentId === receiverAgentId && receiverAgentId !== null
          ? "Receiver is already the owner of this work"
          : "Receiver must be an active agent member of this room with permission to post",
      );
    }

    const summary = stringField(body, "summary") ?? "";
    const links = normalizeList(stringArrayField(body, "links") ?? []);
    const questions = normalizeList(stringArrayField(body, "openQuestions") ?? []);
    const errors: [string, string][] = [];

    if (summary.trim() === "") errors.push(["summary", "Summary can't be blank"]);

    if ([...summary].length > HANDOFF_SUMMARY_LIMIT) {
      errors.push([
        "summary",
        `Summary is too long (maximum is ${HANDOFF_SUMMARY_LIMIT} characters)`,
      ]);
    }

    if (links.length > HANDOFF_COLLECTION_LIMIT) {
      errors.push(["links", `Links are limited to ${HANDOFF_COLLECTION_LIMIT} per handoff`]);
    }

    for (const link of links) {
      if ([...link].length > HANDOFF_ENTRY_LIMIT) {
        errors.push(["links", `Links must be at most ${HANDOFF_ENTRY_LIMIT} characters each`]);
        break;
      }

      if (!/^https?:\/\//i.test(link)) {
        errors.push(["links", "Links must be http(s) URLs"]);
        break;
      }
    }

    if (questions.length > HANDOFF_COLLECTION_LIMIT) {
      errors.push([
        "openQuestions",
        `Open questions are limited to ${HANDOFF_COLLECTION_LIMIT} per handoff`,
      ]);
    }

    if (questions.some((question) => [...question].length > HANDOFF_ENTRY_LIMIT)) {
      errors.push([
        "openQuestions",
        `Open questions must be at most ${HANDOFF_ENTRY_LIMIT} characters each`,
      ]);
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
        toOwner: ownerSnapshot(world, receiver.userId),
        note: null,
        handoff: {
          summary: truncate(summary, HISTORY_SUMMARY_LIMIT),
          linkCount: links.length,
          questionCount: questions.length,
        },
      },
      at,
    );
    setOwner(world, work, receiver.userId);
    work.ownerActive = true;
    work.updatedAt = touched(ctx.now(), work.updatedAt);
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
    setStatusAs(threadId, status, actorId) {
      const thread = threads.threadOr404(threadId);

      apply(
        thread,
        { status, ownerId: status === null ? null : undefined, resultMarkdown: undefined },
        actorId,
      );

      return threads.detail(thread);
    },
  };
}
