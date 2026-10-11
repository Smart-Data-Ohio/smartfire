import { Effect, Schema } from "effect";
import type { SudoGoogleStart } from "../gen/SudoGoogleStart.ts";
import type { SudoSubmission } from "../gen/SudoSubmission.ts";
import { noContent } from "./call.ts";
import { ApiClient, type ApiRequest } from "./client.ts";
import { SudoResponse } from "./schema/sudo.ts";

const decodeSudo = Schema.decodeUnknownEffect(SudoResponse);

/**
 * How a caller takes a `navigate` answer: `ownsNavigation` when it follows the location itself
 * (only while its screen still shows), so the client doesn't leave on a 401 for it.
 */
interface Leaving {
  readonly ownsNavigation?: true;
}

/** `GET /sudo`: only a look, nothing held is let go. The caller follows a `navigate` itself. */
export const readSudo = Effect.fn("readSudo")(function* () {
  const client = yield* ApiClient;

  return yield* client.execute(
    { method: "GET", path: "/sudo", auth: true, ownsNavigation: true },
    decodeSudo,
  );
});

export const submitSudo = Effect.fn("submitSudo")(function* (
  body: SudoSubmission,
  leaving: Leaving = {},
) {
  const client = yield* ApiClient;

  return yield* client.execute(
    { method: "POST", path: "/sudo", body, auth: true, ...leaving },
    decodeSudo,
  );
});

/**
 * `POST /sudo/google`, for the SPA's `attempt` id: the continuation names it once Google confirms.
 * Without one (`/app/sudo/new`, which keeps no writes) the continuation asks only whether it did.
 * The caller follows a `navigate` itself.
 */
export const startSudoGoogle = Effect.fn("startSudoGoogle")(function* (attempt: string | null) {
  const client = yield* ApiClient;
  const body: SudoGoogleStart = attempt === null ? {} : { attempt };

  // Every caller leaves for the answer's location itself (Google, or sign-in), if still there.
  return yield* client.execute(
    { method: "POST", path: "/sudo/google", body, auth: true, ownsNavigation: true },
    decodeSudo,
  );
});

/**
 * `GET /sudo/continue?attempt=`: `confirmed` names `attempt` only when this session's Google
 * confirmation for it finished; it answers the same way again until acknowledged or expired.
 */
export const resumeSudo = Effect.fn("resumeSudo")(function* (
  attempt: string | null,
  leaving: Leaving = {},
) {
  const client = yield* ApiClient;

  const request: ApiRequest =
    attempt === null
      ? { method: "GET", path: "/sudo/continue", auth: true, ...leaving }
      : { method: "GET", path: "/sudo/continue", query: { attempt }, auth: true, ...leaving };

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
