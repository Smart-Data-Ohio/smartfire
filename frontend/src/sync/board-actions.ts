import { Effect } from "effect";
import * as api from "../api/board-endpoints.ts";
import type { BoardTagCatalog } from "../gen/BoardTagCatalog.ts";
import type { CreateBoardTagRule } from "../gen/CreateBoardTagRule.ts";
import type { CreateMessage } from "../gen/CreateMessage.ts";
import type { CreateWorkHandoff } from "../gen/CreateWorkHandoff.ts";
import type { SaveBoardTag } from "../gen/SaveBoardTag.ts";
import type { UpdateBoardSlaTimers } from "../gen/UpdateBoardSlaTimers.ts";
import type { UpdateBoardTagPolicy } from "../gen/UpdateBoardTagPolicy.ts";
import type { UpdateWork } from "../gen/UpdateWork.ts";
import type { WorkStatus } from "../gen/WorkStatus.ts";
import type { BoardQuery } from "../store/boards.ts";
import { mutations, store } from "../store/store.ts";
import { captureWorkRead } from "../store/work.ts";
import { settledDetail } from "./settle.ts";
import * as workActions from "./work-actions.ts";

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

/**
 * Catalog answers by the order their requests started: the listing, the new-post form, catalog
 * reads and writes all carry the catalog, and an answer to an older request mustn't replace a
 * newer one (a refetch that crossed a write, say).
 */
const catalogIssued = new Map<number, number>();

const catalogShown = new Map<number, number>();

function catalogTicket(roomId: number): number {
  const ticket = (catalogIssued.get(roomId) ?? 0) + 1;

  catalogIssued.set(roomId, ticket);

  return ticket;
}

function showCatalog(ticket: number, catalog: BoardTagCatalog): void {
  if (ticket <= (catalogShown.get(catalog.roomId) ?? 0)) return;

  catalogShown.set(catalog.roomId, ticket);
  mutations.setBoardTagCatalog(catalog);
}

function catalogOf(
  roomId: number,
  from: Pick<BoardTagCatalog, "tags" | "tagsRequired" | "defaultBoardTagId">,
): BoardTagCatalog {
  return {
    roomId,
    tags: from.tags,
    tagsRequired: from.tagsRequired,
    defaultBoardTagId: from.defaultBoardTagId,
  };
}

/** Reads the catalog afresh: on opening the tag editor, or after `board.automations.changed`. */
export const tagCatalog = Effect.fn("boards.tagCatalog")(function* (roomId: number) {
  const ticket = catalogTicket(roomId);
  const catalog = yield* api.boardTagCatalog(roomId);

  showCatalog(ticket, catalog);

  return catalog;
});

const catalogWrite = <E, R>(roomId: number, write: Effect.Effect<BoardTagCatalog, E, R>) =>
  Effect.gen(function* () {
    const ticket = catalogTicket(roomId);
    const catalog = yield* write;

    showCatalog(ticket, catalog);

    return catalog;
  });

export const createTag = Effect.fn("boards.createTag")(function* (
  roomId: number,
  input: SaveBoardTag,
) {
  return yield* catalogWrite(roomId, api.createBoardTag(roomId, input));
});

export const updateTag = Effect.fn("boards.updateTag")(function* (
  roomId: number,
  tagId: number,
  input: SaveBoardTag,
) {
  return yield* catalogWrite(roomId, api.updateBoardTag(roomId, tagId, input));
});

export const deleteTag = Effect.fn("boards.deleteTag")(function* (roomId: number, tagId: number) {
  return yield* catalogWrite(roomId, api.deleteBoardTag(roomId, tagId));
});

export const reorderTags = Effect.fn("boards.reorderTags")(function* (
  roomId: number,
  tagIds: readonly number[],
) {
  return yield* catalogWrite(roomId, api.reorderBoardTags(roomId, { tagIds: [...tagIds] }));
});

export const updateTagPolicy = Effect.fn("boards.updateTagPolicy")(function* (
  roomId: number,
  input: UpdateBoardTagPolicy,
) {
  return yield* catalogWrite(roomId, api.updateBoardTagPolicy(roomId, input));
});

/** Room sessions own room:<id>; changing board filters does not acquire another holder. */
export const open = Effect.fn("boards.open")(function* (roomId: number, query: BoardQuery) {
  mutations.setBoardLoading(roomId, query);
  const board = store.getState().boards[roomId];

  if (board === undefined) return;
  const read = captureWorkRead(store.getState());
  const ticket = catalogTicket(roomId);
  yield* api.board(roomId, { ...board.query, page: 1 }).pipe(
    Effect.tap((listing) =>
      Effect.sync(() => {
        showCatalog(ticket, catalogOf(roomId, listing));
        mutations.loadBoardListing(listing, board.generation, read);
      }),
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
  const ticket = catalogTicket(roomId);
  yield* api.board(roomId, { ...board.query, page: held.page + 1 }).pipe(
    Effect.tap((listing) =>
      Effect.sync(() => {
        showCatalog(ticket, catalogOf(roomId, listing));
        mutations.loadBoardListing(listing, board.generation, read);
      }),
    ),
    Effect.catch((error) =>
      Effect.sync(() => mutations.setBoardError(roomId, board.generation, error.message)),
    ),
  );
});

export const postForm = Effect.fn("boards.postForm")(function* (roomId: number) {
  const ticket = catalogTicket(roomId);
  const form = yield* api.boardPostForm(roomId);
  mutations.mergeUsers(form.users);
  showCatalog(ticket, catalogOf(roomId, form));

  return form;
});

export interface BoardPostInput {
  readonly name: string;
  readonly status: WorkStatus;
  readonly ownerId: number | null;
  readonly tags: readonly string[];
  readonly brief: string;
  readonly attachmentSignedId?: string | null;
  readonly attachmentSignedIds?: readonly string[];
  /**
   * The submission's retry identity (a UUID): keep it across retries of one post, so a retry after
   * a lost reply answers the post already made. The brief's message id is the same.
   */
  readonly clientId: string;
}

function postMessage(input: BoardPostInput): CreateMessage | null {
  const attachmentSignedIds = input.attachmentSignedIds ?? [];

  if (
    input.brief.trim() === "" &&
    input.attachmentSignedId == null &&
    attachmentSignedIds.length === 0
  ) {
    return null;
  }

  const message: CreateMessage = {
    clientMessageId: input.clientId,
    markdownSource: input.brief,
    replyToMessageId: null,
    replyNotifyAuthor: null,
    attachmentSignedId: input.attachmentSignedId ?? null,
  };

  if (attachmentSignedIds.length > 0) {
    message.attachmentSignedIds = [...attachmentSignedIds];
  }

  return message;
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
      message: postMessage(input),
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

export const linkForm = workActions.linkForm;

export const addLink = workActions.addLink;

export const removeLink = workActions.removeLink;
