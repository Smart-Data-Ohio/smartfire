import { Effect } from "effect";
import * as api from "../api/board-endpoints.ts";
import type { CreateBoardTagRule } from "../gen/CreateBoardTagRule.ts";
import type { CreateWorkHandoff } from "../gen/CreateWorkHandoff.ts";
import type { ThreadDetail } from "../gen/ThreadDetail.ts";
import type { UpdateBoardSlaTimers } from "../gen/UpdateBoardSlaTimers.ts";
import type { UpdateWork } from "../gen/UpdateWork.ts";
import type { WorkFilter } from "../gen/WorkFilter.ts";
import type { WorkStatus } from "../gen/WorkStatus.ts";
import type { BoardQuery } from "../store/boards.ts";
import { mutations, store } from "../store/store.ts";
import { settledDetail } from "./settle.ts";
import { refreshWorkPane } from "./work-refresh.ts";

export const automations = Effect.fn("boards.automations")(function* (roomId: number) {
  const settings = yield* api.boardAutomations(roomId);
  mutations.mergeUsers(settings.users);

  return settings;
});

export const addTagRule = Effect.fn("boards.addTagRule")(function* (
  roomId: number,
  input: CreateBoardTagRule,
) {
  const settings = yield* api.createBoardTagRule(roomId, input);
  mutations.mergeUsers(settings.users);

  return settings;
});

export const removeTagRule = Effect.fn("boards.removeTagRule")(function* (
  roomId: number,
  ruleId: number,
) {
  const settings = yield* api.deleteBoardTagRule(roomId, ruleId);
  mutations.mergeUsers(settings.users);

  return settings;
});

export const saveSlaTimers = Effect.fn("boards.saveSlaTimers")(function* (
  roomId: number,
  input: UpdateBoardSlaTimers,
) {
  const settings = yield* api.updateBoardSlaTimers(roomId, input);
  mutations.mergeUsers(settings.users);

  return settings;
});

/** Room sessions own room:<id>; changing board filters does not acquire another holder. */
export const open = Effect.fn("boards.open")(function* (roomId: number, query: BoardQuery) {
  mutations.setBoardLoading(roomId, query);
  const board = store.getState().boards[roomId];

  if (board === undefined) return;
  yield* api.board(roomId, { ...board.query, page: 1 }).pipe(
    Effect.tap((listing) =>
      Effect.sync(() => mutations.loadBoardListing(listing, board.generation)),
    ),
    Effect.catch((error) =>
      Effect.sync(() => mutations.setBoardError(roomId, board.generation, error.message)),
    ),
  );
});

export const loadMore = Effect.fn("boards.loadMore")(function* (roomId: number) {
  const held = store.getState().boards[roomId];

  if (
    held === undefined ||
    held.status !== "ready" ||
    !held.hasMore ||
    held.loadingMore ||
    held.page >= 20
  )
    return;
  mutations.setBoardLoading(roomId, held.query, true);
  const board = store.getState().boards[roomId];

  if (board === undefined) return;
  yield* api.board(roomId, { ...board.query, page: held.page + 1 }).pipe(
    Effect.tap((listing) =>
      Effect.sync(() => mutations.loadBoardListing(listing, board.generation)),
    ),
    Effect.catch((error) =>
      Effect.sync(() => mutations.setBoardError(roomId, board.generation, error.message)),
    ),
  );
});

export const postForm = Effect.fn("boards.postForm")(function* (roomId: number) {
  const form = yield* api.boardPostForm(roomId);
  mutations.mergeUsers(form.users);

  return form;
});

export interface BoardPostInput {
  readonly name: string;
  readonly status: WorkStatus;
  readonly ownerId: number | null;
  readonly tags: readonly string[];
  readonly brief: string;
  /**
   * The submission's retry identity (a UUID): keep it across retries of one post, so a retry after
   * a lost reply answers the post already made. The brief's message id is the same.
   */
  readonly clientId: string;
}

export const createPost = Effect.fn("boards.createPost")(function* (
  roomId: number,
  input: BoardPostInput,
) {
  // A post removed while its creation was in flight stays removed.
  const reply = yield* settledDetail(
    api.createBoardPost(roomId, {
      name: input.name,
      status: input.status,
      ownerId: input.ownerId,
      tags: [...input.tags],
      clientPostId: input.clientId,
      message:
        input.brief.trim() === ""
          ? null
          : {
              clientMessageId: input.clientId,
              markdownSource: input.brief,
              replyToMessageId: null,
              replyNotifyAuthor: null,
              attachmentSignedId: null,
            },
    }),
    (detail, since) => {
      mutations.loadThreadDetail(detail, since);

      // Not when the reply was dropped: the post was removed while it was being created.
      if (store.getState().threads[detail.thread.id] !== undefined) {
        mutations.addBoardPost(detail.thread);
      }
    },
  );

  return reply.answer;
});

/**
 * A save's reply is the post as the server had it when it answered. If anything newer reached the
 * store while it was in flight (another member's change, a refreshed detail, our own broadcast),
 * the reply may be older than what we hold, so it isn't installed: the pane refetches instead,
 * and `refreshWorkPane` settles any change that lands during that GET too.
 */
const installSaved = <E, R>(threadId: number, write: Effect.Effect<ThreadDetail, E, R>) =>
  Effect.gen(function* () {
    const sent = store.getState().threads[threadId];
    let stale = false;

    yield* settledDetail(write, (detail, since) => {
      if (store.getState().threads[threadId] === sent) {
        mutations.loadThreadDetail(detail, since);
      } else {
        stale = true;
      }
    });

    if (stale) {
      yield* refreshWorkPane(threadId);
    }
  });

export const update = Effect.fn("work.update")(function* (threadId: number, body: UpdateWork) {
  yield* installSaved(threadId, api.updateWork(threadId, body));
});

export const handoff = Effect.fn("work.handoff")(function* (
  threadId: number,
  body: CreateWorkHandoff,
) {
  yield* installSaved(threadId, api.handoffWork(threadId, body));
});

/** The work list (`GET /work?state=`); its creators join the store's people. */
export const list = Effect.fn("work.list")(function* (state: WorkFilter) {
  const work = yield* api.workList(state);
  mutations.mergeUsers(work.users);

  return work;
});
