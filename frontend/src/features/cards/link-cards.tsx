import { useState } from "react";
import type { DriveFileCard } from "../../gen/DriveFileCard.ts";
import type { LinkCard as LinkCardData } from "../../gen/LinkCard.ts";
import type { LinkedinCard as LinkedinCardData } from "../../gen/LinkedinCard.ts";
import { Button } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { BrandMark } from "./brand-marks.tsx";
import { hostOf } from "./format.ts";

/** A page's picture, hidden (not broken) when it fails to load. */
function Thumb({ src, className }: { readonly src: string; readonly className: string }) {
  const [broken, setBroken] = useState(false);

  if (broken) {
    return null;
  }

  return (
    <img
      className={className}
      src={src}
      alt=""
      loading="lazy"
      decoding="async"
      onError={() => setBroken(true)}
    />
  );
}

/** A preview of any other page the message links to: site, title, description and picture. */
export function LinkCard({ card }: { readonly card: LinkCardData }) {
  return (
    <a className="card link-card" href={card.url} target="_blank" rel="noopener noreferrer">
      <span className="link-text">
        <span className="link-site">
          <Icon name="globe" size={12} />
          {card.siteName ?? hostOf(card.url)}
        </span>
        {card.title === null ? null : <span className="link-title">{card.title}</span>}
        {card.description === null ? null : (
          <span className="link-description">{card.description}</span>
        )}
      </span>
      {card.imageUrl === null ? null : <Thumb src={card.imageUrl} className="link-image" />}
    </a>
  );
}

/**
 * A LinkedIn post: its title, text and picture, and "Show embedded post" to load LinkedIn's own
 * player in place (only then: it's heavy). Without a title or text, the "View post on LinkedIn"
 * chip.
 */
export function LinkedinCard({ card }: { readonly card: LinkedinCardData }) {
  const [embedded, setEmbedded] = useState(false);

  if (card.title === null && card.description === null) {
    return (
      <a
        className="card card-chip linkedin-chip"
        href={card.url}
        target="_blank"
        rel="noopener noreferrer"
      >
        <BrandMark name="linkedin" size={14} className="linkedin-mark" />
        <span>View post on LinkedIn</span>
        <Icon name="arrow-up-right" size={12} />
      </a>
    );
  }

  return (
    <section className="card linkedin-card" aria-label="LinkedIn post">
      <a className="linkedin-body" href={card.url} target="_blank" rel="noopener noreferrer">
        <span className="link-site">
          <BrandMark name="linkedin" size={12} className="linkedin-mark" />
          LinkedIn
        </span>
        {card.title === null ? null : <span className="link-title">{card.title}</span>}
        {card.description === null ? null : (
          <span className="link-description">{card.description}</span>
        )}
        {card.imageUrl === null || embedded ? null : (
          <Thumb src={card.imageUrl} className="linkedin-image" />
        )}
      </a>
      {card.embedUrl === null ? null : embedded ? (
        <iframe
          className="linkedin-embed"
          src={card.embedUrl}
          title={card.title ?? "LinkedIn post"}
          loading="lazy"
          sandbox="allow-scripts allow-same-origin allow-popups"
          referrerPolicy="strict-origin-when-cross-origin"
        />
      ) : (
        <div className="linkedin-actions">
          <Button variant="ghost" size="sm" icon="play" onClick={() => setEmbedded(true)}>
            Show embedded post
          </Button>
        </div>
      )}
    </section>
  );
}

/** A Google Drive file in the attachment slot: the file's name isn't stored, so it says what it is. */
export function DriveChip({ card }: { readonly card: DriveFileCard }) {
  return (
    <a className="card drive-chip" href={card.url} target="_blank" rel="noopener noreferrer">
      <span className="drive-icon" aria-hidden="true">
        <BrandMark name="drive" size={18} />
      </span>
      <span className="drive-text">
        <span className="drive-title">Google Drive file</span>
        <span className="card-subtle">Open in Drive</span>
      </span>
      <Icon name="arrow-up-right" size={14} className="drive-open" />
    </a>
  );
}
