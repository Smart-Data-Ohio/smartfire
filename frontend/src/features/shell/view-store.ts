import { useStore as useZustand } from "zustand";
import { createStore } from "zustand/vanilla";

/** Which destination the rail has selected: everything, or direct messages only. */
export type Destination = "home" | "dms";

interface ViewState {
  readonly destination: Destination;
}

const viewStore = createStore<ViewState>()(() => ({ destination: "home" }));

/** Local, per-tab view choices that aren't data and aren't worth a URL. */
export function useDestination(): Destination {
  return useZustand(viewStore, (state) => state.destination);
}

export function setDestination(destination: Destination): void {
  viewStore.setState({ destination });
}
