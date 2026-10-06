import { Schema } from "effect";
import type { SavedFilter as GeneratedSavedFilter } from "../../gen/SavedFilter.ts";
import type { SavedItemList as GeneratedSavedItemList } from "../../gen/SavedItemList.ts";
import type { UpdateSavedItem as GeneratedUpdateSavedItem } from "../../gen/UpdateSavedItem.ts";
import { SavedItem, SavedStatus } from "./actions.ts";
import { ConversationName } from "./conversation.ts";
import { SavedItemId } from "./ids.ts";
import { MessageDTO } from "./message.ts";
import type { Assert, Pinned } from "./pin.ts";
import { User } from "./user.ts";

export const SavedFilter = Schema.Literals(["all", "in_progress", "done"]);

export type SavedFilter = typeof SavedFilter.Type;

export type SavedFilterPin = Assert<Pinned<typeof SavedFilter, GeneratedSavedFilter>>;

/** `GET /api/v1/saved?status=&before=`: newest saved first, 50 a page, with the messages. */
export const SavedItemList = Schema.Struct({
  items: Schema.Array(SavedItem),
  messages: Schema.Array(MessageDTO),
  users: Schema.Array(User),
  conversations: Schema.Array(ConversationName),
  nextCursor: Schema.NullOr(SavedItemId),
});

export type SavedItemList = typeof SavedItemList.Type;

export type SavedItemListPin = Assert<Pinned<typeof SavedItemList, GeneratedSavedItemList>>;

/** The body of `PATCH /api/v1/saved/:id`: done or reopened. The reminder moves by re-saving. */
export const UpdateSavedItem = Schema.Struct({ status: SavedStatus });

export type UpdateSavedItem = typeof UpdateSavedItem.Type;

export type UpdateSavedItemPin = Assert<Pinned<typeof UpdateSavedItem, GeneratedUpdateSavedItem>>;
