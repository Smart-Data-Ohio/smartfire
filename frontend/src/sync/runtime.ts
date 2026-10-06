import { ManagedRuntime } from "effect";
import { ApiConfig, endpointUrl } from "../api/client.ts";

/**
 * The one Effect runtime the app shares. Code outside src/api and src/sync never imports
 * `effect`: it calls `actions`, plain async functions that run Effect programs here.
 */
export const runtime = ManagedRuntime.make(ApiConfig.layer("/api/v1"));

export const actions = {
  endpointUrl: (path: string): Promise<string> => runtime.runPromise(endpointUrl(path)),
};
