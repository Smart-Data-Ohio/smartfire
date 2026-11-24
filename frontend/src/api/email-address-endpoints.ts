/**
 * Linked email addresses: the account's list, adding one by emailed confirmation, removing one
 * and making one primary (each write may ask for a fresh confirmation), and the confirmation
 * link's own endpoints, which a signed-out browser can use.
 */
import { Effect, Schema } from "effect";
import type { EmailAddressList } from "../gen/EmailAddressList.ts";
import type { EmailAddressRequestResult } from "../gen/EmailAddressRequestResult.ts";
import { call, get, noContent } from "./call.ts";
import { ApiClient } from "./client.ts";
import {
  EmailAddressConfirmationAnswer,
  EmailAddressList as EmailAddressListSchema,
  EmailAddressRequestResult as EmailAddressRequestResultSchema,
} from "./schema/email-addresses.ts";
import { wire } from "./wire.ts";

const ROOT = "/email_addresses";

const listReply = wire<EmailAddressList>(EmailAddressListSchema);

const requestReply = wire<EmailAddressRequestResult>(EmailAddressRequestResultSchema);

const decodeConfirmation = Schema.decodeUnknownEffect(EmailAddressConfirmationAnswer);

const confirmationPath = (token: string): string =>
  `${ROOT}/confirmations/${encodeURIComponent(token)}`;

/** `GET /email_addresses`: the primary, the linked addresses, and whether adding works. */
export const emailAddresses = Effect.fn("api.emailAddresses")(function* () {
  return yield* call(get(ROOT), listReply);
});

/** `POST /email_addresses`: emails a confirmation link; the answer never says if it's taken. */
export const addEmailAddress = Effect.fn("api.addEmailAddress")(function* (emailAddress: string) {
  return yield* call({ method: "POST", path: ROOT, body: { emailAddress } }, requestReply);
});

/** `DELETE /email_addresses/:id` (204): unlinks the address. */
export const removeEmailAddress = Effect.fn("api.removeEmailAddress")(function* (id: number) {
  return yield* call({ method: "DELETE", path: `${ROOT}/${id}` }, noContent);
});

/** `POST /email_addresses/:id/primary` (204): it becomes primary; the old primary stays linked. */
export const setPrimaryEmailAddress = Effect.fn("api.setPrimaryEmailAddress")(function* (
  id: number,
) {
  return yield* call({ method: "POST", path: `${ROOT}/${id}/primary` }, noContent);
});

/** `GET /email_addresses/confirmations/:token`: the link's address, or why it can't be used. */
export const readEmailConfirmation = Effect.fn("api.readEmailConfirmation")(function* (
  token: string,
) {
  const client = yield* ApiClient;

  return yield* client.execute(
    { method: "GET", path: confirmationPath(token), auth: true },
    decodeConfirmation,
  );
});

/** `PUT /email_addresses/confirmations/:token`: links the address; success names the next page. */
export const confirmEmailAddress = Effect.fn("api.confirmEmailAddress")(function* (token: string) {
  const client = yield* ApiClient;

  return yield* client.execute(
    { method: "PUT", path: confirmationPath(token), auth: true },
    decodeConfirmation,
  );
});
