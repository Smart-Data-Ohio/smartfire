import { Effect } from "effect";
import * as api from "../api/board-endpoints.ts";
import * as links from "../api/work-link-endpoints.ts";
import type { CreateBoardTagRule } from "../gen/CreateBoardTagRule.ts";
import type { CreateWorkHandoff } from "../gen/CreateWorkHandoff.ts";
import type { CreateWorkLink } from "../gen/CreateWorkLink.ts";
import type { Thread } from "../gen/Thread.ts";
import type { ThreadDetail } from "../gen/ThreadDetail.ts";
import type { UpdateBoardSlaTimers } from "../gen/UpdateBoardSlaTimers.ts";
import type { UpdateWork } from "../gen/UpdateWork.ts";
import type { WorkStatus } from "../gen/WorkStatus.ts";
import type { BoardQuery } from "../store/boards.ts";
import { mutations, store } from "../store/store.ts";
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
  const since = store.getState().removalCount;

  const detail = yield* api.createBoardPost(roomId, {
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
  });

  mutations.loadThreadDetail(detail, since);
  mutations.addBoardPost(detail.thread);

  return detail;
});

/**
 * A save's reply is the post as the server had it when it answered. If anything newer reached the
 * store while it was in flight (another member's change, a refreshed detail, our own broadcast),
 * the reply may be older than what we hold, so it isn't installed: the pane refetches instead,
 * and `refreshWorkPane` settles any change that lands during that GET too.
 */
const installSaved = Effect.fnUntraced(function* (
  threadId: number,
  sent: Thread | undefined,
  since: number,
  detail: ThreadDetail,
) {
  if (store.getState().threads[threadId] === sent) {
    mutations.loadThreadDetail(detail, since);

    return;
  }

  yield* refreshWorkPane(threadId);
});

export const update = Effect.fn("work.update")(function* (threadId: number, body: UpdateWork) {
  const sent = store.getState().threads[threadId];
  const since = store.getState().removalCount;

  yield* installSaved(threadId, sent, since, yield* api.updateWork(threadId, body));
});

export const handoff = Effect.fn("work.handoff")(function* (
  threadId: number,
  body: CreateWorkHandoff,
) {
  const sent = store.getState().threads[threadId];
  const since = store.getState().removalCount;

  yield* installSaved(threadId, sent, since, yield* api.handoffWork(threadId, body));
});

export const linkForm = links.workLinkForm;

export const addLink = Effect.fn("work.addLink")(function* (
  threadId: number,
  input: CreateWorkLink,
) {
  const sent = store.getState().threads[threadId];
  const since = store.getState().removalCount;
  const detail = yield* links.createWorkLink(threadId, input);
  yield* installSaved(threadId, sent, since, detail);

  return detail;
});

export const removeLink = Effect.fn("work.removeLink")(function* (
  threadId: number,
  linkId: number,
) {
  const sent = store.getState().threads[threadId];
  const since = store.getState().removalCount;
  const detail = yield* links.deleteWorkLink(threadId, linkId);
  yield* installSaved(threadId, sent, since, detail);

  return detail;
});
