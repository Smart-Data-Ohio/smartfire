/**
 * The welcome card and server guide as this viewer sees them, read once the shell wants them and
 * again whenever they may have changed: an administrator's save (`workspace.updated`), a resync,
 * joining a suggested room, or a view that shows them mounting. Until the first answer arrives
 * the status is `loading`; a failed read leaves the last copy (or none) in place.
 */
import { useEffect } from "react";
import { useStore as useZustand } from "zustand";
import { createStore, type StoreApi } from "zustand/vanilla";
import type { WorkspaceWelcome } from "../../gen/WorkspaceWelcome.ts";
import { actions, onWelcomeStale } from "../../sync/runtime.ts";

export interface WelcomeState {
  readonly status: "idle" | "loading" | "ready" | "failed";
  readonly welcome: WorkspaceWelcome | null;
}

export interface WelcomeSource {
  readonly read: () => Promise<WorkspaceWelcome>;
  readonly onStale: (listener: () => void) => () => void;
}

export interface WelcomeStore {
  readonly store: StoreApi<WelcomeState>;
  /** Reads it now; overlapping reads settle in order, so the newest answer wins. */
  readonly refresh: () => Promise<void>;
  /** The first call reads it and starts following changes; later calls just read it again. */
  readonly ensure: () => void;
}

export function createWelcomeStore(source: WelcomeSource): WelcomeStore {
  const store = createStore<WelcomeState>()(() => ({ status: "idle", welcome: null }));
  let latest = 0;
  let following = false;

  const refresh = async () => {
    latest += 1;
    const ticket = latest;

    if (store.getState().status === "idle") {
      store.setState({ status: "loading" });
    }

    try {
      const next = await source.read();

      if (ticket === latest) {
        store.setState({ status: "ready", welcome: next });
      }
    } catch {
      if (ticket === latest && store.getState().welcome === null) {
        store.setState({ status: "failed" });
      }
    }
  };

  const ensure = () => {
    if (!following) {
      following = true;
      source.onStale(() => void refresh());
    }

    void refresh();
  };

  return { store, refresh, ensure };
}

export const welcomeStore = createWelcomeStore({
  read: () => actions.welcome(),
  onStale: onWelcomeStale,
});

/** The viewer's welcome, read again each time a component using it mounts. */
export function useWelcome(): WelcomeState {
  useEffect(() => welcomeStore.ensure(), []);

  return useZustand(welcomeStore.store);
}

/** Whether there is a welcome card to show: a message or a suggested room. */
export function hasWelcomeCard(welcome: WorkspaceWelcome | null): welcome is WorkspaceWelcome {
  return welcome !== null && (welcome.messageHtml !== null || welcome.rooms.length > 0);
}
