import { type ReactNode, useCallback } from "react";
import type { FizzyCard as FizzyCardData } from "../../gen/FizzyCard.ts";
import type { FizzyCardRef } from "../../gen/FizzyCardRef.ts";
import { fizzyKey } from "../../store/cards.ts";
import type { MessageDTO } from "../../store/model.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Skeleton } from "../../ui/skeleton.tsx";
import { useNow } from "../threads/use-now.ts";
import { CardAvatar } from "./card-avatar.tsx";
import { ago } from "./format.ts";
import { usePreview } from "./use-preview.ts";

/** The most assignees shown as faces. */
const FACES = 4;

/** "In Review" for a column, else what the status means on the board. */
function statusLabel(card: FizzyCardData): string {
  switch (card.status) {
    case "closed":
      return "Done";
    case "postponed":
      return "Not now";
    case "column":
      return card.columnName ?? "In progress";
    case "triage":
      return "Maybe?";
  }
}

/** The card's number and link, the line every state starts with. */
function Head({ card, children }: { readonly card: FizzyCardRef; readonly children?: ReactNode }) {
  return (
    <div className="fizzy-head">
      <span className="fizzy-mark" aria-hidden="true">
        <Icon name="list-checks" size={14} />
      </span>
      <a className="fizzy-number tabular" href={card.url} target="_blank" rel="noopener noreferrer">
        Fizzy #{card.number}
      </a>
      {children}
    </div>
  );
}

function Loaded({ card, fizzy }: { readonly card: FizzyCardRef; readonly fizzy: FizzyCardData }) {
  const now = useNow();
  const shown = fizzy.assignees.slice(0, FACES);
  const more = fizzy.assignees.length - shown.length;

  return (
    <section
      className="card fizzy-card"
      data-status={fizzy.status}
      aria-label={`Fizzy card: ${fizzy.title}`}
    >
      <Head card={card}>
        {fizzy.boardName === null ? null : <span className="fizzy-board">{fizzy.boardName}</span>}
        <span className="card-tag fizzy-status" data-status={fizzy.status}>
          {statusLabel(fizzy)}
        </span>
      </Head>
      <a className="fizzy-title" href={fizzy.url} target="_blank" rel="noopener noreferrer">
        {fizzy.title}
      </a>
      {fizzy.tags.length === 0 ? null : (
        <ul className="fizzy-tags">
          {fizzy.tags.map((tag) => (
            <li key={tag} className="fizzy-tag">
              #{tag}
            </li>
          ))}
        </ul>
      )}
      <div className="fizzy-foot card-subtle">
        {shown.length === 0 ? null : (
          <span
            className="fizzy-assignees"
            title={fizzy.assignees.map((person) => person.name).join(", ")}
          >
            {shown.map((person) => (
              <CardAvatar
                key={person.name}
                name={person.name}
                seed={person.name}
                src={person.avatarUrl}
                size={20}
              />
            ))}
            {more > 0 || fizzy.hasMoreAssignees ? <span className="fizzy-more">+ more</span> : null}
            <span className="visually-hidden">
              Assigned to {fizzy.assignees.map((person) => person.name).join(", ")}
            </span>
          </span>
        )}
        {fizzy.stepsTotal === 0 ? null : (
          <span className="fizzy-steps">
            <Icon name="circle-check" size={12} />
            <span className="tabular">
              {fizzy.stepsCompleted}/{fizzy.stepsTotal}
            </span>
            <span className="visually-hidden">steps done</span>
            <span
              className="fizzy-steps-bar"
              style={{
                "--fizzy-progress": `${Math.round((fizzy.stepsCompleted / fizzy.stepsTotal) * 100)}%`,
              }}
              aria-hidden="true"
            />
          </span>
        )}
        {fizzy.lastActiveAt === null ? null : <span>Active {ago(fizzy.lastActiveAt, now)}</span>}
      </div>
    </section>
  );
}

/** A one-line Fizzy state (not connected, not found, failed) under the card's number. */
function Notice({ card, children }: { readonly card: FizzyCardRef; readonly children: ReactNode }) {
  return (
    <section
      className="card fizzy-card"
      data-state="notice"
      aria-label={`Fizzy card #${card.number}`}
    >
      <Head card={card} />
      <div className="fizzy-notice">{children}</div>
    </section>
  );
}

/**
 * A Fizzy card the message links to, as the viewer's own Fizzy account sees it: board, title,
 * where it stands, assignees, tags and steps. Without a connected account, or when Fizzy can't
 * find it, a line says so; a failed fetch offers Retry.
 */
export function FizzyCard({
  message,
  card,
}: {
  readonly message: MessageDTO;
  readonly card: FizzyCardRef;
}) {
  const load = useCallback(
    () => actions.cards.loadFizzy(message.roomId, card.fizzyCardId, message.id),
    [message.roomId, card.fizzyCardId, message.id],
  );

  const preview = usePreview("fizzy", fizzyKey(message.roomId, card.fizzyCardId, message.id), load);
  const value = preview?.value ?? null;
  const retry = () => load().catch(() => undefined);

  const failed = (reason: string) => (
    <Notice card={card}>
      <span className="card-error">
        <Icon name="circle-alert" size={14} />
        Couldn't load this card. {reason}
      </span>
      <Button variant="secondary" size="sm" icon="refresh-cw" onClick={retry}>
        Retry
      </Button>
    </Notice>
  );

  if (value === null) {
    return preview?.status === "error" ? (
      failed(preview.error ?? "")
    ) : (
      <section
        className="card fizzy-card"
        aria-busy="true"
        aria-label={`Fizzy card #${card.number}`}
      >
        <Head card={card} />
        <Skeleton width="75%" height={14} />
        <Skeleton width="40%" height={10} />
      </section>
    );
  }

  switch (value.state) {
    case "not_connected":
      return (
        <Notice card={card}>
          <span className="card-subtle">
            <Icon name="lock" size={12} /> Connect Fizzy in your settings to see this card.
          </span>
        </Notice>
      );
    case "not_found":
      return (
        <Notice card={card}>
          <span className="card-subtle">
            <Icon name="eye-off" size={12} /> This card isn't there, or your Fizzy account can't see
            it.
          </span>
        </Notice>
      );
    case "loading":
      return (
        <section
          className="card fizzy-card"
          aria-busy="true"
          aria-label={`Fizzy card #${card.number}`}
        >
          <Head card={card} />
          <Skeleton width="75%" height={14} />
          <Skeleton width="40%" height={10} />
        </section>
      );
    case "failed":
      return failed(value.message);
    case "loaded":
      return <Loaded card={card} fizzy={value} />;
  }
}
