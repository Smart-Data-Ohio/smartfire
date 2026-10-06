import { Context, Effect, Layer } from "effect";

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
