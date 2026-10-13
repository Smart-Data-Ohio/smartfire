/**
 * MCP OAuth (`/api/v1/oauth/*`): the consent screen a client's sign-in lands on, and the
 * administrators' list of connected clients (`/api/v1/admin/oauth_clients`). Approving waits for a
 * fresh confirmation once the last has lapsed; its answer carries a one-time code in the client's
 * callback, so it is `secret` and never kept across a Google confirmation.
 */
import { Effect } from "effect";
import type { AdminOAuthClient } from "../gen/AdminOAuthClient.ts";
import type { AdminOAuthClientList } from "../gen/AdminOAuthClientList.ts";
import type { ApproveOAuthAuthorization } from "../gen/ApproveOAuthAuthorization.ts";
import type { OAuthConsent } from "../gen/OAuthConsent.ts";
import type { OAuthRedirect } from "../gen/OAuthRedirect.ts";
import { call, get } from "./call.ts";
import {
  AdminOAuthClientList as AdminOAuthClientListSchema,
  AdminOAuthClient as AdminOAuthClientSchema,
  OAuthConsent as OAuthConsentSchema,
  OAuthRedirect as OAuthRedirectSchema,
} from "./schema/oauth.ts";
import { wire } from "./wire.ts";

const ROOT = "/oauth/authorizations";

const redirectReply = wire<OAuthRedirect>(OAuthRedirectSchema);

/** The authorization's path segment, kept to what the server mints (base64url). */
function authorization(publicId: string): string {
  return `${ROOT}/${encodeURIComponent(publicId)}`;
}

/** `GET /oauth/authorizations/:publicId`: 404 once it has expired, been used, or isn't yours. */
export const oauthConsent = Effect.fn("api.oauthConsent")(function* (publicId: string) {
  return yield* call(get(authorization(publicId)), wire<OAuthConsent>(OAuthConsentSchema));
});

/** `POST …/approve`: the agent, its access and the connection in one write; the client's callback. */
export const approveOAuth = Effect.fn("api.approveOAuth")(function* (
  publicId: string,
  body: ApproveOAuthAuthorization,
) {
  return yield* call(
    { method: "POST", path: `${authorization(publicId)}/approve`, body, secret: true },
    redirectReply,
  );
});

/** `POST …/deny`: the client's callback, with `access_denied`. */
export const denyOAuth = Effect.fn("api.denyOAuth")(function* (publicId: string) {
  return yield* call(
    { method: "POST", path: `${authorization(publicId)}/deny`, body: {} },
    redirectReply,
  );
});

const ADMIN = "/admin/oauth_clients";

const clientReply = wire<AdminOAuthClient>(AdminOAuthClientSchema);

/** `GET /admin/oauth_clients`: the clients that have connected, or are blocked. */
export const oauthClients = Effect.fn("api.oauthClients")(function* () {
  return yield* call(get(ADMIN), wire<AdminOAuthClientList>(AdminOAuthClientListSchema));
});

/** `PUT /admin/oauth_clients/:id/block`, or `DELETE` to unblock. */
export const setOAuthClientBlocked = Effect.fn("api.setOAuthClientBlocked")(function* (
  clientId: number,
  blocked: boolean,
) {
  return yield* call(
    { method: blocked ? "PUT" : "DELETE", path: `${ADMIN}/${clientId}/block` },
    clientReply,
  );
});
