import { Schema } from "effect";
import type { TwoFactorRequirement as WireRequirement } from "../../gen/TwoFactorRequirement.ts";
import type { TwoFactorSetupResponse as WireResponse } from "../../gen/TwoFactorSetupResponse.ts";
import type { TwoFactorSetupState as WireState } from "../../gen/TwoFactorSetupState.ts";
import type { Assert, Pinned } from "./pin.ts";

export const TwoFactorSetupState = Schema.Struct({
  secret: Schema.String,
  manualKey: Schema.String,
  otpauthUri: Schema.String,
  qrSvg: Schema.String,
});

export const TwoFactorSetupResponse = Schema.Union([
  Schema.Struct({ kind: Schema.Literal("ready"), setup: TwoFactorSetupState }),
  Schema.Struct({
    kind: Schema.Literal("recoveryCodes"),
    codes: Schema.Array(Schema.String),
    signedOut: Schema.Int,
    continueUrl: Schema.String,
  }),
  Schema.Struct({ kind: Schema.Literal("navigate"), location: Schema.String }),
  Schema.Struct({
    kind: Schema.Literal("error"),
    message: Schema.String,
    setup: TwoFactorSetupState,
  }),
]);

export const TwoFactorRequirement = Schema.Union([
  Schema.Struct({ kind: Schema.Literal("setup"), location: Schema.String }),
  Schema.Struct({ kind: Schema.Literal("challenge"), location: Schema.String }),
]);

export type TwoFactorSetupStatePin = Assert<Pinned<typeof TwoFactorSetupState, WireState>>;

export type TwoFactorSetupResponsePin = Assert<Pinned<typeof TwoFactorSetupResponse, WireResponse>>;

export type TwoFactorRequirementPin = Assert<Pinned<typeof TwoFactorRequirement, WireRequirement>>;
