import { Schema } from "effect";
import type { EmailAddressConfirmation as WireConfirmation } from "../../gen/EmailAddressConfirmation.ts";
import type { EmailAddressEntry as WireEntry } from "../../gen/EmailAddressEntry.ts";
import type { EmailAddressList as WireList } from "../../gen/EmailAddressList.ts";
import type { EmailAddressRequestResult as WireRequestResult } from "../../gen/EmailAddressRequestResult.ts";
import { AuthResponse } from "./auth.ts";
import type { Assert, Pinned } from "./pin.ts";

/** A confirmed address linked to the account besides its primary. */
export const EmailAddressEntry = Schema.Struct({
  id: Schema.Number,
  emailAddress: Schema.String,
  createdAt: Schema.String,
});

/** The account's primary address, its linked ones, and whether new ones can be emailed. */
export const EmailAddressList = Schema.Struct({
  primary: Schema.NullOr(Schema.String),
  secondary: Schema.Array(EmailAddressEntry),
  canAdd: Schema.Boolean,
});

/** The same answer whether or not the address is free. */
export const EmailAddressRequestResult = Schema.Struct({
  kind: Schema.Literal("confirmationSent"),
  emailAddress: Schema.String,
});

/** A confirmation link that works: the address, and whether signing in follows. */
export const EmailAddressConfirmation = Schema.Struct({
  emailAddress: Schema.String,
  accountName: Schema.String,
  requiresSignIn: Schema.Boolean,
  accountMismatch: Schema.Boolean,
});

/** A confirmation endpoint's answer: the link's address, or a next action (a refusal, a page). */
export const EmailAddressConfirmationAnswer = Schema.Union([
  EmailAddressConfirmation,
  ...AuthResponse.members,
]);

export type EmailAddressEntryPin = Assert<Pinned<typeof EmailAddressEntry, WireEntry>>;

export type EmailAddressListPin = Assert<Pinned<typeof EmailAddressList, WireList>>;

export type EmailAddressRequestResultPin = Assert<
  Pinned<typeof EmailAddressRequestResult, WireRequestResult>
>;

export type EmailAddressConfirmationPin = Assert<
  Pinned<typeof EmailAddressConfirmation, WireConfirmation>
>;
