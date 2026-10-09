/**
 * The product tour's state, apart from its view: whether it shows, which step, and the one stamp
 * each run ends with. Skipping and finishing both stamp (classic's users/tours#update is shared),
 * and a run stamps once however it ends. It starts by itself once per page load for someone who
 * hasn't completed it, as classic's layout does, and the help menu restarts it on demand without
 * clearing the stamp.
 */
import { useStore as useZustand } from "zustand";
import { createStore, type StoreApi } from "zustand/vanilla";

export interface TourState {
  readonly open: boolean;
  readonly index: number;
}

export interface Tour {
  readonly store: StoreApi<TourState>;
  /** Opens at the first step (a restart while open goes back to it). */
  readonly start: () => void;
  /** The next step, or on the last one, finishing. */
  readonly next: () => void;
  readonly back: () => void;
  readonly skip: () => void;
  readonly finish: () => void;
  /**
   * Starts the tour when `tourCompleted` is `false` (known and not done), the first time only:
   * later calls in the same page load do nothing, so finishing it can't bring it back. Answers
   * whether it started.
   */
  readonly autoStart: (tourCompleted: boolean | null) => boolean;
}

export interface TourOptions {
  readonly steps: number;
  /** Records the tour as completed (best effort, as classic's fetch is). */
  readonly stamp: () => void;
}

export function createTour({ steps, stamp }: TourOptions): Tour {
  const store = createStore<TourState>()(() => ({ open: false, index: 0 }));
  let autoStarted = false;

  const start = () => store.setState({ open: true, index: 0 });

  const complete = () => {
    if (!store.getState().open) {
      return;
    }

    store.setState({ open: false });
    stamp();
  };

  const next = () => {
    const { open, index } = store.getState();

    if (!open) {
      return;
    }

    if (index >= steps - 1) {
      complete();
    } else {
      store.setState({ index: index + 1 });
    }
  };

  const back = () => {
    const { open, index } = store.getState();

    if (open && index > 0) {
      store.setState({ index: index - 1 });
    }
  };

  const autoStart = (tourCompleted: boolean | null) => {
    if (autoStarted || tourCompleted !== false) {
      return false;
    }

    autoStarted = true;
    start();

    return true;
  };

  return { store, start, next, back, skip: complete, finish: complete, autoStart };
}

export function useTourState<T>(tour: Tour, select: (state: TourState) => T): T {
  return useZustand(tour.store, select);
}
