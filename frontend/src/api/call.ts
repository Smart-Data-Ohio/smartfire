import { Effect } from "effect";
import { ApiClient, type ApiRequest, type ResponseDecoder } from "./client.ts";

/** Runs one `/api/v1` request on the app's client. */
export const call = <A>(request: ApiRequest, decode: ResponseDecoder<A>) =>
  ApiClient.use((client) => client.execute(request, decode));

/** A `GET` with an optional query. */
export const get = (path: string, query?: Readonly<Record<string, string>>): ApiRequest =>
  query === undefined ? { method: "GET", path } : { method: "GET", path, query };

/** For 204 replies: the empty body is accepted and nothing comes back. */
export const noContent: ResponseDecoder<void> = () => Effect.void;
