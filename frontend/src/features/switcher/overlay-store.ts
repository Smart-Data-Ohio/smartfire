/**
 * Which app-wide overlay is open: the quick switcher, the shortcuts dialog, the new-DM picker or
 * the create-a-room dialog. One at a time; any button in the app can open one (the sidebar's "+", the user panel's
 * keyboard button) and `GlobalOverlays` renders it.
 */
import { useStore as useZustand } from "zustand";
import { createStore } from "zustand/vanilla";

export type Overlay = "switcher" | "shortcuts" | "new-direct" | "new-room";

interface OverlayState {
  readonly open: Overlay | null;
}

export const overlayStore = createStore<OverlayState>()(() => ({ open: null }));

export function useOverlay(): Overlay | null {
  return useZustand(overlayStore, (state) => state.open);
}

export function openOverlay(overlay: Overlay): void {
  overlayStore.setState({ open: overlay });
}

/** Closes `overlay` if it is the open one (a late close can't shut its successor). */
export function closeOverlay(overlay: Overlay): void {
  if (overlayStore.getState().open === overlay) {
    overlayStore.setState({ open: null });
  }
}

/** The shortcut's toggle: open it, or close it when it is already open. */
export function toggleOverlay(overlay: Overlay): void {
  overlayStore.setState((state) => ({ open: state.open === overlay ? null : overlay }));
}

/** The overlays' chunks, shared by `React.lazy` and the idle preload. */
export const overlayChunks = {
  switcher: () => import("./switcher-dialog.tsx"),
  shortcuts: () => import("./shortcuts-dialog.tsx"),
  "new-direct": () => import("../directs/new-direct-dialog.tsx"),
  "new-room": () => import("../rooms/new-room-dialog.tsx"),
} as const;
