import { Schema } from "effect";

/**
 * The sync socket couldn't open, dropped, went silent, or broke the protocol (a `welcome` that
 * doesn't decode). The engine closes it and reconnects.
 */
export class SyncSocketError extends Schema.TaggedError<SyncSocketError>()("SyncSocketError", {
  reason: Schema.String,
}) {}
