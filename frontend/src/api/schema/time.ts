import { Schema } from "effect";

/**
 * A timestamp: on the wire an RFC 3339 UTC string with milliseconds
 * (`"2026-09-26T12:26:46.848Z"`), decoded to a `DateTime.Utc`.
 */
export const Timestamp = Schema.DateTimeUtcFromString;

/**
 * A server row version: an RFC 3339 UTC string with exactly six fractional digits
 * (`"2026-09-26T12:26:46.848123Z"`). The fixed width makes string order time order, so the client
 * compares it as a string and never decodes it.
 */
export const RowTimestamp = Schema.String.pipe(
  Schema.check(Schema.isPattern(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{6}Z$/)),
);

/** The injected server clock used to evaluate a snapshot, in UTC with nanoseconds. */
export const EvaluationTimestamp = Schema.String.pipe(
  Schema.check(Schema.isPattern(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{9}Z$/)),
);
