import { Effect, Schema } from "effect";
import type { SudoGoogleStart } from "../gen/SudoGoogleStart.ts";
import type { SudoSubmission } from "../gen/SudoSubmission.ts";
import { noContent } from "./call.ts";
import { ApiClient, type ApiRequest } from "./client.ts";
import { SudoResponse } from "./schema/sudo.ts";

const decodeSudo = Schema.decodeUnknownEffect(SudoResponse);

export const readSudo = Effect.fn("readSudo")(function* () {
  const client = yield* ApiClient;

  return yield* client.execute({ method: "GET", path: "/sudo", auth: true }, decodeSudo);
});

export const submitSudo = Effect.fn("submitSudo")(function* (body: SudoSubmission) {
  const client = yield* ApiClient;

  return yield* client.execute({ method: "POST", path: "/sudo", body, auth: true }, decodeSudo);
});

/** `POST /sudo/google`, for the SPA's `attempt` id: the continuation names it once Google confirms. */
export const startSudoGoogle = Effect.fn("startSudoGoogle")(function* (attempt: string) {
  const client = yield* ApiClient;
  const body: SudoGoogleStart = { attempt };

  return yield* client.execute(
    { method: "POST", path: "/sudo/google", body, auth: true },
    decodeSudo,
  );
});

/**
 * `GET /sudo/continue?attempt=`: `confirmed` names `attempt` only when this session's Google
 * confirmation for it finished; it answers the same way again until acknowledged or expired.
 */
export const resumeSudo = Effect.fn("resumeSudo")(function* (attempt: string | null) {
  const client = yield* ApiClient;

  const request: ApiRequest =
    attempt === null
      ? { method: "GET", path: "/sudo/continue", auth: true }
      : { method: "GET", path: "/sudo/continue", query: { attempt }, auth: true };

  return yield* client.execute(request, decodeSudo);
});

/** `DELETE /sudo/continue?attempt=` (204): the SPA is done with what `attempt` confirmed. */
export const acknowledgeSudo = Effect.fn("acknowledgeSudo")(function* (attempt: string) {
  const client = yield* ApiClient;

  return yield* client.execute(
    { method: "DELETE", path: "/sudo/continue", query: { attempt }, auth: true },
    noContent,
  );
});
