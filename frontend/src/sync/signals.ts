/**
 * Applied sync events, for features that react to an event rather than (or as well as) store it:
 * an incoming call rings, a call notice toasts, a role change rejoins. The engine hands each
 * applied batch here after its store commit, in order. Plain TypeScript; listeners must not throw.
 */
import type { SyncEvent } from "../gen/SyncEvent.ts";

type Listener = (events: readonly SyncEvent[]) => void;

type ResyncListener = (topics: readonly string[]) => void;

const listeners = new Set<Listener>();

const resyncListeners = new Set<ResyncListener>();

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

/**
 * Calls `listener` with the topics of every resync from now on: those the server couldn't replay
 * (an expired replay, a server restart, a lagging socket), whose events were lost. The engine
 * refetches what the store holds for them; a feature that reads its own data, such as a calendar
 * screen, reads it again here. Returns the unsubscribe.
 */
export function onResync(listener: ResyncListener): () => void {
  resyncListeners.add(listener);

  return () => {
    resyncListeners.delete(listener);
  };
}

/** The engine's side: a resync of `topics` begins. */
export function emitResync(topics: readonly string[]): void {
  for (const listener of resyncListeners) {
    listener(topics);
  }
}
