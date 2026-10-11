/**
 * The shared workspace layout as Effect programs (C3, C14): the categories and channel order an
 * administrator sets for everyone. Each change shows at once as the layout it will leave
 * (store/workspace-layout.ts) and that goes when it settles: by then the reply has landed or,
 * after a refusal, the layout is as it was. The server also publishes every change to each
 * member as `workspace.layout.updated`, this tab included, so other people's sidebars follow.
 *
 * The calls run one at a time, in the order they were made, so a room dropped into a category
 * created a moment before reaches the server after the category does.
 */
import { Effect, Semaphore } from "effect";
import * as api from "../api/organize-endpoints.ts";
import type { WorkspaceCategory } from "../gen/WorkspaceCategory.ts";
import type { WorkspaceLayout } from "../gen/WorkspaceLayout.ts";
import { mutations, store } from "../store/store.ts";
import {
  isPlacedAt,
  moveRequest,
  sharedCategories,
  sharedLayout,
  withCategoriesOrdered,
  withCategory,
  withoutCategory,
  withRoomMoved,
} from "../store/workspace-layout.ts";
import { refetchSidebar, SERVER_TIMEOUT, stalled } from "./organize-actions.ts";
import { withRowTicket } from "./row-ticket.ts";

/** One server change at a time, in order. */
const lock = Semaphore.makeUnsafe(1);

/** Temporary ids of shared categories being created, to the ids the server gave them. */
const createdIds = new Map<number, number>();

let nextTemporaryId = -1;

/** The layout as the administrator sees it now, pending changes included. */
const view = () => sharedLayout(store.getState().sidebar);

/** The layout the server has confirmed: what the next call is checked against. */
const confirmed = () => store.getState().sidebar.workspaceLayout;

/** The id the server knows a category by (a new one's, once it has been created). */
const serverCategoryId = (categoryId: number) => createdIds.get(categoryId) ?? categoryId;

/**
 * Shows `next` from now until `change` settles; the change waits its turn for the server. A
 * change still unanswered after `SERVER_TIMEOUT` is given up so the next one can run, and the
 * sidebar is fetched again in the background in case the server made it after all.
 */
const pending = <A, E, R>(next: WorkspaceLayout, change: Effect.Effect<A, E, R>) =>
  Effect.sync(() => mutations.addWorkspaceOverlay(next)).pipe(
    Effect.andThen(
      Semaphore.withPermit(
        lock,
        change.pipe(Effect.timeoutOrElse({ duration: SERVER_TIMEOUT, orElse: () => stalled })),
      ),
    ),
    Effect.ensuring(Effect.sync(() => mutations.dropWorkspaceOverlay(next))),
  );

/** Sends a call holding a row ticket, then lands `change` to the layout from its reply. */
const landing = <A, E, R>(
  request: Effect.Effect<A, E, R>,
  change: (reply: A) => (layout: WorkspaceLayout) => WorkspaceLayout,
) =>
  withRowTicket((since) =>
    Effect.tap(request, (reply) =>
      Effect.sync(() => mutations.landWorkspaceChange(change(reply), since)),
    ),
  );

/**
 * Moves a room into a shared category (`null`: out of every one) just before `beforeRoomId`, or
 * at the end with `null`, for everyone. Administrators only; the server refuses anyone else.
 */
export const moveRoom = Effect.fn("workspaceLayout.moveRoom")(function* (
  roomId: number,
  categoryId: number | null,
  beforeRoomId: number | null,
) {
  const current = view();

  if (isPlacedAt(current, roomId, categoryId, beforeRoomId)) {
    return;
  }

  // The category's real id, and whether the room it goes before is still there, are read when
  // the call's turn comes, against the layout the server has by then.
  yield* pending(
    withRoomMoved(current, roomId, categoryId, beforeRoomId),
    landing(
      Effect.suspend(() => {
        const target = categoryId === null ? null : serverCategoryId(categoryId);

        return api.moveWorkspaceRoom(
          roomId,
          moveRequest(confirmed(), roomId, target, beforeRoomId),
        );
      }),
      (layout) => () => layout,
    ),
  );
});

/** Adds a shared category at the end, shown at once under a temporary id. */
export const createCategory = Effect.fn("workspaceLayout.createCategory")(function* (name: string) {
  const current = view();
  const temporaryId = nextTemporaryId;

  nextTemporaryId -= 1;

  const draft: WorkspaceCategory = {
    id: temporaryId,
    name: name.trim(),
    position: (sharedCategories(current).at(-1)?.position ?? -1) + 1,
  };

  return yield* pending(
    withCategory(current, draft),
    landing(api.createWorkspaceCategory({ name: draft.name }), (created) => {
      createdIds.set(temporaryId, created.id);

      return (layout) => withCategory(layout, created);
    }),
  );
});

/** Renames a shared category (trimmed; 1 to 50 characters, or the server says 422). */
export const renameCategory = Effect.fn("workspaceLayout.renameCategory")(function* (
  categoryId: number,
  name: string,
) {
  const current = view();
  const category = current.categories.find((candidate) => candidate.id === categoryId);

  if (category === undefined) {
    return;
  }

  const trimmed = name.trim();

  yield* pending(
    withCategory(current, { ...category, name: trimmed }),
    landing(
      Effect.suspend(() =>
        api.updateWorkspaceCategory(serverCategoryId(categoryId), { name: trimmed }),
      ),
      (renamed) => (layout) => withCategory(layout, renamed),
    ),
  );
});

/** Deletes a shared category; its rooms join the uncategorized ones, at the end. */
export const deleteCategory = Effect.fn("workspaceLayout.deleteCategory")(function* (
  categoryId: number,
) {
  yield* pending(
    withoutCategory(view(), categoryId),
    Effect.suspend(() => {
      const serverId = serverCategoryId(categoryId);

      return landing(
        api.deleteWorkspaceCategory(serverId),
        () => (layout) => withoutCategory(layout, serverId),
      );
    }),
  );
});

/**
 * Puts the shared categories in this order (every one of them, once). When another
 * administrator added or removed one meanwhile the server answers 409: the sidebar is fetched
 * again and the `Conflict` fails the action, so the fresh list shows before a retry.
 */
export const reorderCategories = Effect.fn("workspaceLayout.reorderCategories")(function* (
  categoryIds: readonly number[],
) {
  yield* pending(
    withCategoriesOrdered(view(), categoryIds),
    landing(
      Effect.suspend(() => api.reorderWorkspaceCategories(categoryIds.map(serverCategoryId))),
      (layout) => () => layout,
    ).pipe(
      Effect.catchTag("Conflict", (conflict) =>
        refetchSidebar.pipe(Effect.andThen(Effect.fail(conflict))),
      ),
    ),
  );
});
