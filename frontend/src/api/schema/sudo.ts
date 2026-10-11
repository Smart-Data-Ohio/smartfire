import { Schema } from "effect";
import type { SudoResponse as WireResponse } from "../../gen/SudoResponse.ts";
import type { SudoState as WireState } from "../../gen/SudoState.ts";
import type { Assert, Pinned } from "./pin.ts";

export const SudoRetry = Schema.Struct({
  method: Schema.String,
  path: Schema.String,
  returnTo: Schema.String,
});

export const SudoState = Schema.Struct({
  methods: Schema.Array(Schema.Literals(["password", "totp", "google"])),
  retry: Schema.NullOr(SudoRetry),
});

export const SudoResponse = Schema.Union([
  Schema.Struct({ kind: Schema.Literal("ready"), reauthentication: SudoState }),
  Schema.Struct({ kind: Schema.Literal("confirmed"), retry: Schema.NullOr(SudoRetry) }),
  Schema.Struct({ kind: Schema.Literal("navigate"), location: Schema.String }),
  Schema.Struct({ kind: Schema.Literal("error"), message: Schema.String }),
]);

export type SudoResponsePin = Assert<Pinned<typeof SudoResponse, WireResponse>>;

export type SudoStatePin = Assert<Pinned<typeof SudoState, WireState>>;
