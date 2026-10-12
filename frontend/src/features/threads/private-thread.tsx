/** The private-thread mark: a lock wherever a private thread is listed or shown. */
import { useStore } from "../../store/store.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import "./threads.css";

/** Whether the thread is held and private. */
export function usePrivateThread(threadId: number | null): boolean {
  return useStore(
    (state) => threadId !== null && state.threads[threadId]?.visibility === "private",
  );
}

/** The lock before a private thread's name in a list or on a message's reply summary. */
export function PrivateGlyph({ size = 12 }: { readonly size?: number }) {
  return (
    <span className="thread-private-glyph" role="img" aria-label="Private" title="Private thread">
      <Icon name="lock" size={size} />
    </span>
  );
}

/** "Private" beside the room in a private thread's header. */
export function PrivateTag() {
  return (
    <span className="thread-status" data-status="private" title="Private thread">
      <Icon name="lock" size={12} />
      Private
    </span>
  );
}
