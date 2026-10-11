import { Effect } from "effect";
import type { ChannelWebhook } from "../gen/ChannelWebhook.ts";
import type { ChannelWebhookCredential } from "../gen/ChannelWebhookCredential.ts";
import type { ChannelWebhookList } from "../gen/ChannelWebhookList.ts";
import type { CreateChannelWebhook } from "../gen/CreateChannelWebhook.ts";
import type { CreateRoom } from "../gen/CreateRoom.ts";
import type { GithubSubscription } from "../gen/GithubSubscription.ts";
import type { GithubSubscriptionList } from "../gen/GithubSubscriptionList.ts";
import type { InboundEmail } from "../gen/InboundEmail.ts";
import type { MessageDTO } from "../gen/MessageDTO.ts";
import type { RoomForm } from "../gen/RoomForm.ts";
import type { RoomKind } from "../gen/RoomKind.ts";
import type { RoomLeft } from "../gen/RoomLeft.ts";
import type { RoomMutation } from "../gen/RoomMutation.ts";
import type { RoomRemoved } from "../gen/RoomRemoved.ts";
import type { SubscribeGithubRepository } from "../gen/SubscribeGithubRepository.ts";
import type { UpdateChannelWebhook } from "../gen/UpdateChannelWebhook.ts";
import type { UpdateGithubSubscription } from "../gen/UpdateGithubSubscription.ts";
import type { UpdateRoom } from "../gen/UpdateRoom.ts";
import { call, get, noContent } from "./call.ts";
import { MessageDTO as MessageDTOSchema } from "./schema/message.ts";
import {
  ChannelWebhookCredential as ChannelWebhookCredentialSchema,
  ChannelWebhookList as ChannelWebhookListSchema,
  ChannelWebhook as ChannelWebhookSchema,
  GithubSubscriptionList as GithubSubscriptionListSchema,
  GithubSubscription as GithubSubscriptionSchema,
  InboundEmail as InboundEmailSchema,
} from "./schema/room-integrations.ts";
import {
  RoomForm as RoomFormSchema,
  RoomLeft as RoomLeftSchema,
  RoomMutation as RoomMutationSchema,
  RoomRemoved as RoomRemovedSchema,
} from "./schema/room-management.ts";
import { wire } from "./wire.ts";

export const newRoom = Effect.fn("api.newRoom")(function* (type: RoomKind) {
  return yield* call(get("/rooms/new", { type }), wire<RoomForm>(RoomFormSchema));
});

export const editRoom = Effect.fn("api.editRoom")(function* (roomId: number) {
  return yield* call(get(`/rooms/${roomId}/edit`), wire<RoomForm>(RoomFormSchema));
});

/** A retry must reuse this request's clientRoomId. */
export const createRoom = Effect.fn("api.createRoom")(function* (body: CreateRoom) {
  return yield* call(
    { method: "POST", path: "/rooms", body },
    wire<RoomMutation>(RoomMutationSchema),
  );
});

export const updateRoom = Effect.fn("api.updateRoom")(function* (roomId: number, body: UpdateRoom) {
  return yield* call(
    { method: "PATCH", path: `/rooms/${roomId}`, body },
    wire<RoomMutation>(RoomMutationSchema),
  );
});

export const removeRoom = Effect.fn("api.removeRoom")(function* (roomId: number) {
  return yield* call(
    { method: "DELETE", path: `/rooms/${roomId}` },
    wire<RoomRemoved>(RoomRemovedSchema),
  );
});

/** The leave action offered by a direct room's edit page. */
export const leaveDirectRoom = Effect.fn("api.leaveDirectRoom")(function* (roomId: number) {
  return yield* call(
    { method: "DELETE", path: `/rooms/${roomId}/membership` },
    wire<RoomLeft>(RoomLeftSchema),
  );
});

/** Repositories whose pull-request events post into the room, plus the subscribe form's catalog. */
export const githubSubscriptions = Effect.fn("api.githubSubscriptions")(function* (roomId: number) {
  return yield* call(
    get(`/rooms/${roomId}/github_subscriptions`),
    wire<GithubSubscriptionList>(GithubSubscriptionListSchema),
  );
});

export const subscribeRepository = Effect.fn("api.subscribeRepository")(function* (
  roomId: number,
  body: SubscribeGithubRepository,
) {
  return yield* call(
    { method: "POST", path: `/rooms/${roomId}/github_subscriptions`, body },
    wire<GithubSubscription>(GithubSubscriptionSchema),
  );
});

export const updateGithubSubscription = Effect.fn("api.updateGithubSubscription")(function* (
  roomId: number,
  subscriptionId: number,
  body: UpdateGithubSubscription,
) {
  return yield* call(
    { method: "PATCH", path: `/rooms/${roomId}/github_subscriptions/${subscriptionId}`, body },
    wire<GithubSubscription>(GithubSubscriptionSchema),
  );
});

export const unsubscribeRepository = Effect.fn("api.unsubscribeRepository")(function* (
  roomId: number,
  subscriptionId: number,
) {
  return yield* call(
    { method: "DELETE", path: `/rooms/${roomId}/github_subscriptions/${subscriptionId}` },
    wire<GithubSubscription>(GithubSubscriptionSchema),
  );
});

/** The room's forward-to-room address, when inbound email is configured. */
export const inboundEmail = Effect.fn("api.inboundEmail")(function* (roomId: number) {
  return yield* call(get(`/rooms/${roomId}/inbound_email`), wire<InboundEmail>(InboundEmailSchema));
});

/** Creates the address, or rotates it. The old address stops working. */
export const rotateInboundEmail = Effect.fn("api.rotateInboundEmail")(function* (roomId: number) {
  return yield* call(
    { method: "POST", path: `/rooms/${roomId}/inbound_email` },
    wire<InboundEmail>(InboundEmailSchema),
  );
});

/** The room's incoming webhooks, for its creator and administrators. No tokens. */
export const channelWebhooks = Effect.fn("api.channelWebhooks")(function* (roomId: number) {
  return yield* call(
    get(`/rooms/${roomId}/webhooks`),
    wire<ChannelWebhookList>(ChannelWebhookListSchema),
  );
});

/** Creates a webhook; the reply is the only time its URL is readable. */
export const createChannelWebhook = Effect.fn("api.createChannelWebhook")(function* (
  roomId: number,
  body: CreateChannelWebhook,
) {
  return yield* call(
    { method: "POST", path: `/rooms/${roomId}/webhooks`, body },
    wire<ChannelWebhookCredential>(ChannelWebhookCredentialSchema),
  );
});

export const updateChannelWebhook = Effect.fn("api.updateChannelWebhook")(function* (
  roomId: number,
  webhookId: number,
  body: UpdateChannelWebhook,
) {
  return yield* call(
    { method: "PATCH", path: `/rooms/${roomId}/webhooks/${webhookId}`, body },
    wire<ChannelWebhook>(ChannelWebhookSchema),
  );
});

export const deleteChannelWebhook = Effect.fn("api.deleteChannelWebhook")(function* (
  roomId: number,
  webhookId: number,
) {
  return yield* call(
    { method: "DELETE", path: `/rooms/${roomId}/webhooks/${webhookId}` },
    noContent,
  );
});

/** A new token and URL; the old URL stops working. */
export const regenerateChannelWebhook = Effect.fn("api.regenerateChannelWebhook")(function* (
  roomId: number,
  webhookId: number,
) {
  return yield* call(
    { method: "POST", path: `/rooms/${roomId}/webhooks/${webhookId}/regenerate_token` },
    wire<ChannelWebhookCredential>(ChannelWebhookCredentialSchema),
  );
});

/** Posts a fixed test message into the room as the webhook. */
export const testChannelWebhook = Effect.fn("api.testChannelWebhook")(function* (
  roomId: number,
  webhookId: number,
) {
  return yield* call(
    { method: "POST", path: `/rooms/${roomId}/webhooks/${webhookId}/test` },
    wire<MessageDTO>(MessageDTOSchema),
  );
});
