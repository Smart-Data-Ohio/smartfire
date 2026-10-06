import { useStore as useZustand } from "zustand";
import { createStore } from "zustand/vanilla";

interface EditingState {
  /** The message being edited in place, or `null`. Only one edits at a time. */
  readonly messageId: number | null;
}

const editingStore = createStore<EditingState>()(() => ({ messageId: null }));

/**
 * Which message is open for an in-place edit. The hover bar, the context menu, the `E` key and
 * the composer's ↑-to-edit-last all start editing through here; the row renders the editor.
 */
export function useEditingId(): number | null {
  return useZustand(editingStore, (state) => state.messageId);
}

export function startEditing(messageId: number): void {
  editingStore.setState({ messageId });
}

export function stopEditing(): void {
  editingStore.setState({ messageId: null });
}
