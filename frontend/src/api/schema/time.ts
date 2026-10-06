import { Schema } from "effect";

/**
 * A timestamp: on the wire an RFC 3339 UTC string with milliseconds
 * (`"2026-09-26T12:26:46.848Z"`), decoded to a `DateTime.Utc`.
 */
export const Timestamp = Schema.DateTimeUtcFromString;
