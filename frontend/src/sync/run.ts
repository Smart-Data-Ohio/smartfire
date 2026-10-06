import { Effect } from "effect";

/** A failed action, with a message fit to show (the API error's own message). */
export class ActionError extends Error {
  /** The API error's tag (`Forbidden`, `Validation`, `NetworkError`…), for callers that care. */
  readonly tag: string;

  constructor(tag: string, message: string) {
    super(message);
    this.name = "ActionError";
    this.tag = tag;
  }
}

/** Failures become `ActionError`s, so React code can show `error.message` without Effect. */
export const asAction = <A, E extends { readonly _tag: string; readonly message: string }, R>(
  effect: Effect.Effect<A, E, R>,
): Effect.Effect<A, ActionError, R> =>
  Effect.mapError(effect, (error) => new ActionError(error._tag, error.message));
