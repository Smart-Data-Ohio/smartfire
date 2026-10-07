import { Effect } from "effect";

/** A failed action, with a message fit to show (the API error's own message). */
export class ActionError extends Error {
  /** The API error's tag (`Forbidden`, `Validation`, `NetworkError`…), for callers that care. */
  readonly tag: string;

  /** A `Validation` error's messages by wire field (`currentPassword`); empty otherwise. */
  readonly fields: Readonly<Record<string, readonly string[]>>;

  constructor(
    tag: string,
    message: string,
    fields: Readonly<Record<string, readonly string[]>> = {},
  ) {
    super(message);
    this.name = "ActionError";
    this.tag = tag;
    this.fields = fields;
  }
}

/** What an action can fail with: an API error (a `Validation` one carries `fields`) or the like. */
export interface ActionFailure {
  readonly _tag: string;
  readonly message: string;
  readonly fields?: Readonly<Record<string, readonly string[]>>;
}

/** Failures become `ActionError`s, so React code can show `error.message` without Effect. */
export const asAction = <A, E extends ActionFailure, R>(
  effect: Effect.Effect<A, E, R>,
): Effect.Effect<A, ActionError, R> =>
  Effect.mapError(effect, (error) => new ActionError(error._tag, error.message, error.fields));
