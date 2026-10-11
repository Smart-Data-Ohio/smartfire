import { Context, type Effect } from "effect";
import type { ApiRequest } from "./client.ts";
import type { ConfirmationCancelled, SudoRequired } from "./errors.ts";

/** What the client notes about a write as it sends it, for the confirmation it may need. */
export interface ConfirmationTicket {
  /** How many confirmations had finished. */
  readonly confirmations: number;
  /** The screen that sent it: leaving that screen cancels its wait for a confirmation. */
  readonly screen: string;
}

/**
 * Where a write that answered `SudoRequired` waits for the person to confirm it's them. The client
 * replays the write once after `confirm` succeeds. Without this service (tests, the signed-out
 * boot) the `SudoRequired` failure reaches the caller unchanged.
 */
export class Reauthentication extends Context.Service<
  Reauthentication,
  {
    /**
     * Read before sending a write: how many confirmations have finished (so a refusal that crossed
     * one in flight is replayed without asking again) and the screen sending it.
     */
    readonly ticket: Effect.Effect<ConfirmationTicket>;
    /**
     * Waits for the confirmation `required` asks for; concurrent writes share one. Fails with
     * `ConfirmationCancelled` when the person closes it or leaves the screen that sent the write,
     * and with `required` itself when they leave to confirm with Google and `request` can't be
     * kept for the return.
     */
    readonly confirm: (
      required: SudoRequired,
      request: ApiRequest,
      ticket: ConfirmationTicket,
    ) => Effect.Effect<void, SudoRequired | ConfirmationCancelled>;
  }
>()("smartfire/api/Reauthentication") {}
