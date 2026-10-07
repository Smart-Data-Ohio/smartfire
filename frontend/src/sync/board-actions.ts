import { Effect } from "effect";
import * as api from "../api/board-endpoints.ts";
import type { CreateWorkHandoff } from "../gen/CreateWorkHandoff.ts";
import type { UpdateWork } from "../gen/UpdateWork.ts";
import type { WorkStatus } from "../gen/WorkStatus.ts";
import { uuid7 } from "../lib/uuid7.ts";
import type { BoardQuery } from "../store/boards.ts";
import { mutations, store } from "../store/store.ts";

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
  /** Preserve this when retrying a request whose response was lost. */
  readonly clientMessageId?: string;
}

export const createPost = Effect.fn("boards.createPost")(function* (
  roomId: number,
  input: BoardPostInput,
) {
  const detail = yield* api.createBoardPost(roomId, {
    name: input.name,
    status: input.status,
    ownerId: input.ownerId,
    tags: [...input.tags],
    message:
      input.brief.trim() === ""
        ? null
        : {
            clientMessageId: input.clientMessageId ?? uuid7(Date.now()),
            markdownSource: input.brief,
            replyToMessageId: null,
            replyNotifyAuthor: null,
            attachmentSignedId: null,
          },
  });

  mutations.loadThreadDetail(detail);
  mutations.addBoardPost(detail.thread);

  return detail;
});

export const update = Effect.fn("work.update")(function* (threadId: number, body: UpdateWork) {
  mutations.loadThreadDetail(yield* api.updateWork(threadId, body));
});

export const handoff = Effect.fn("work.handoff")(function* (
  threadId: number,
  body: CreateWorkHandoff,
) {
  mutations.loadThreadDetail(yield* api.handoffWork(threadId, body));
});
