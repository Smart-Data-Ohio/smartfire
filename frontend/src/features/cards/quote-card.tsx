import { Link } from "@tanstack/react-router";
import { useCallback } from "react";
import type { QuoteCard as QuoteCardData } from "../../gen/QuoteCard.ts";
import type { QuotePreview } from "../../gen/QuotePreview.ts";
import { formatFull } from "../../lib/time.ts";
import { quoteKey } from "../../store/cards.ts";
import type { MessageDTO } from "../../store/model.ts";
import { actions } from "../../sync/runtime.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { Skeleton } from "../../ui/skeleton.tsx";
import { useNow } from "../threads/use-now.ts";
import { CardAvatar } from "./card-avatar.tsx";
import { ago } from "./format.ts";
import { usePreview } from "./use-preview.ts";

/** The quoted message: who, where and when, the start of what they said; opens the source. */
function Quoted({ preview }: { readonly preview: QuotePreview }) {
  const now = useNow();

  const body = (
    <>
      <span className="quote-head">
        <CardAvatar
          name={preview.authorName}
          seed={String(preview.creatorId)}
          src={null}
          size={18}
        />
        <span className="quote-author">{preview.authorName}</span>
        <span className="quote-where">
          in {preview.roomLabel.startsWith("a ") ? preview.roomLabel : `#${preview.roomLabel}`}
        </span>
        <time
          className="quote-time"
          dateTime={preview.createdAt}
          title={formatFull(preview.createdAt)}
        >
          {ago(preview.createdAt, now)}
        </time>
      </span>
      <span className="quote-excerpt">{preview.excerpt}</span>
    </>
  );

  return preview.threadId === null ? (
    <Link
      to="/r/$roomId/m/$messageId"
      params={{ roomId: preview.roomId, messageId: preview.messageId }}
      className="card quote-card"
      preload={false}
      aria-label={`Quoted message from ${preview.authorName}`}
    >
      {body}
    </Link>
  ) : (
    <Link
      to="/r/$roomId/t/$threadId"
      params={{ roomId: preview.roomId, threadId: preview.threadId }}
      search={{ m: preview.messageId }}
      className="card quote-card"
      preload={false}
      aria-label={`Quoted message from ${preview.authorName}`}
    >
      {body}
    </Link>
  );
}

/** A quote from another room: fetched for the viewer, nothing at all when they can't see it. */
function Fetched({
  message,
  card,
}: {
  readonly message: MessageDTO;
  readonly card: QuoteCardData;
}) {
  const load = useCallback(
    () => actions.cards.loadQuote(message.roomId, card.referenceId),
    [message.roomId, card.referenceId],
  );

  const preview = usePreview("quotes", quoteKey(message.roomId, card.referenceId), load);
  const value = preview?.value ?? null;

  if (value === null) {
    return preview?.status === "error" ? null : (
      <div className="card quote-card" aria-busy="true">
        <span className="quote-head">
          <Icon name="quote" size={14} />
          <Skeleton width={140} height={10} />
        </span>
        <Skeleton height={12} />
      </div>
    );
  }

  return value.state === "loaded" ? <Quoted preview={value} /> : null;
}

/**
 * A message this one links to, quoted: the author, room and time with an excerpt; clicking opens
 * the source message. A same-room quote comes with the message; any other is fetched per viewer.
 */
export function QuoteCard({
  message,
  card,
}: {
  readonly message: MessageDTO;
  readonly card: QuoteCardData;
}) {
  return card.preview === null ? (
    <Fetched message={message} card={card} />
  ) : (
    <Quoted preview={card.preview} />
  );
}
