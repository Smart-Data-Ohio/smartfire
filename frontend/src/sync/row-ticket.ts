/**
 * Sidebar row tickets as Effect resources: a request whose reply installs or removes sidebar rows
 * holds one from before the request until the reply has landed, failed or been interrupted, so
 * the store keeps the sync touches that reply must not overwrite, and no longer.
 */
import { Effect, type Scope } from "effect";
import { mutations } from "../store/store.ts";

/** A ticket held until the enclosing scope closes. */
export const rowTicket: Effect.Effect<number, never, Scope.Scope> = Effect.acquireRelease(
  Effect.sync(() => mutations.openRowTicket()),
  (since) => Effect.sync(() => mutations.closeRowTicket(since)),
);

/** Runs `use` holding a ticket, released however it ends. */
export const withRowTicket = <A, E, R>(
  use: (since: number) => Effect.Effect<A, E, R>,
): Effect.Effect<A, E, Exclude<R, Scope.Scope>> => Effect.scoped(Effect.flatMap(rowTicket, use));
