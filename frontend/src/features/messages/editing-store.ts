import { useStore as useZustand } from "zustand";
import { createStore } from "zustand/vanilla";

interface EditingState {
  /** The message being edited in place, or `null`. Only one edits at a time. */
  readonly messageId: number | null;
  /** Where focus goes when the edit ends (the composer that asked); `null` for the row. */
  readonly returnFocusTo: HTMLElement | null;
}

const editingStore = createStore<EditingState>()(() => ({ messageId: null, returnFocusTo: null }));

/**
 * Which message is open for an in-place edit. The hover bar, the context menu, the `E` key and
 * the composer's ↑-to-edit-last all start editing through here; the row renders the editor.
 */
export function useEditingId(): number | null {
  return useZustand(editingStore, (state) => state.messageId);
}

/** Opens `messageId` for editing; `returnFocusTo` gets focus back when it closes. */
export function startEditing(messageId: number, returnFocusTo: HTMLElement | null = null): void {
  editingStore.setState({ messageId, returnFocusTo });
}

/** Closes the editor; answers where focus should go (or `null` for the row's own choice). */
export function stopEditing(): HTMLElement | null {
  const target = editingStore.getState().returnFocusTo;

  editingStore.setState({ messageId: null, returnFocusTo: null });

  return target?.isConnected === true ? target : null;
}

/** The message open for editing right now (outside React). */
export function editingId(): number | null {
  return editingStore.getState().messageId;
}
