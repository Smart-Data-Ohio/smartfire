import { Context, type Effect } from "effect";
import type { ApiRequest } from "./client.ts";
import type { ConfirmationCancelled, SudoRequired } from "./errors.ts";

/**
 * Where a write that answered `SudoRequired` waits for the person to confirm it's them. The client
 * replays the write once after `confirm` succeeds. Without this service (tests, the signed-out
 * boot) the `SudoRequired` failure reaches the caller unchanged.
 */
export class Reauthentication extends Context.Service<
  Reauthentication,
  {
    /**
     * How many confirmations have finished so far. The client reads it before sending a write, so
     * a refusal that crossed a confirmation in flight is replayed without asking again.
     */
    readonly confirmations: Effect.Effect<number>;
    /**
     * Waits for the confirmation `required` asks for; concurrent writes share one. Fails with
     * `ConfirmationCancelled` when the person closes it, and with `required` itself when they
     * leave to confirm with Google and `request` can't be kept for the return.
     */
    readonly confirm: (
      required: SudoRequired,
      request: ApiRequest,
      sentAfter: number,
    ) => Effect.Effect<void, SudoRequired | ConfirmationCancelled>;
  }
>()("smartfire/api/Reauthentication") {}
