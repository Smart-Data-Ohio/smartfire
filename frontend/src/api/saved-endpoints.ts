/**
 * The S3 saved-for-later page's endpoints: a page of saved items and the done/reopen switch.
 * Saving and unsaving are the S2 `POST /saved` and `DELETE /saved/:id` in message-endpoints.ts.
 */
import { Effect } from "effect";
import type { SavedFilter } from "../gen/SavedFilter.ts";
import type { SavedItem } from "../gen/SavedItem.ts";
import type { SavedItemList } from "../gen/SavedItemList.ts";
import type { SavedStatus } from "../gen/SavedStatus.ts";
import { call, get } from "./call.ts";
import { SavedItem as SavedItemSchema } from "./schema/actions.ts";
import { SavedItemList as SavedItemListSchema } from "./schema/saved.ts";
import { wire } from "./wire.ts";

/**
 * `GET /saved?status=&before=`: 50 saved items, newest saved first, with their messages, the
 * messages' authors and the conversations they're in. `before` is the previous `nextCursor`.
 */
export const savedList = Effect.fn("api.savedList")(function* (
  status: SavedFilter,
  before: string | null,
) {
  const query = before === null ? { status } : { status, before };

  return yield* call(get("/saved", query), wire<SavedItemList>(SavedItemListSchema));
});

/** `PATCH /saved/:id`: mark it done or reopen it. The reminder moves by saving again. */
export const updateSavedItem = Effect.fn("api.updateSavedItem")(function* (
  savedItemId: number,
  status: SavedStatus,
) {
  return yield* call(
    { method: "PATCH", path: `/saved/${savedItemId}`, body: { status } },
    wire<SavedItem>(SavedItemSchema),
  );
});
