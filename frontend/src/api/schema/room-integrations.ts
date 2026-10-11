import { Schema } from "effect";
import type { ChannelWebhook as GeneratedChannelWebhook } from "../../gen/ChannelWebhook.ts";
import type { ChannelWebhookCredential as GeneratedChannelWebhookCredential } from "../../gen/ChannelWebhookCredential.ts";
import type { ChannelWebhookList as GeneratedChannelWebhookList } from "../../gen/ChannelWebhookList.ts";
import type { GithubEventChoice as GeneratedGithubEventChoice } from "../../gen/GithubEventChoice.ts";
import type { GithubSubscription as GeneratedGithubSubscription } from "../../gen/GithubSubscription.ts";
import type { GithubSubscriptionList as GeneratedGithubSubscriptionList } from "../../gen/GithubSubscriptionList.ts";
import type { InboundEmail as GeneratedInboundEmail } from "../../gen/InboundEmail.ts";
import type { SubscribeGithubRepository as GeneratedSubscribeGithubRepository } from "../../gen/SubscribeGithubRepository.ts";
import type { UpdateGithubSubscription as GeneratedUpdateGithubSubscription } from "../../gen/UpdateGithubSubscription.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";

/** One pull-request event the classic subscription form offers. */
export const GithubEventChoice = Schema.Struct({
  key: Schema.String,
  label: Schema.String,
  selectedByDefault: Schema.Boolean,
});

export type GithubEventChoicePin = Assert<
  Pinned<typeof GithubEventChoice, GeneratedGithubEventChoice>
>;

/** A repository whose pull-request events post into the room. */
export const GithubSubscription = Schema.Struct({
  id: Schema.Int,
  fullName: Schema.String,
  events: Schema.Array(Schema.String),
});

export type GithubSubscriptionPin = Assert<
  Pinned<typeof GithubSubscription, GeneratedGithubSubscription>
>;

/** `GET /api/v1/rooms/:room_id/github_subscriptions`. */
export const GithubSubscriptionList = Schema.Struct({
  subscriptions: Schema.Array(GithubSubscription),
  administrator: Schema.Boolean,
  connectPath: Schema.NullOr(Schema.String),
  events: Schema.Array(GithubEventChoice),
});

export type GithubSubscriptionListPin = Assert<
  Pinned<typeof GithubSubscriptionList, GeneratedGithubSubscriptionList>
>;

/** `POST /api/v1/rooms/:room_id/github_subscriptions`. */
export const SubscribeGithubRepository = Schema.Struct({
  fullName: Schema.String,
  events: Schema.Array(Schema.String),
  skipAccessCheck: Schema.Boolean,
});

export type SubscribeGithubRepositoryPin = Assert<
  Pinned<typeof SubscribeGithubRepository, GeneratedSubscribeGithubRepository>
>;

/** `PATCH /api/v1/rooms/:room_id/github_subscriptions/:id`. */
export const UpdateGithubSubscription = Schema.Struct({
  events: Schema.Array(Schema.String),
});

export type UpdateGithubSubscriptionPin = Assert<
  Pinned<typeof UpdateGithubSubscription, GeneratedUpdateGithubSubscription>
>;

/** `GET` and `POST /api/v1/rooms/:room_id/inbound_email`. */
export const InboundEmail = Schema.Struct({
  enabled: Schema.Boolean,
  address: Schema.NullOr(Schema.String),
});

export type InboundEmailPin = Assert<Pinned<typeof InboundEmail, GeneratedInboundEmail>>;

/** A room's incoming webhook. The token never appears here, only in a credential. */
export const ChannelWebhook = Schema.Struct({
  id: Schema.Int,
  roomId: Schema.Int,
  creatorId: Schema.Int,
  name: Schema.String,
  avatarUrl: Schema.NullOr(Schema.String),
  lastUsedAt: Schema.NullOr(Timestamp),
  createdAt: Timestamp,
  updatedAt: Timestamp,
});

export type ChannelWebhookPin = Assert<Pinned<typeof ChannelWebhook, GeneratedChannelWebhook>>;

/** `GET /api/v1/rooms/:room_id/webhooks`. */
export const ChannelWebhookList = Schema.Struct({
  webhooks: Schema.Array(ChannelWebhook),
});

export type ChannelWebhookListPin = Assert<
  Pinned<typeof ChannelWebhookList, GeneratedChannelWebhookList>
>;

/** Create and regenerate: the only replies that carry the token and its execution URL. */
export const ChannelWebhookCredential = Schema.Struct({
  webhook: ChannelWebhook,
  token: Schema.String,
  url: Schema.String,
});

export type ChannelWebhookCredentialPin = Assert<
  Pinned<typeof ChannelWebhookCredential, GeneratedChannelWebhookCredential>
>;
