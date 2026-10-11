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

/** The signed-in session a reply names: a Google hand-off is good only in its own. */
const session = Schema.optionalKey(Schema.Number);

export const SudoResponse = Schema.Union([
  Schema.Struct({ kind: Schema.Literal("ready"), reauthentication: SudoState, session }),
  Schema.Struct({
    kind: Schema.Literal("confirmed"),
    retry: Schema.NullOr(SudoRetry),
    attempt: Schema.optionalKey(Schema.String),
    session,
  }),
  Schema.Struct({ kind: Schema.Literal("navigate"), location: Schema.String, session }),
  Schema.Struct({ kind: Schema.Literal("error"), message: Schema.String }),
]);

export type SudoResponsePin = Assert<Pinned<typeof SudoResponse, WireResponse>>;

export type SudoStatePin = Assert<Pinned<typeof SudoState, WireState>>;
