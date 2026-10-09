import { type ComponentType, useSyncExternalStore } from "react";
import { loadForUpdate } from "../../service-worker/update-required.ts";
import type { MessageDTO } from "../../store/model.ts";

interface MessageCardsProps {
  readonly message: MessageDTO;
  readonly threadId: number | null;
}

/** The cards chunk once it has arrived; set once and never replaced. */
let loaded: ComponentType<MessageCardsProps> | null = null;

let fetching = false;

/** The last fetch failed; nothing waits on the chunk until a slot asks for it again. */
let failed = false;

const listeners = new Set<() => void>();

/** Fetches the chunk (unless it's here or on its way) and tells every waiting slot when it lands. */
function fetchChunk(): void {
  if (loaded !== null || fetching) {
    return;
  }

  fetching = true;
  loadForUpdate(() => import("./message-cards.tsx")).then(
    (module) => {
      loaded = module.default;
      fetching = false;
      notifyAll();
    },
    () => {
      // Asked for again when the next slot that needs it mounts.
      fetching = false;
      failed = true;
      notifyAll();
    },
  );
}

function notifyAll(): void {
  for (const notify of listeners) {
    notify();
  }
}

// Fetch the chunk as soon as the app starts (alongside the boot requests), so a room's first rows
// render with their cards rather than growing a moment later, which would push a permalinked
// row off centre. A timeline still waits for it (`useCardsChunkSettled`) in case it's slower.
fetchChunk();

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  fetchChunk();

  return () => {
    listeners.delete(listener);
  };
}

const snapshot = () => loaded;

const settledSnapshot = () => loaded !== null || failed;

const loadedSnapshot = () => loaded !== null;

/** Whether a message shows anything from the cards chunk: a poll or a card. */
export function hasCards(message: MessageDTO): boolean {
  return message.poll !== null || message.cards.length > 0;
}

/**
 * Whether the cards chunk has arrived (or failed to). A timeline waits on it before placing a
 * window that holds cards: the chunk is fetched at boot, but a slow fetch can still land after
 * the first rows, and the cards growing in then push a permalinked row out of view.
 */
export function useCardsChunkSettled(): boolean {
  return useSyncExternalStore(subscribe, settledSnapshot, settledSnapshot);
}

/** A successful reveal, including a retry after the initial fetch failed. */
export function useCardsChunkLoaded(): boolean {
  return useSyncExternalStore(subscribe, loadedSnapshot, loadedSnapshot);
}

/**
 * The cards once their chunk is here, nothing until then. The chunk is read through
 * `useSyncExternalStore`, so the component type only ever goes from nothing to the loaded one:
 * the cards never remount (and lose a poll's Change vote or ticks) when the chunk lands, and
 * they appear in the same commit as it, with no Suspense reveal delay to push rows around.
 */
function LoadedCards({ message, threadId }: MessageCardsProps) {
  const Cards = useSyncExternalStore(subscribe, snapshot, snapshot);

  return Cards === null ? null : <Cards message={message} threadId={threadId} />;
}

/**
 * The poll and cards under a message, from their own chunk (fetched at boot, or again the first
 * time a message has any if that failed). Most messages have none and render nothing here.
 */
export function CardSlot({ message, threadId }: MessageCardsProps) {
  if (!hasCards(message)) {
    return null;
  }

  return <LoadedCards message={message} threadId={threadId} />;
}
