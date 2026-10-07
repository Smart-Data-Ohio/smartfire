/**
 * The S7 bot endpoints (`/api/v1/admin/bots/*`): the classic chat bot pages. Administrators see
 * the list and every control; the person who owns a bot's agent may open and edit that bot (not
 * its webhook URL), suspend it, reset its signing secret, and revoke its credentials and grants.
 * The writes the classic pages guard with the password fail with `SudoRequired` once its
 * confirmation has lapsed.
 */
import { Effect } from "effect";
import type { Bot } from "../gen/Bot.ts";
import type { BotChange } from "../gen/BotChange.ts";
import type { BotKey } from "../gen/BotKey.ts";
import type { BotList } from "../gen/BotList.ts";
import type { BotRemoved } from "../gen/BotRemoved.ts";
import type { CreateBot } from "../gen/CreateBot.ts";
import type { CreateCredential } from "../gen/CreateCredential.ts";
import type { CreateGrant } from "../gen/CreateGrant.ts";
import type { CredentialCreated } from "../gen/CredentialCreated.ts";
import type { CredentialList } from "../gen/CredentialList.ts";
import type { GrantList } from "../gen/GrantList.ts";
import type { UpdateBot } from "../gen/UpdateBot.ts";
import { call, get } from "./call.ts";
import {
  BotChange as BotChangeSchema,
  BotKey as BotKeySchema,
  BotList as BotListSchema,
  BotRemoved as BotRemovedSchema,
  Bot as BotSchema,
  CredentialCreated as CredentialCreatedSchema,
  CredentialList as CredentialListSchema,
  GrantList as GrantListSchema,
} from "./schema/bots.ts";
import { wire } from "./wire.ts";

const keyReply = wire<BotKey>(BotKeySchema);

const changeReply = wire<BotChange>(BotChangeSchema);

const credentialsReply = wire<CredentialList>(CredentialListSchema);

const grantsReply = wire<GrantList>(GrantListSchema);

/** `GET /admin/bots`: the active bots, by name, with the commands that post as each. */
export const bots = Effect.fn("api.bots")(function* () {
  return yield* call(get("/admin/bots"), wire<BotList>(BotListSchema));
});

/** `POST /admin/bots`: a workspace agent the viewer owns; answers its key, shown once. */
export const createBot = Effect.fn("api.createBot")(function* (body: CreateBot) {
  return yield* call({ method: "POST", path: "/admin/bots", body }, keyReply);
});

/** `GET /admin/bots/:id`: one bot, as its classic edit page shows it. */
export const bot = Effect.fn("api.bot")(function* (botId: number) {
  return yield* call(get(`/admin/bots/${botId}`), wire<Bot>(BotSchema));
});

/** `PATCH /admin/bots/:id`: `null` leaves a field alone. */
export const updateBot = Effect.fn("api.updateBot")(function* (botId: number, body: UpdateBot) {
  return yield* call({ method: "PATCH", path: `/admin/bots/${botId}`, body }, changeReply);
});

/** `DELETE /admin/bots/:id`: deactivates the bot and suspends its agent. */
export const removeBot = Effect.fn("api.removeBot")(function* (botId: number) {
  return yield* call(
    { method: "DELETE", path: `/admin/bots/${botId}` },
    wire<BotRemoved>(BotRemovedSchema),
  );
});

/** `POST /admin/bots/:id/kill_switch`: suspends the agent and cancels its pending approvals. */
export const suspendBot = Effect.fn("api.suspendBot")(function* (botId: number) {
  return yield* call({ method: "POST", path: `/admin/bots/${botId}/kill_switch` }, changeReply);
});

/** `PUT /admin/bots/:id/key`: a new key, shown once; the old one stops working. */
export const resetBotKey = Effect.fn("api.resetBotKey")(function* (botId: number) {
  return yield* call({ method: "PUT", path: `/admin/bots/${botId}/key` }, keyReply);
});

/** `POST /admin/bots/:id/webhook_secret`: a new signing secret for its webhook deliveries. */
export const resetSigningSecret = Effect.fn("api.resetSigningSecret")(function* (botId: number) {
  return yield* call({ method: "POST", path: `/admin/bots/${botId}/webhook_secret` }, changeReply);
});

/** `PUT /admin/bots/:id/github_connection`: a token GitHub accepts links its account. */
export const connectGithub = Effect.fn("api.connectGithub")(function* (
  botId: number,
  accessToken: string,
) {
  return yield* call(
    { method: "PUT", path: `/admin/bots/${botId}/github_connection`, body: { accessToken } },
    changeReply,
  );
});

/** `DELETE /admin/bots/:id/github_connection`. */
export const disconnectGithub = Effect.fn("api.disconnectGithub")(function* (botId: number) {
  return yield* call(
    { method: "DELETE", path: `/admin/bots/${botId}/github_connection` },
    changeReply,
  );
});

/** `GET /admin/bots/:id/credentials`: newest first; a legacy bot gets its agent. */
export const credentials = Effect.fn("api.botCredentials")(function* (botId: number) {
  return yield* call(get(`/admin/bots/${botId}/credentials`), credentialsReply);
});

/** `POST /admin/bots/:id/credentials`: answers the secret, shown once, and the list. */
export const issueCredential = Effect.fn("api.issueCredential")(function* (
  botId: number,
  body: CreateCredential,
) {
  return yield* call(
    { method: "POST", path: `/admin/bots/${botId}/credentials`, body },
    wire<CredentialCreated>(CredentialCreatedSchema),
  );
});

/** `DELETE /admin/bots/:id/credentials/:credentialId`: answers the list. */
export const revokeCredential = Effect.fn("api.revokeCredential")(function* (
  botId: number,
  credentialId: number,
) {
  return yield* call(
    { method: "DELETE", path: `/admin/bots/${botId}/credentials/${credentialId}` },
    credentialsReply,
  );
});

/** `GET /admin/bots/:id/grants`: active first; a legacy bot gets its agent. */
export const grants = Effect.fn("api.botGrants")(function* (botId: number) {
  return yield* call(get(`/admin/bots/${botId}/grants`), grantsReply);
});

/** `POST /admin/bots/:id/grants`: answers the list (a grant already held changes nothing). */
export const createGrant = Effect.fn("api.createGrant")(function* (
  botId: number,
  body: CreateGrant,
) {
  return yield* call({ method: "POST", path: `/admin/bots/${botId}/grants`, body }, grantsReply);
});

/** `DELETE /admin/bots/:id/grants/:grantId`: answers the list. */
export const revokeGrant = Effect.fn("api.revokeGrant")(function* (botId: number, grantId: number) {
  return yield* call(
    { method: "DELETE", path: `/admin/bots/${botId}/grants/${grantId}` },
    grantsReply,
  );
});
