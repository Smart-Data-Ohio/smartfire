import { type ComponentType, lazy, Suspense } from "react";
import type { MessageDTO } from "../../store/model.ts";

interface MessageCardsProps {
  readonly message: MessageDTO;
  readonly threadId: number | null;
}

const load = () => import("./message-cards.tsx");

/** The cards chunk once it has arrived: rows then render it at once, without suspending. */
interface Chunk {
  Cards: ComponentType<MessageCardsProps> | null;
}

const chunk: Chunk = { Cards: null };

const MessageCards = lazy(() =>
  load().then((module) => {
    chunk.Cards = module.default;

    return module;
  }),
);

// Fetch the chunk as soon as the app starts (alongside the boot requests), so a room's first rows
// render with their cards rather than growing a moment later, which would push a permalinked
// row off centre. If it fails, the lazy component loads it again when a card first needs it.
load().then(
  (module) => {
    chunk.Cards = module.default;
  },
  () => undefined,
);

/**
 * The poll and cards under a message, from their own chunk (fetched at idle after boot, or the
 * first time a message has any). Most messages have none and render nothing here.
 */
export function CardSlot({ message, threadId }: MessageCardsProps) {
  if (message.poll === null && message.cards.length === 0) {
    return null;
  }

  if (chunk.Cards !== null) {
    return <chunk.Cards message={message} threadId={threadId} />;
  }

  return (
    <Suspense fallback={null}>
      <MessageCards message={message} threadId={threadId} />
    </Suspense>
  );
}
