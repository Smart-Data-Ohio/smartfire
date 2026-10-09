import { Link } from "@tanstack/react-router";
import type { KeyboardEvent } from "react";
import type { AgentLedgerEvent } from "../../gen/AgentLedgerEvent.ts";
import { formatFull } from "../../lib/time.ts";
import { useStore } from "../../store/store.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { Tooltip } from "../../ui/tooltip.tsx";
import { focusSiblingRow, ListRow } from "../destinations/list-row.tsx";
import { timeAgo } from "../threads/thread-format.ts";
import {
  externalLine,
  hopText,
  ledgerIcon,
  ledgerSentence,
  outcomeLabel,
  webhookLine,
} from "./ledger-format.ts";

interface LedgerRowProps {
  readonly event: AgentLedgerEvent;
  readonly agentName: string;
  readonly now: number;
}

/** Where it happened: the room (a link), "a room you're not in", or nothing outside a room. */
function Room({ event }: { readonly event: AgentLedgerEvent }) {
  if (event.roomId === null) {
    return null;
  }

  return event.roomName === null ? (
    <span className="ledger-room" data-hidden="">
      a room you're not in
    </span>
  ) : (
    <Link to="/r/$roomId" params={{ roomId: event.roomId }} className="ledger-room" preload={false}>
      {event.roomName}
    </Link>
  );
}

/**
 * One ledger entry: a glyph and a sentence, its outcome and when; then where, the hop count, the
 * detail, a handoff's summary, the message (or "Content unavailable"), a GitHub or Fizzy result
 * and the webhook's delivery, with a link to the message when the viewer can open it. Read-only:
 * the row's sentence takes focus for arrow keys, and its links follow in the Tab order.
 */
export function LedgerRow({ event, agentName, now }: LedgerRowProps) {
  const actor = useStore((state) =>
    event.actorId === null ? null : (state.users[event.actorId]?.name ?? null),
  );

  const webhook = webhookLine(event);
  const external = externalLine(event);
  const hop = hopText(event.hop);
  const viewable = event.messageId !== null && event.roomId !== null && event.roomName !== null;

  const onKeyDown = (keyboard: KeyboardEvent<HTMLDivElement>) => {
    const plain = !keyboard.metaKey && !keyboard.ctrlKey && !keyboard.altKey && !keyboard.shiftKey;

    if (plain && (keyboard.key === "ArrowDown" || keyboard.key === "ArrowUp")) {
      keyboard.preventDefault();
      focusSiblingRow(keyboard.currentTarget, keyboard.key === "ArrowDown" ? 1 : -1);
    }
  };

  return (
    <ListRow motion={undefined} state={event.outcome ?? "none"} onKeyDown={onKeyDown}>
      <div className="ledger-row" data-type={event.eventType}>
        <span className="ledger-glyph" data-outcome={event.outcome ?? "none"} aria-hidden="true">
          <Icon name={ledgerIcon(event.eventType)} size={16} />
        </span>
        <div className="ledger-main">
          <div className="ledger-line">
            <p className="ledger-sentence list-row-open" tabIndex={-1}>
              {ledgerSentence(event.eventType, { actor, agent: agentName })}
            </p>
            {event.outcome === null ? null : (
              <span className="ledger-chip" data-outcome={event.outcome}>
                {outcomeLabel(event.outcome)}
              </span>
            )}
            <Tooltip content={formatFull(event.createdAt)} describe={false}>
              <time className="ledger-time list-row-time" dateTime={event.createdAt}>
                {timeAgo(event.createdAt, now)}
              </time>
            </Tooltip>
          </div>
          {event.roomId === null && hop === null ? null : (
            <p className="ledger-meta">
              <Room event={event} />
              {event.roomId !== null && hop !== null ? <span aria-hidden="true">·</span> : null}
              {hop === null ? null : <span>{hop}</span>}
            </p>
          )}
          {event.detail === null ? null : <p className="ledger-detail">{event.detail}</p>}
          {event.handoffSummary === null ? null : (
            <p className="ledger-detail">
              <span className="ledger-label">Handoff:</span> {event.handoffSummary}
            </p>
          )}
          {event.messageId === null ? null : event.content === null ? (
            <p className="ledger-content" data-unavailable="">
              Content unavailable
            </p>
          ) : (
            <blockquote className="ledger-content">{event.content}</blockquote>
          )}
          {external === null ? null : (
            <p className="ledger-external">
              <Icon name="external-link" size={12} className="ledger-external-icon" />
              <span>{external}</span>
            </p>
          )}
          {webhook === null ? null : (
            <p className="ledger-webhook" data-status={event.webhookStatus}>
              {webhook}
            </p>
          )}
          {viewable && event.messageId !== null && event.roomId !== null ? (
            <Link
              to="/r/$roomId/m/$messageId"
              params={{ roomId: event.roomId, messageId: event.messageId }}
              className="ledger-open"
              preload={false}
            >
              View message
              <Icon name="chevron-right" size={12} />
            </Link>
          ) : null}
        </div>
      </div>
    </ListRow>
  );
}
