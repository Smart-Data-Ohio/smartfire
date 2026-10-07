import { Context, Data, Effect, Layer, Option, Ref, Schema } from "effect";
import * as FetchHttpClient from "effect/http/FetchHttpClient";
import * as HttpClient from "effect/http/HttpClient";
import * as HttpClientRequest from "effect/http/HttpClientRequest";
import type * as HttpClientResponse from "effect/http/HttpClientResponse";
import {
  ApiErrorResponse,
  type ApiFailure,
  InvalidAuthenticityToken,
  NetworkError,
  ServerError,
  Unauthorized,
} from "./errors.ts";

/** Where the JSON API lives: `/api/v1` on the serving origin (Vite proxies it to Rust in dev). */
export class ApiConfig extends Context.Service<ApiConfig, { readonly baseUrl: string }>()(
  "smartfire/api/ApiConfig",
) {
  static readonly layer = (baseUrl: string) => Layer.succeed(ApiConfig, { baseUrl });
}

/** The URL of an API path such as `/me`. Endpoint functions build on this. */
export const endpointUrl = Effect.fn("endpointUrl")(function* (path: string) {
  const { baseUrl } = yield* ApiConfig;

  return `${baseUrl}${path}`;
});

/**
 * The browser's address bar, behind a service so tests never touch `window.location`. The client
 * uses it to send a signed-out person to sign in and back.
 */
export class Navigation extends Context.Service<
  Navigation,
  {
    /** The current path and query, e.g. `/app/rooms/12?message=4`. */
    readonly location: Effect.Effect<string>;
    /** Leaves the SPA for `url` (a full page load). */
    readonly assign: (url: string) => Effect.Effect<void>;
  }
>()("smartfire/api/Navigation") {
  static readonly layerBrowser = Layer.succeed(Navigation, {
    location: Effect.sync(() => `${window.location.pathname}${window.location.search}`),
    assign: (url: string) => Effect.sync(() => window.location.assign(url)),
  });
}

/** One `/api/v1` request. Bodies and replies are JSON. */
export interface ApiRequest {
  readonly method: "GET" | "POST" | "PATCH" | "PUT" | "DELETE";
  /** Below the API base, e.g. `/rooms/12/messages`. */
  readonly path: string;
  readonly query?: Readonly<Record<string, string>>;
  readonly body?: Schema.Json;
}

/** Validates a success body; a failure becomes a `ServerError`. */
export type ResponseDecoder<A> = (json: Schema.Json) => Effect.Effect<A, Schema.SchemaError>;

/**
 * The `/api/v1` client: JSON in and out, the CSRF token on every write (refreshed and retried once
 * when stale), a redirect to sign in on 401, and typed errors for everything else.
 */
export class ApiClient extends Context.Service<
  ApiClient,
  {
    readonly execute: <A>(
      request: ApiRequest,
      decode: ResponseDecoder<A>,
    ) => Effect.Effect<A, ApiFailure>;
    /** Adopts a CSRF token learned elsewhere (the dev boot endpoint carries one). */
    readonly setCsrfToken: (token: string) => Effect.Effect<void>;
    /**
     * The CSRF token writes carry (fetched first when none is held), for the few requests that
     * must go out as a raw `keepalive` fetch; `null` when it can't be had.
     */
    readonly csrfToken: Effect.Effect<string | null>;
  }
>()("smartfire/api/ApiClient") {
  /** Needs an `HttpClient`, `ApiConfig` and `Navigation`. */
  static readonly layer = Layer.effect(
    ApiClient,
    Effect.gen(function* () {
      const http = yield* HttpClient.HttpClient;
      const { baseUrl } = yield* ApiConfig;
      const navigation = yield* Navigation;
      const token = yield* Ref.make(readCsrfMeta());

      const refreshToken = refreshCsrfToken(http, baseUrl).pipe(
        Effect.tap((fresh) => Ref.set(token, fresh)),
      );

      const attempt = Effect.fnUntraced(function* <A>(
        request: ApiRequest,
        decode: ResponseDecoder<A>,
      ) {
        const isWrite = request.method !== "GET";
        const held = isWrite ? yield* Ref.get(token) : null;
        const csrf = isWrite && held === null ? yield* refreshToken : held;

        const response = yield* http.execute(toHttpRequest(baseUrl, request, csrf)).pipe(
          Effect.catchReason(
            "HttpClientError",
            "TransportError",
            (reason) => Effect.fail(new NetworkError({ message: reason.message })),
            (reason) => Effect.fail(new ServerError({ status: 0, message: reason.message })),
          ),
        );

        if (response.status >= 200 && response.status < 300) {
          return yield* readSuccess(response, decode);
        }

        return yield* readFailure(response, navigation);
      });

      const execute = <A>(request: ApiRequest, decode: ResponseDecoder<A>) =>
        attempt(request, decode).pipe(
          Effect.catchTag("StaleCsrfToken", () =>
            refreshToken.pipe(
              Effect.andThen(attempt(request, decode)),
              Effect.catchTag("StaleCsrfToken", (stale) => Effect.fail(stale.error)),
            ),
          ),
        );

      const csrfToken = Effect.flatMap(Ref.get(token), (held) =>
        held === null ? Effect.orElseSucceed(refreshToken, () => null) : Effect.succeed(held),
      );

      return ApiClient.of({ execute, setCsrfToken: (fresh) => Ref.set(token, fresh), csrfToken });
    }),
  );

  /** The app's client: `fetch` with same-origin credentials, the real address bar. */
  static readonly layerBrowser = (baseUrl: string) =>
    ApiClient.layer.pipe(
      Layer.provide(
        Layer.mergeAll(ApiConfig.layer(baseUrl), Navigation.layerBrowser, sameOriginFetch),
      ),
    );
}

/** `fetch` with the session cookie (same origin only). */
const sameOriginFetch = Layer.effect(
  HttpClient.HttpClient,
  Effect.map(HttpClient.HttpClient, (client) =>
    HttpClient.transform(client, (response) =>
      Effect.provideService(response, FetchHttpClient.RequestInit, {
        credentials: "same-origin",
      }),
    ),
  ),
).pipe(Layer.provide(FetchHttpClient.layer));

/** A 422 that says the CSRF token was stale; `error` is what surfaces if the retry fails too. */
class StaleCsrfToken extends Data.TaggedError("StaleCsrfToken")<{
  readonly error: InvalidAuthenticityToken | ServerError;
}> {}

/** The masked token the Rust shell puts in `<meta name="csrf-token">`; `null` in dev. */
function readCsrfMeta(): string | null {
  if (!("document" in globalThis)) {
    return null;
  }

  return document.querySelector('meta[name="csrf-token"]')?.getAttribute("content") ?? null;
}

function toHttpRequest(
  baseUrl: string,
  request: ApiRequest,
  csrf: string | null,
): HttpClientRequest.HttpClientRequest {
  let built = HttpClientRequest.make(request.method)(`${baseUrl}${request.path}`).pipe(
    HttpClientRequest.acceptJson,
  );

  if (request.query !== undefined) {
    built = HttpClientRequest.setUrlParams(built, request.query);
  }

  if (csrf !== null && request.method !== "GET") {
    built = HttpClientRequest.setHeader(built, "X-CSRF-Token", csrf);
  }

  if (request.body !== undefined) {
    built = HttpClientRequest.bodyJsonUnsafe(built, request.body);
  }

  return built;
}

const CsrfReply = Schema.Struct({ csrfToken: Schema.String });

const decodeCsrfReply = Schema.decodeUnknownEffect(CsrfReply);

/** Fetches a fresh CSRF token from the boot endpoint. */
function refreshCsrfToken(
  http: HttpClient.HttpClient,
  baseUrl: string,
): Effect.Effect<string, NetworkError | ServerError> {
  return http.get(`${baseUrl}/boot`, { acceptJson: true }).pipe(
    Effect.flatMap((response) => response.json),
    Effect.flatMap(decodeCsrfReply),
    Effect.map((reply) => reply.csrfToken),
    Effect.catchTag("HttpClientError", (error) =>
      Effect.fail(new NetworkError({ message: error.message })),
    ),
    Effect.catchTag("SchemaError", (error) =>
      Effect.fail(new ServerError({ status: 200, message: error.message })),
    ),
  );
}

function readSuccess<A>(
  response: HttpClientResponse.HttpClientResponse,
  decode: ResponseDecoder<A>,
): Effect.Effect<A, ServerError> {
  const fail = (message: string) =>
    Effect.fail(new ServerError({ status: response.status, message }));

  return response.text.pipe(
    Effect.flatMap((text) => (text === "" ? Effect.succeed(null) : decodeJsonText(text))),
    Effect.flatMap(decode),
    Effect.catchTag("HttpClientError", (error) => fail(error.message)),
    Effect.catchTag("SchemaError", (error) => fail(error.message)),
  );
}

const decodeJsonText = Schema.decodeUnknownEffect(Schema.fromJsonString(Schema.Json));

const decodeErrorBody = Schema.decodeUnknownEffect(Schema.fromJsonString(ApiErrorResponse));

/** Turns a non-2xx response into its typed error; 401 also leaves for the sign-in page. */
function readFailure(
  response: HttpClientResponse.HttpClientResponse,
  navigation: Navigation["Service"],
): Effect.Effect<never, ApiFailure | StaleCsrfToken> {
  return Effect.gen(function* () {
    const text = yield* response.text.pipe(Effect.orElseSucceed(() => ""));

    const decoded = yield* decodeErrorBody(text).pipe(
      Effect.map((body) => body.error),
      Effect.option,
    );

    const fallback = new ServerError({
      status: response.status,
      message: `The server answered ${response.status}`,
    });

    if (response.status === 401) {
      const location = yield* navigation.location;

      yield* navigation.assign(`/session/new?return_to=${encodeURIComponent(location)}`);

      const error = Option.getOrUndefined(decoded);

      return yield* error instanceof Unauthorized
        ? error
        : new Unauthorized({ message: "Sign in to continue" });
    }

    if (response.status === 422) {
      // A 422 without a JSON error body is the CSRF check's bare rejection.
      if (Option.isNone(decoded)) {
        return yield* new StaleCsrfToken({ error: fallback });
      }

      if (decoded.value instanceof InvalidAuthenticityToken) {
        return yield* new StaleCsrfToken({ error: decoded.value });
      }
    }

    return yield* Option.getOrElse(decoded, () => fallback);
  });
}
