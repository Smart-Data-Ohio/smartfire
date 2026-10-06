import { Schema } from "effect";
import type { ApiError as GeneratedApiError } from "../gen/ApiError.ts";
import type { ApiErrorResponse as GeneratedApiErrorResponse } from "../gen/ApiErrorResponse.ts";
import type { Assert, Pinned } from "./schema/pin.ts";

/** 401: not signed in, or the session ended. The app sends the person to sign in. */
export class Unauthorized extends Schema.TaggedError<Unauthorized>()("Unauthorized", {
  message: Schema.String,
}) {}

/** 403: signed in, but not allowed. */
export class Forbidden extends Schema.TaggedError<Forbidden>()("Forbidden", {
  message: Schema.String,
}) {}

/** 403: the action needs a fresh password confirmation (`/sudo/new`). */
export class SudoRequired extends Schema.TaggedError<SudoRequired>()("SudoRequired", {
  message: Schema.String,
}) {}

/** 403: two-factor setup or verification comes first. */
export class TwoFactorRequired extends Schema.TaggedError<TwoFactorRequired>()(
  "TwoFactorRequired",
  { message: Schema.String },
) {}

export class NotFound extends Schema.TaggedError<NotFound>()("NotFound", {
  message: Schema.String,
}) {}

/** 409: the record changed underneath the request. */
export class Conflict extends Schema.TaggedError<Conflict>()("Conflict", {
  message: Schema.String,
}) {}

/** 422: `fields` maps each attribute to its messages. */
export class Validation extends Schema.TaggedError<Validation>()("Validation", {
  message: Schema.String,
  fields: Schema.Record(Schema.String, Schema.Array(Schema.String)),
}) {}

/** 429: try again after `retryAfter` seconds. */
export class RateLimited extends Schema.TaggedError<RateLimited>()("RateLimited", {
  message: Schema.String,
  retryAfter: Schema.Int,
}) {}

/** Every typed `/api/v1` error, told apart by `_tag`. */
export const ApiError = Schema.Union([
  Unauthorized,
  Forbidden,
  SudoRequired,
  TwoFactorRequired,
  NotFound,
  Conflict,
  Validation,
  RateLimited,
]);

export type ApiError = typeof ApiError.Type;

export type ApiErrorPin = Assert<Pinned<typeof ApiError, GeneratedApiError>>;

/** The body of a failed `/api/v1` response. */
export const ApiErrorResponse = Schema.Struct({ error: ApiError });

export type ApiErrorResponsePin = Assert<
  Pinned<typeof ApiErrorResponse, GeneratedApiErrorResponse>
>;
