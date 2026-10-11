/**
 * Whether this person put the welcome card away. The saved answer lives in their appearance
 * preferences, which take a frame or more to show a change (a view transition, then the save).
 * A change made here shows at once instead, so a view acting on it (Home after "Show welcome")
 * never sees the old answer; it gives way to the saved preference once that agrees, and a refused
 * save takes it back.
 */
import { useStore as useZustand } from "zustand";
import { createStore, type StoreApi } from "zustand/vanilla";
import { appearanceSnapshot, onAppearanceChange, useAppearance } from "../../lib/appearance.ts";
import { saveAccountPersonalAppearance } from "../../sync/settings.ts";

export interface DismissalState {
  /** The change being saved, shown in place of the preference; `null` when there's none. */
  readonly pending: boolean | null;
}

export interface DismissalSource {
  readonly save: (dismissed: boolean) => Promise<void>;
  /** The saved preference as currently shown. */
  readonly saved: () => boolean;
  readonly onSavedChange: (listener: () => void) => () => void;
}

export interface Dismissal {
  readonly store: StoreApi<DismissalState>;
  /** Shows `dismissed` at once, then saves it; rejects (showing the old answer) if refused. */
  readonly set: (dismissed: boolean) => Promise<void>;
}

export function createDismissal(source: DismissalSource): Dismissal {
  const store = createStore<DismissalState>()(() => ({ pending: null }));
  let latest = 0;
  let stopWaiting: (() => void) | null = null;

  const settle = (ticket: number) => {
    if (ticket === latest) {
      stopWaiting?.();
      stopWaiting = null;
      store.setState({ pending: null });
    }
  };

  const set = async (dismissed: boolean) => {
    latest += 1;
    const ticket = latest;

    stopWaiting?.();
    stopWaiting = null;
    store.setState({ pending: dismissed });

    try {
      await source.save(dismissed);
    } catch (error) {
      settle(ticket);

      throw error;
    }

    if (ticket !== latest) {
      return;
    }

    if (source.saved() === dismissed) {
      settle(ticket);

      return;
    }

    // Saved, but not on screen yet: hold the change until the preference shows it.
    stopWaiting = source.onSavedChange(() => {
      if (source.saved() === dismissed) {
        settle(ticket);
      }
    });
  };

  return { store, set };
}

export const welcomeDismissal = createDismissal({
  save: (dismissed) => saveAccountPersonalAppearance({ welcome: dismissed ? "dismissed" : null }),
  saved: () => appearanceSnapshot().welcomeDismissed,
  onSavedChange: onAppearanceChange,
});

/** Whether the welcome card is put away: a change being saved, else the saved preference. */
export function useWelcomeDismissed(): boolean {
  const { welcomeDismissed } = useAppearance();
  const { pending } = useZustand(welcomeDismissal.store);

  return pending ?? welcomeDismissed;
}
