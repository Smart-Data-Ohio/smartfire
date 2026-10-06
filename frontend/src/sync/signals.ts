/**
 * Applied sync events, for features that react to an event rather than (or as well as) store it:
 * an incoming call rings, a call notice toasts, a role change rejoins. The engine hands each
 * applied batch here after its store commit, in order. Plain TypeScript; listeners must not throw.
 */
import type { SyncEvent } from "../gen/SyncEvent.ts";

type Listener = (events: readonly SyncEvent[]) => void;

const listeners = new Set<Listener>();

/** Calls `listener` with every batch the engine applies from now on; returns the unsubscribe. */
export function onSyncEvents(listener: Listener): () => void {
  listeners.add(listener);

  return () => {
    listeners.delete(listener);
  };
}

/** The engine's side: one applied batch. */
export function emitSyncEvents(events: readonly SyncEvent[]): void {
  for (const listener of listeners) {
    listener(events);
  }
}
