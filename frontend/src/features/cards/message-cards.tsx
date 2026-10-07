import { visibleCards } from "../../store/cards.ts";
import type { MessageDTO } from "../../store/model.ts";
import { CardBoundary } from "./card-boundary.tsx";
import { EventCard } from "./event-card.tsx";
import { FizzyCard } from "./fizzy-card.tsx";
import { GithubCard } from "./github-card.tsx";
import { DriveChip, LinkCard, LinkedinCard } from "./link-cards.tsx";
import { PollCard } from "./poll-card.tsx";
import { QuoteCard } from "./quote-card.tsx";
import { XCard } from "./x-card.tsx";
import "./cards.css";

type Card = ReturnType<typeof visibleCards>[number];

function CardView({
  message,
  card,
  threadId,
}: {
  readonly message: MessageDTO;
  readonly card: Card;
  readonly threadId: number | null;
}) {
  switch (card.kind) {
    case "drive":
      return <DriveChip card={card.data} />;
    case "github":
      return <GithubCard message={message} card={card.data} threadId={threadId} />;
    case "x":
      return <XCard card={card.data} />;
    case "event":
      return <EventCard event={card.data} />;
    case "fizzy":
      return <FizzyCard message={message} card={card.data} />;
    case "quote":
      return <QuoteCard message={message} card={card.data} />;
    case "linkedin":
      return <LinkedinCard card={card.data} />;
    case "link":
      return <LinkCard card={card.data} />;
  }
}

/**
 * Everything under a message's body in the classic slot order: its poll, then its cards (Drive
 * files, pull requests, posts, events, Fizzy cards, quotes, LinkedIn posts, other pages). A kind
 * this app doesn't know, or a card it can't read, is skipped on its own. `threadId` is the thread
 * pane's thread when this message heads it (a pull request's discussion lists its files there).
 */
export default function MessageCards({
  message,
  threadId,
}: {
  readonly message: MessageDTO;
  readonly threadId: number | null;
}) {
  const cards = visibleCards(message);

  if (message.poll === null && cards.length === 0) {
    return null;
  }

  return (
    <div className="message-cards">
      {message.poll === null ? null : (
        <CardBoundary>
          <PollCard message={message} poll={message.poll} />
        </CardBoundary>
      )}
      {cards.map((card, index) => (
        // Cards have no id of their own; their place in the slot order is stable per message.
        // biome-ignore lint/suspicious/noArrayIndexKey: the server's slot order is the identity
        <CardBoundary key={`${index}:${card.kind}`}>
          <CardView message={message} card={card} threadId={threadId} />
        </CardBoundary>
      ))}
    </div>
  );
}
