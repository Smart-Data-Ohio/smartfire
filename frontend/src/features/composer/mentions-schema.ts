/** Session-storage metadata is parsed beside the editor that owns its ranges. */
import { Schema } from "effect";

const DraftMention = Schema.Struct({
  start: Schema.Int,
  text: Schema.String,
  token: Schema.String,
});

export type DraftMention = typeof DraftMention.Type;

export const decodeDraftMentions = Schema.decodeUnknownSync(
  Schema.fromJsonString(Schema.Array(DraftMention)),
);
