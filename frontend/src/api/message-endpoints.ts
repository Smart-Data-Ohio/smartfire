/**
 * The S2 message actions: edit, delete, reactions and boosts, pins, saves, forwards, and the
 * direct-upload start. Each validates the reply with its pinned schema (see endpoints.ts).
 */
import { Effect } from "effect";
import type { CreateForwards } from "../gen/CreateForwards.ts";
import type { CreateUpload } from "../gen/CreateUpload.ts";
import type { DirectUpload } from "../gen/DirectUpload.ts";
import type { ForwardDestinationList } from "../gen/ForwardDestinationList.ts";
import type { ForwardResult } from "../gen/ForwardResult.ts";
import type { MessageDTO } from "../gen/MessageDTO.ts";
import type { MessageReactions } from "../gen/MessageReactions.ts";
import type { MessageSource } from "../gen/MessageSource.ts";
import type { PinList } from "../gen/PinList.ts";
import type { PinState } from "../gen/PinState.ts";
import type { SavedItem } from "../gen/SavedItem.ts";
import { call, get, noContent } from "./call.ts";
import {
  ForwardDestinationList as ForwardDestinationListSchema,
  ForwardResult as ForwardResultSchema,
  PinList as PinListSchema,
  PinState as PinStateSchema,
  SavedItem as SavedItemSchema,
} from "./schema/actions.ts";
import { DirectUpload as DirectUploadSchema } from "./schema/attachment.ts";
import {
  MessageDTO as MessageSchema,
  MessageSource as MessageSourceSchema,
} from "./schema/message.ts";
import { MessageReactions as MessageReactionsSchema } from "./schema/reaction.ts";
import { wire } from "./wire.ts";

/** `PATCH /messages/:id`: the creator's edit; answers the updated message. */
export const updateMessage = Effect.fn("api.updateMessage")(function* (
  messageId: number,
  markdownSource: string,
) {
  return yield* call(
    { method: "PATCH", path: `/messages/${messageId}`, body: { markdownSource } },
    wire<MessageDTO>(MessageSchema),
  );
});

/** `GET /messages/:id/source`: the Markdown the edit box starts from. */
export const messageSource = Effect.fn("api.messageSource")(function* (messageId: number) {
  return yield* call(
    get(`/messages/${messageId}/source`),
    wire<MessageSource>(MessageSourceSchema),
  );
});

/** `DELETE /messages/:id` (204); the `message.removed` event follows. */
export const deleteMessage = Effect.fn("api.deleteMessage")(function* (messageId: number) {
  return yield* call({ method: "DELETE", path: `/messages/${messageId}` }, noContent);
});

/** `POST /messages/:id/boosts`: a reaction toggles the viewer's; other text boosts. */
export const react = Effect.fn("api.react")(function* (messageId: number, content: string) {
  return yield* call(
    { method: "POST", path: `/messages/${messageId}/boosts`, body: { content } },
    wire<MessageReactions>(MessageReactionsSchema),
  );
});

/** `DELETE /messages/:id/boosts/:boostId`: the booster takes a boost back. */
export const deleteBoost = Effect.fn("api.deleteBoost")(function* (
  messageId: number,
  boostId: number,
) {
  return yield* call(
    { method: "DELETE", path: `/messages/${messageId}/boosts/${boostId}` },
    wire<MessageReactions>(MessageReactionsSchema),
  );
});

/** `POST` (pin) or `DELETE` (unpin) `/messages/:id/pin`. */
export const setPinned = Effect.fn("api.setPinned")(function* (messageId: number, pinned: boolean) {
  return yield* call(
    { method: pinned ? "POST" : "DELETE", path: `/messages/${messageId}/pin` },
    wire<PinState>(PinStateSchema),
  );
});

/** `GET /rooms/:id/pins`: newest first, with the messages and their authors. */
export const pins = Effect.fn("api.pins")(function* (roomId: number) {
  return yield* call(get(`/rooms/${roomId}/pins`), wire<PinList>(PinListSchema));
});

/** `POST /saved`: save a message for later, optionally with a reminder. */
export const saveMessage = Effect.fn("api.saveMessage")(function* (
  messageId: number,
  remindAt: string | null,
) {
  return yield* call(
    { method: "POST", path: "/saved", body: { messageId, remindAt } },
    wire<SavedItem>(SavedItemSchema),
  );
});

/** `DELETE /saved/:id` (204). */
export const unsave = Effect.fn("api.unsave")(function* (savedItemId: number) {
  return yield* call({ method: "DELETE", path: `/saved/${savedItemId}` }, noContent);
});

/** `GET /forward_destinations`: rooms (and their open threads) the viewer may forward to. */
export const forwardDestinations = Effect.fn("api.forwardDestinations")(function* () {
  return yield* call(
    get("/forward_destinations"),
    wire<ForwardDestinationList>(ForwardDestinationListSchema),
  );
});

/** `POST /messages/:id/forwards`: one copy per destination, in order. */
export const forward = Effect.fn("api.forward")(function* (
  messageId: number,
  body: CreateForwards,
) {
  return yield* call(
    {
      method: "POST",
      path: `/messages/${messageId}/forwards`,
      body,
    },
    wire<ForwardResult>(ForwardResultSchema),
  );
});

/** `POST /uploads`: starts a direct upload; `PUT` the bytes to `uploadUrl` next. */
export const createUpload = Effect.fn("api.createUpload")(function* (body: CreateUpload) {
  return yield* call(
    {
      method: "POST",
      path: "/uploads",
      body,
    },
    wire<DirectUpload>(DirectUploadSchema),
  );
});
