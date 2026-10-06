import { useStore as useZustand } from "zustand";
import { createStore } from "zustand/vanilla";

/** The right pane's non-thread views. An open thread is in the URL (`/r/$roomId/t/$threadId`). */
export type PaneKind = "members" | "pins" | "files" | "threads";

interface PaneState {
  readonly open: PaneKind | null;
}

const paneStore = createStore<PaneState>()(() => ({ open: null }));

/** Which side pane is open beside the conversation; per tab, not worth a URL. */
export function useOpenPane(): PaneKind | null {
  return useZustand(paneStore, (state) => state.open);
}

export function openPane(open: PaneKind | null): void {
  paneStore.setState({ open });
}

/** Header buttons toggle their pane. */
export function togglePane(kind: PaneKind): void {
  paneStore.setState((state) => ({ open: state.open === kind ? null : kind }));
}
