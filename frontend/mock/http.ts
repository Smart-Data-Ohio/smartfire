/**
 * The mock's request and response types and the `ApiError` bodies it answers with, shared by
 * the S1 routes in server.ts and the S2 modules under s2/.
 */
import type { ApiError } from "../src/gen/ApiError.ts";
import type { Json } from "./json.ts";

/** Request headers, names in any case. */
export interface MockHeaders {
  readonly [name: string]: string | undefined;
}

/** An HTTP request as the transports hand it over. */
export interface MockRequest {
  readonly method: string;
  /** The path, e.g. `/api/v1/rooms/1/messages`; a `?query` here is read too. */
  readonly path: string;
  readonly query?: URLSearchParams | string | undefined;
  /** The parsed JSON body, if any. */
  readonly body?: Json | undefined;
  readonly headers?: MockHeaders | undefined;
}

export interface MockResponse {
  readonly status: number;
  readonly json: Json;
}

/**
 * A request for raw bytes: the direct-upload `PUT`, blob downloads and icon images. These
 * aren't JSON, so they take and answer bytes.
 */
export interface MockBinaryRequest {
  readonly method: string;
  /** The path, query included, e.g. `/rails/active_storage/blobs/redirect/…?disposition=attachment`. */
  readonly path: string;
  readonly headers?: MockHeaders | undefined;
  /** The request body (the uploaded file); `null` for none. */
  readonly bytes: Uint8Array | null;
}

export interface MockBinaryResponse {
  readonly status: number;
  /** `null` for an empty body. */
  readonly contentType: string | null;
  readonly bytes: Uint8Array;
  /** Extra headers, e.g. `Content-Disposition`. */
  readonly headers: Readonly<Record<string, string>>;
}

/** An `ApiError` on its way out: thrown by handlers, turned into a response by the router. */
export class HttpError extends Error {
  readonly status: number;
  readonly error: ApiError;

  constructor(status: number, error: ApiError) {
    super(error.message);
    this.status = status;
    this.error = error;
  }
}

/** The `ApiError` variants that carry only a message. */
type PlainErrorTag = Exclude<ApiError["_tag"], "Validation" | "RateLimited">;

/**
 * An error body exactly as the Rust server serializes it. This is wire JSON, not an Effect
 * tagged value (Effect stays out of the mock), so the tag is plain data here.
 */
export function plainError(status: number, tag: PlainErrorTag, message: string): HttpError {
  return new HttpError(status, { _tag: tag, message });
}

/** 404 `NotFound`. */
export const notFound = (message = "Not found") => plainError(404, "NotFound", message);

/** 403 `Forbidden`. */
export const forbidden = (message = "You aren't allowed to do that") =>
  plainError(403, "Forbidden", message);

/** 409 `Conflict`. */
export const conflict = (message: string) => plainError(409, "Conflict", message);

const VALIDATION: ApiError["_tag"] = "Validation";

/** 422 `Validation` on one field; `alert` may supply the classic controller's whole sentence. */
export const validation = (
  field: string,
  message: string,
  alert = `Validation failed: ${message}`,
) =>
  new HttpError(422, {
    _tag: VALIDATION,
    message: alert,
    fields: { [field]: [message] },
  });

/**
 * 422 `Validation` as the server answers a model's errors (`record_invalid`): each message under
 * its wire field as the model words it ("can't be blank"), and the message their full sentences
 * ("Summary can't be blank"), joined by commas.
 */
export const recordInvalid = (
  errors: readonly { readonly field: string; readonly label: string; readonly message: string }[],
) => {
  const fields: Record<string, string[]> = {};

  for (const error of errors) {
    fields[error.field] = [...(fields[error.field] ?? []), error.message];
  }

  return new HttpError(422, {
    _tag: VALIDATION,
    message: errors.map((error) => `${error.label} ${error.message}`).join(", "),
    fields,
  });
};

/** 422 `Validation` with one whole sentence, the message and its field's only entry. */
export const sentence = (field: string, message: string) =>
  new HttpError(422, { _tag: VALIDATION, message, fields: { [field]: [message] } });

/** 422 `Validation` with no fields: a classic alert, word for word (`api::admin::refusal`). */
export const refused = (message: string) =>
  new HttpError(422, { _tag: VALIDATION, message, fields: {} });

const RATE_LIMITED: ApiError["_tag"] = "RateLimited";

/** 429 `RateLimited` with the classic alert (`retryAfter` 0: the classic limit names no time). */
export const rateLimited = (message: string) =>
  new HttpError(429, { _tag: RATE_LIMITED, message, retryAfter: 0 });

/** The query string of a request, from `query` or the path's own `?…`. */
export function queryOf(request: MockRequest): URLSearchParams {
  const inline = request.path.includes("?") ? request.path.slice(request.path.indexOf("?")) : "";

  if (request.query === undefined) return new URLSearchParams(inline);

  return new URLSearchParams(request.query);
}

/** A header by lowercase name. */
export function headerOf(headers: MockHeaders | undefined, name: string): string | undefined {
  for (const [key, value] of Object.entries(headers ?? {})) {
    if (key.toLowerCase() === name) return value;
  }

  return undefined;
}

/** Runs a handler, answering a thrown `HttpError` as its JSON body. */
export function respond(run: () => MockResponse): MockResponse {
  try {
    return run();
  } catch (error) {
    if (error instanceof HttpError) return { status: error.status, json: { error: error.error } };

    throw error;
  }
}

/** A JSON response. */
export const ok = (json: Json, status = 200): MockResponse => ({ status, json });

/** 204 No Content. */
export const noContent = (): MockResponse => ({ status: 204, json: null });
