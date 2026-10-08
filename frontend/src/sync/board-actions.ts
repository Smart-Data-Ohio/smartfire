import { Effect } from "effect";
import * as api from "../api/board-endpoints.ts";
import type { CreateWorkHandoff } from "../gen/CreateWorkHandoff.ts";
import type { UpdateWork } from "../gen/UpdateWork.ts";
import type { WorkStatus } from "../gen/WorkStatus.ts";
import type { BoardQuery } from "../store/boards.ts";
import { mutations, store } from "../store/store.ts";
import { captureWorkRead } from "../store/work.ts";
import { settledDetail } from "./settle.ts";
import * as workActions from "./work-actions.ts";

/** Room sessions own room:<id>; changing board filters does not acquire another holder. */
export const open = Effect.fn("boards.open")(function* (roomId: number, query: BoardQuery) {
  mutations.setBoardLoading(roomId, query);
  const board = store.getState().boards[roomId];

  if (board === undefined) return;
  const read = captureWorkRead(store.getState());
  yield* api.board(roomId, { ...board.query, page: 1 }).pipe(
    Effect.tap((listing) =>
      Effect.sync(() => mutations.loadBoardListing(listing, board.generation, read)),
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
  const read = captureWorkRead(store.getState());
  yield* api.board(roomId, { ...board.query, page: held.page + 1 }).pipe(
    Effect.tap((listing) =>
      Effect.sync(() => mutations.loadBoardListing(listing, board.generation, read)),
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
    (detail, since, read) => {
      mutations.loadThreadDetail(detail, since, read);

      // Not when the reply was dropped: the post was removed while it was being created.
      if (store.getState().threads[detail.thread.id] !== undefined) {
        mutations.addBoardPost(detail.thread);
      }
    },
  );

  return reply.answer;
});

export const update = Effect.fn("boards.updateWork")(function* (
  threadId: number,
  body: UpdateWork,
) {
  yield* workActions.update(threadId, body);
});

export const handoff = Effect.fn("boards.handoffWork")(function* (
  threadId: number,
  body: CreateWorkHandoff,
) {
  yield* workActions.handOff(threadId, body);
});
