import { Schema } from "effect";
import type { CreateForwards as GeneratedCreateForwards } from "../../gen/CreateForwards.ts";
import type { ForwardDestination as GeneratedForwardDestination } from "../../gen/ForwardDestination.ts";
import type { ForwardDestinationList as GeneratedForwardDestinationList } from "../../gen/ForwardDestinationList.ts";
import type { ForwardResult as GeneratedForwardResult } from "../../gen/ForwardResult.ts";
import type { ForwardTarget as GeneratedForwardTarget } from "../../gen/ForwardTarget.ts";
import type { ForwardThread as GeneratedForwardThread } from "../../gen/ForwardThread.ts";
import type { Pin as GeneratedPin } from "../../gen/Pin.ts";
import type { PinList as GeneratedPinList } from "../../gen/PinList.ts";
import type { PinState as GeneratedPinState } from "../../gen/PinState.ts";
import type { SavedChanged as GeneratedSavedChanged } from "../../gen/SavedChanged.ts";
import type { SavedItem as GeneratedSavedItem } from "../../gen/SavedItem.ts";
import type { SavedStatus as GeneratedSavedStatus } from "../../gen/SavedStatus.ts";
import type { SaveMessage as GeneratedSaveMessage } from "../../gen/SaveMessage.ts";
import { MessageId, RoomId, SavedItemId, ThreadId, UserId } from "./ids.ts";
import { MessageDTO } from "./message.ts";
import type { Assert, Pinned } from "./pin.ts";
import { ThreadStatus } from "./thread-parts.ts";
import { Timestamp } from "./time.ts";
import { User } from "./user.ts";

/** A pin or unpin's result, and the `message.pinned` event. */
export const PinState = Schema.Struct({
  messageId: MessageId,
  roomId: RoomId,
  pinned: Schema.Boolean,
  pinCount: Schema.Int,
});

export type PinState = typeof PinState.Type;

export type PinStatePin = Assert<Pinned<typeof PinState, GeneratedPinState>>;

export const Pin = Schema.Struct({ messageId: MessageId, pinnerId: UserId, pinnedAt: Timestamp });

export type Pin = typeof Pin.Type;

export type PinPin = Assert<Pinned<typeof Pin, GeneratedPin>>;

/** `GET /api/v1/rooms/:id/pins`: newest first, at most 50, with the messages in full. */
export const PinList = Schema.Struct({
  pins: Schema.Array(Pin),
  messages: Schema.Array(MessageDTO),
  users: Schema.Array(User),
});

export type PinList = typeof PinList.Type;

export type PinListPin = Assert<Pinned<typeof PinList, GeneratedPinList>>;

export const SavedStatus = Schema.Literals(["in_progress", "done"]);

export type SavedStatus = typeof SavedStatus.Type;

export type SavedStatusPin = Assert<Pinned<typeof SavedStatus, GeneratedSavedStatus>>;

export const SavedItem = Schema.Struct({
  id: SavedItemId,
  messageId: MessageId,
  status: SavedStatus,
  remindAt: Schema.NullOr(Timestamp),
  remindedAt: Schema.NullOr(Timestamp),
  createdAt: Timestamp,
});

export type SavedItem = typeof SavedItem.Type;

export type SavedItemPin = Assert<Pinned<typeof SavedItem, GeneratedSavedItem>>;

/** The body of `POST /api/v1/saved`. Saving again only moves the reminder. */
export const SaveMessage = Schema.Struct({
  messageId: MessageId,
  remindAt: Schema.NullOr(Timestamp),
});

export type SaveMessage = typeof SaveMessage.Type;

export type SaveMessagePin = Assert<Pinned<typeof SaveMessage, GeneratedSaveMessage>>;

/** The `saved.changed` event: the item as it is now, or `null` once unsaved. */
export const SavedChanged = Schema.Struct({
  messageId: MessageId,
  item: Schema.NullOr(SavedItem),
});

export type SavedChanged = typeof SavedChanged.Type;

export type SavedChangedPin = Assert<Pinned<typeof SavedChanged, GeneratedSavedChanged>>;

export const ForwardThread = Schema.Struct({
  id: ThreadId,
  name: Schema.String,
  status: ThreadStatus,
});

export type ForwardThreadPin = Assert<Pinned<typeof ForwardThread, GeneratedForwardThread>>;

export const ForwardDestination = Schema.Struct({
  roomId: RoomId,
  name: Schema.String,
  direct: Schema.Boolean,
  threads: Schema.Array(ForwardThread),
});

export type ForwardDestination = typeof ForwardDestination.Type;

export type ForwardDestinationPin = Assert<
  Pinned<typeof ForwardDestination, GeneratedForwardDestination>
>;

/** `GET /api/v1/forward_destinations`: the viewer's rooms but boards, with their open threads. */
export const ForwardDestinationList = Schema.Struct({
  destinations: Schema.Array(ForwardDestination),
});

export type ForwardDestinationList = typeof ForwardDestinationList.Type;

export type ForwardDestinationListPin = Assert<
  Pinned<typeof ForwardDestinationList, GeneratedForwardDestinationList>
>;

export const ForwardTarget = Schema.Struct({ roomId: RoomId, threadId: Schema.NullOr(ThreadId) });

export type ForwardTarget = typeof ForwardTarget.Type;

export type ForwardTargetPin = Assert<Pinned<typeof ForwardTarget, GeneratedForwardTarget>>;

/** The body of `POST /api/v1/messages/:id/forwards`: 1 to 5 distinct destinations. */
export const CreateForwards = Schema.Struct({
  note: Schema.NullOr(Schema.String),
  destinations: Schema.Array(ForwardTarget),
});

export type CreateForwards = typeof CreateForwards.Type;

export type CreateForwardsPin = Assert<Pinned<typeof CreateForwards, GeneratedCreateForwards>>;

/** The forwarded copies, one per destination, in order. */
export const ForwardResult = Schema.Struct({ forwards: Schema.Array(MessageDTO) });

export type ForwardResult = typeof ForwardResult.Type;

export type ForwardResultPin = Assert<Pinned<typeof ForwardResult, GeneratedForwardResult>>;
