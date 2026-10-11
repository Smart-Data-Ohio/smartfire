import { Schema } from "effect";
import type { AuthResponse as WireResponse } from "../../gen/AuthResponse.ts";
import type { ChallengeState as WireChallenge } from "../../gen/ChallengeState.ts";
import type { SignedOutBoot as WireBoot } from "../../gen/SignedOutBoot.ts";
import type { Assert, Pinned } from "./pin.ts";

export const ChallengeState = Schema.Struct({
  methods: Schema.Array(Schema.Literals(["totp", "recoveryCode"])),
  rememberDevice: Schema.Boolean,
});

export const AuthResponse = Schema.Union([
  Schema.Struct({ kind: Schema.Literal("signedIn"), location: Schema.String }),
  Schema.Struct({ kind: Schema.Literal("secondFactorRequired"), challenge: ChallengeState }),
  Schema.Struct({ kind: Schema.Literal("navigate"), location: Schema.String }),
  Schema.Struct({
    kind: Schema.Literal("error"),
    fieldErrors: Schema.Record(Schema.String, Schema.Array(Schema.String)),
  }),
]);

export const SignedOutBoot = Schema.Struct({
  kind: Schema.Literal("signedOut"),
  workspace: Schema.Struct({
    name: Schema.NullOr(Schema.String),
    logoUrl: Schema.NullOr(Schema.String),
    description: Schema.String,
  }),
  signInMethods: Schema.Struct({
    password: Schema.Boolean,
    google: Schema.Boolean,
    googleDomains: Schema.Array(Schema.String),
  }),
  firstRunPending: Schema.Boolean,
  helpContact: Schema.NullOr(Schema.Struct({ name: Schema.String, emailAddress: Schema.String })),
  version: Schema.String,
  csrfToken: Schema.String,
});

export type AuthResponsePin = Assert<Pinned<typeof AuthResponse, WireResponse>>;

export type ChallengeStatePin = Assert<Pinned<typeof ChallengeState, WireChallenge>>;

export type SignedOutBootPin = Assert<Pinned<typeof SignedOutBoot, WireBoot>>;
