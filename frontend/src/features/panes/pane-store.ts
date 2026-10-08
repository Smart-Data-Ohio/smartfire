import { useStore as useZustand } from "zustand";
import { createStore } from "zustand/vanilla";

/** The right pane's non-thread views. An open thread is in the URL (`/r/$roomId/t/$threadId`). */
export type PaneKind = "members" | "pins" | "files" | "threads" | "stage" | "automations";

/** The existing side panes that also have classic page mappings. */
export type RoutePaneKind = Extract<PaneKind, "threads" | "files" | "pins" | "automations">;

type OpenPane =
  | { readonly kind: "local"; readonly pane: PaneKind }
  | { readonly kind: "route"; readonly pane: RoutePaneKind; readonly roomId: number };

interface PaneState {
  readonly open: OpenPane | null;
}

const paneStore = createStore<PaneState>()(() => ({ open: null }));

/** The local side pane, or a routed list remembered below a thread in this room. */
export function useOpenPane(roomId: number): PaneKind | null {
  return useZustand(paneStore, (state) => {
    const open = state.open;

    return open === null || (open.kind === "route" && open.roomId !== roomId) ? null : open.pane;
  });
}

export function openPane(open: PaneKind | null): void {
  paneStore.setState({ open: open === null ? null : { kind: "local", pane: open } });
}

/** Remembers the URL list under a thread, so Back returns to the list's URL too. */
export function openRoutePane(roomId: number, pane: RoutePaneKind): void {
  paneStore.setState({ open: { kind: "route", pane, roomId } });
}

export function useRoutePaneReturn(roomId: number): RoutePaneKind | null {
  return useZustand(paneStore, (state) =>
    state.open?.kind === "route" && state.open.roomId === roomId ? state.open.pane : null,
  );
}

/** Forgets a routed return destination; room cleanup only forgets its own list. */
export function clearRoutePane(roomId?: number): void {
  const open = paneStore.getState().open;

  if (open?.kind === "route" && (roomId === undefined || open.roomId === roomId)) {
    paneStore.setState({ open: null });
  }
}

/** Header buttons toggle their pane. */
export function togglePane(kind: PaneKind): void {
  paneStore.setState((state) => ({
    open: state.open?.pane === kind ? null : { kind: "local", pane: kind },
  }));
}
