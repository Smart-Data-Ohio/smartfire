import { Schema } from "effect";
import type { Boost as GeneratedBoost } from "../../gen/Boost.ts";
import type { CreateBoost as GeneratedCreateBoost } from "../../gen/CreateBoost.ts";
import type { MessageReactions as GeneratedMessageReactions } from "../../gen/MessageReactions.ts";
import type { Reaction as GeneratedReaction } from "../../gen/Reaction.ts";
import { BoostId, MessageId, RoomId, ThreadId, UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";

/** A reaction pill: one content, everyone who reacted with it (the viewer's id = "you"). */
export const Reaction = Schema.Struct({
  content: Schema.String,
  title: Schema.String,
  imageUrl: Schema.NullOr(Schema.String),
  reactorIds: Schema.Array(UserId),
});

export type Reaction = typeof Reaction.Type;

export type ReactionPin = Assert<Pinned<typeof Reaction, GeneratedReaction>>;

/** A free-text boost chip. */
export const Boost = Schema.Struct({
  id: BoostId,
  boosterId: UserId,
  content: Schema.String,
  createdAt: Timestamp,
});

export type Boost = typeof Boost.Type;

export type BoostPin = Assert<Pinned<typeof Boost, GeneratedBoost>>;

/** The body of `POST /api/v1/messages/:id/boosts`: a reaction toggles, other text boosts. */
export const CreateBoost = Schema.Struct({ content: Schema.String });

export type CreateBoostPin = Assert<Pinned<typeof CreateBoost, GeneratedCreateBoost>>;

/** A message's reactions and boosts after a change: a reply and the `message.reactions` event. */
export const MessageReactions = Schema.Struct({
  messageId: MessageId,
  roomId: RoomId,
  threadId: Schema.NullOr(ThreadId),
  reactions: Schema.Array(Reaction),
  boosts: Schema.Array(Boost),
  updatedAt: Timestamp,
});

export type MessageReactions = typeof MessageReactions.Type;

export type MessageReactionsPin = Assert<
  Pinned<typeof MessageReactions, GeneratedMessageReactions>
>;
