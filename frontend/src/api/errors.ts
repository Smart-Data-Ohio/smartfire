import { Schema } from "effect";
import type { ApiError as GeneratedApiError } from "../gen/ApiError.ts";
import type { ApiErrorResponse as GeneratedApiErrorResponse } from "../gen/ApiErrorResponse.ts";
import type { Assert, Pinned } from "./schema/pin.ts";
import { SudoState } from "./schema/sudo.ts";

/** 401: not signed in, or the session ended. The app sends the person to sign in. */
export class Unauthorized extends Schema.TaggedError<Unauthorized>()("Unauthorized", {
  message: Schema.String,
}) {}

/** 403: signed in, but not allowed. */
export class Forbidden extends Schema.TaggedError<Forbidden>()("Forbidden", {
  message: Schema.String,
}) {}

/** 403: confirm one of the available credentials, then retry the pending write. */
export class SudoRequired extends Schema.TaggedError<SudoRequired>()("SudoRequired", {
  message: Schema.String,
  reauthentication: SudoState,
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

/** 422: the CSRF token was missing or stale. The client refreshes it and retries once. */
export class InvalidAuthenticityToken extends Schema.TaggedError<InvalidAuthenticityToken>()(
  "InvalidAuthenticityToken",
  { message: Schema.String },
) {}

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

/** 503: the feature isn't set up on this server ("Huddles are not configured"); hide it. */
export class Unavailable extends Schema.TaggedError<Unavailable>()("Unavailable", {
  message: Schema.String,
}) {}

export class FizzyNotConnected extends Schema.TaggedError<FizzyNotConnected>()(
  "FizzyNotConnected",
  {
    message: Schema.String,
  },
) {}

export class FizzyUnreachable extends Schema.TaggedError<FizzyUnreachable>()("FizzyUnreachable", {
  message: Schema.String,
}) {}

export class FizzyTokenRejected extends Schema.TaggedError<FizzyTokenRejected>()(
  "FizzyTokenRejected",
  {
    message: Schema.String,
  },
) {}

export class FizzyReadOnly extends Schema.TaggedError<FizzyReadOnly>()("FizzyReadOnly", {
  message: Schema.String,
}) {}

export class FizzyRefused extends Schema.TaggedError<FizzyRefused>()("FizzyRefused", {
  message: Schema.String,
}) {}

export class FizzyThreadLocked extends Schema.TaggedError<FizzyThreadLocked>()(
  "FizzyThreadLocked",
  {
    message: Schema.String,
  },
) {}

/** The card already exists. Show this message without retrying card creation. */
export class FizzyReplyFailed extends Schema.TaggedError<FizzyReplyFailed>()("FizzyReplyFailed", {
  message: Schema.String,
  number: Schema.String,
  url: Schema.String,
}) {}

/** Every typed `/api/v1` error, told apart by `_tag`. */
export const ApiError = Schema.Union([
  Unauthorized,
  Forbidden,
  SudoRequired,
  TwoFactorRequired,
  NotFound,
  Conflict,
  InvalidAuthenticityToken,
  Validation,
  RateLimited,
  Unavailable,
  FizzyNotConnected,
  FizzyUnreachable,
  FizzyTokenRejected,
  FizzyReadOnly,
  FizzyRefused,
  FizzyThreadLocked,
  FizzyReplyFailed,
]);

export type ApiError = typeof ApiError.Type;

export type ApiErrorPin = Assert<Pinned<typeof ApiError, GeneratedApiError>>;

/** The body of a failed `/api/v1` response. */
export const ApiErrorResponse = Schema.Struct({ error: ApiError });

export type ApiErrorResponsePin = Assert<
  Pinned<typeof ApiErrorResponse, GeneratedApiErrorResponse>
>;

/** The request got no response at all: offline, a dropped connection, DNS. Worth retrying. */
export class NetworkError extends Schema.TaggedError<NetworkError>()("NetworkError", {
  message: Schema.String,
}) {}

/**
 * A response the client can't use: a 5xx, an error body outside the contract, or a success body
 * that fails its schema. `status` is the HTTP status (0 when there was none to speak of).
 */
export class ServerError extends Schema.TaggedError<ServerError>()("ServerError", {
  status: Schema.Int,
  message: Schema.String,
}) {}

/** Everything an `/api/v1` call can fail with. */
export type ApiFailure = ApiError | NetworkError | ServerError;
