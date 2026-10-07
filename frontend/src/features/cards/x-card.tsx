import type { XMedia } from "../../gen/XMedia.ts";
import type { XPostCard } from "../../gen/XPostCard.ts";
import type { XQuote } from "../../gen/XQuote.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { Skeleton } from "../../ui/skeleton.tsx";
import { BrandMark } from "./brand-marks.tsx";
import { CardAvatar } from "./card-avatar.tsx";
import { compactCount, formatPosted } from "./format.ts";

/** Up to four media tiles, as X lays them out; a video or GIF shows its still with a play mark. */
function Media({ media, url }: { readonly media: readonly XMedia[]; readonly url: string }) {
  const shown = media.slice(0, 4);

  return (
    <div className="x-media" data-count={shown.length}>
      {shown.map((item) => {
        const still = item.kind === "photo" ? item.url : (item.thumbnailUrl ?? item.url);

        return (
          <a
            key={item.url}
            className="x-media-item"
            href={url}
            target="_blank"
            rel="noopener noreferrer"
            aria-label={item.alt ?? (item.kind === "photo" ? "Photo" : "Video")}
          >
            <img src={still} alt={item.alt ?? ""} loading="lazy" decoding="async" />
            {item.kind === "photo" ? null : (
              <span className="x-play" aria-hidden="true">
                <Icon name="play" size={18} />
              </span>
            )}
          </a>
        );
      })}
    </div>
  );
}

/** The post it quotes, as a nested box. */
function Quote({ quote }: { readonly quote: XQuote }) {
  const head = (
    <span className="x-quote-head">
      <span className="x-name">{quote.authorName ?? "Post on X"}</span>
      {quote.authorHandle === null ? null : <span className="x-handle">@{quote.authorHandle}</span>}
    </span>
  );

  const body = quote.text === null ? null : <span className="x-quote-text">{quote.text}</span>;

  return quote.url === null ? (
    <div className="x-quote">
      {head}
      {body}
    </div>
  ) : (
    <a className="x-quote" href={quote.url} target="_blank" rel="noopener noreferrer">
      {head}
      {body}
    </a>
  );
}

/** A count under the post, compact ("1.2K"), with what it counts for screen readers. */
function Stat({
  icon,
  value,
  label,
}: {
  readonly icon: "message-circle" | "repeat-2" | "heart";
  readonly value: number | null;
  readonly label: string;
}) {
  if (value === null) {
    return null;
  }

  return (
    <span className="x-stat">
      <Icon name={icon} size={14} />
      <span className="tabular" aria-hidden="true">
        {compactCount(value)}
      </span>
      <span className="visually-hidden">
        {value} {label}
      </span>
    </span>
  );
}

/**
 * A post on X: the author, the text, photos and video stills, a quoted post, and the counts.
 * While the server is still fetching it shows a frame; when the fetch failed, just the link.
 */
export function XCard({ card }: { readonly card: XPostCard }) {
  if (card.fetch === "failed") {
    return (
      <a
        className="card card-chip x-chip"
        href={card.url}
        target="_blank"
        rel="noopener noreferrer"
      >
        <BrandMark name="x" size={14} />
        <span>{card.authorName}</span>
        <Icon name="arrow-up-right" size={12} />
      </a>
    );
  }

  if (card.fetch === "loading") {
    return (
      <section className="card x-card" aria-busy="true" aria-label="Post on X">
        <div className="x-head">
          <Skeleton width={36} height={36} radius="pill" />
          <div className="x-author">
            <Skeleton width={120} height={12} />
            <Skeleton width={80} height={10} />
          </div>
          <BrandMark name="x" size={16} className="x-mark" />
        </div>
        <Skeleton height={12} />
        <Skeleton width="70%" height={12} />
      </section>
    );
  }

  return (
    <section className="card x-card" aria-label={`Post by ${card.authorName}`}>
      <div className="x-head">
        <CardAvatar
          name={card.authorName}
          seed={card.authorHandle ?? card.authorName}
          src={card.authorAvatarUrl}
          size={36}
        />
        <a className="x-author" href={card.url} target="_blank" rel="noopener noreferrer">
          <span className="x-name">{card.authorName}</span>
          {card.authorHandle === null ? null : (
            <span className="x-handle">@{card.authorHandle}</span>
          )}
        </a>
        <BrandMark name="x" size={16} className="x-mark" />
      </div>
      {card.text === null ? null : <p className="x-text">{card.text}</p>}
      {card.media.length === 0 ? null : <Media media={card.media} url={card.url} />}
      {card.quote === null ? null : <Quote quote={card.quote} />}
      <div className="x-foot">
        <Stat icon="message-circle" value={card.replies} label="replies" />
        <Stat icon="repeat-2" value={card.reposts} label="reposts" />
        <Stat icon="heart" value={card.likes} label="likes" />
        {card.postedAt === null ? null : (
          <a className="x-time" href={card.url} target="_blank" rel="noopener noreferrer">
            {formatPosted(card.postedAt)}
          </a>
        )}
      </div>
    </section>
  );
}
