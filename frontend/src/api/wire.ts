import { Effect, Schema } from "effect";
import type { ResponseDecoder } from "./client.ts";

/**
 * A decoder that validates JSON with a pinned schema and hands back the JSON itself, typed as the
 * generated wire type `Wire`. The store keeps wire DTOs, so branded ids and `DateTime` values
 * never leave src/api; the schema's pin keeps `Wire` and the schema's encoded side equal.
 */
export function wire<Wire>(schema: Schema.Decoder<unknown>): ResponseDecoder<Wire> {
  const decode = Schema.decodeUnknownEffect(schema);

  // SAFETY: decoding succeeded, and the schema's encoded side is pinned to `Wire`.
  return (json) => Effect.as(decode(json), json as Wire);
}
