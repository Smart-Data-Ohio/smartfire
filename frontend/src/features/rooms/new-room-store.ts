/**
 * Which kind of room the create dialog opens on: the "+" by Voice opens it on a voice channel,
 * the PWA shortcut and the classic `rooms/<kind>/new` links on theirs.
 */
import { createStore } from "zustand/vanilla";
import { openOverlay } from "../switcher/overlay-store.ts";
import type { ManagedKind } from "./room-forms.ts";

export const newRoomPreset = createStore<{ readonly kind: ManagedKind }>()(() => ({
  kind: "open",
}));

/** Opens the create-a-room dialog on `kind` (a public text channel unless said otherwise). */
export function openNewRoom(kind: ManagedKind = "open"): void {
  newRoomPreset.setState({ kind });
  openOverlay("new-room");
}
