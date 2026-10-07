import { Link } from "@tanstack/react-router";
import { type KeyboardEvent, useState } from "react";
import type { AgentApproval } from "../../gen/AgentApproval.ts";
import type { ApprovalDecision } from "../../gen/ApprovalDecision.ts";
import { formatFull } from "../../lib/time.ts";
import { useStore } from "../../store/store.ts";
import { Beam } from "../../ui/beam.tsx";
import { Button } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { Tooltip } from "../../ui/tooltip.tsx";
import type { RowMotion } from "../destinations/list-motion.ts";
import { focusSiblingRow, ListRow } from "../destinations/list-row.tsx";
import {
  actsAsText,
  adminOnlyText,
  approvalStatusLabel,
  decisionLine,
  shownStatus,
} from "./approval-format.ts";

/** The longest note a decision may carry (the server's limit). */
const NOTE_MAX = 200;

interface ApprovalCardProps {
  readonly approval: AgentApproval;
  readonly now: number;
  readonly motion: RowMotion;
  /** This card holds the page's one beam (the newest pending request). */
  readonly beamed: boolean;
  readonly onDecide: (
    approval: AgentApproval,
    decision: ApprovalDecision,
    note: string | null,
  ) => void;
}

/** Where the agent asked: the room (a link), "a room you're not in", or nothing outside a room. */
function Where({ approval }: { readonly approval: AgentApproval }) {
  if (approval.roomId === null) {
    return null;
  }

  return (
    <>
      <span aria-hidden="true">·</span>
      {approval.roomName === null ? (
        <span className="approval-room" data-hidden="">
          a room you're not in
        </span>
      ) : (
        <Link
          to="/r/$roomId"
          params={{ roomId: approval.roomId }}
          className="approval-room"
          preload={false}
        >
          {approval.roomName}
        </Link>
      )}
    </>
  );
}

/** Approve and Deny (whichever the viewer may), with an optional note. */
function Decide({
  approval,
  onDecide,
}: {
  readonly approval: AgentApproval;
  readonly onDecide: ApprovalCardProps["onDecide"];
}) {
  const [noting, setNoting] = useState(false);
  const [note, setNote] = useState("");
  const trimmed = note.trim();

  const decide = (decision: ApprovalDecision) =>
    onDecide(approval, decision, trimmed === "" ? null : trimmed);

  return (
    <div className="approval-decide">
      {noting ? (
        <div className="approval-note-field">
          <TextField
            label="Note"
            hint="Saved with your decision, for the agent and its owner."
            maxLength={NOTE_MAX}
            value={note}
            onChange={(event) => setNote(event.currentTarget.value)}
            autoFocus
          />
        </div>
      ) : null}
      <div className="approval-buttons">
        {approval.approvable ? (
          <Button variant="primary" size="sm" icon="check" onClick={() => decide("approved")}>
            Approve
          </Button>
        ) : null}
        {approval.deniable ? (
          <Button variant="secondary" size="sm" icon="x" onClick={() => decide("denied")}>
            Deny
          </Button>
        ) : null}
        {noting ? null : (
          <Button variant="ghost" size="sm" icon="pencil" onClick={() => setNoting(true)}>
            Add a note
          </Button>
        )}
      </div>
    </div>
  );
}

/**
 * One approval request as a card: what the agent wants to do and where, its status, and either
 * Approve and Deny (with an optional note) while it's pending, or who decided it and when. The
 * newest pending card carries the agent's beam; the other pending cards a still violet edge.
 */
export function ApprovalCard({ approval, now, motion, beamed, onDecide }: ApprovalCardProps) {
  const status = shownStatus(approval, now);
  const pending = status === "pending";

  const deciderName = useStore((state) =>
    approval.decidedById === null ? null : (state.users[approval.decidedById]?.name ?? null),
  );

  const actsAs = actsAsText(approval);
  const adminOnly = pending ? adminOnlyText(approval) : null;
  const decides = pending && (approval.approvable || approval.deniable);

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const plain = !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey;

    if (
      plain &&
      (event.key === "ArrowDown" || event.key === "ArrowUp") &&
      event.target instanceof HTMLElement &&
      event.target.classList.contains("list-row-open")
    ) {
      event.preventDefault();
      focusSiblingRow(event.currentTarget, event.key === "ArrowDown" ? 1 : -1);
    }
  };

  return (
    <ListRow motion={motion} state={status} onKeyDown={onKeyDown}>
      <div className="approval-card-pad">
        <Beam active={beamed && pending} radius={10}>
          <article
            className="approval-card"
            data-status={status}
            aria-label={`${approvalStatusLabel(status)}: ${approval.summary}`}
          >
            <div className="approval-head">
              <span className="approval-glyph" aria-hidden="true">
                <Icon name="shield" size={16} />
              </span>
              <div className="approval-main">
                {/* The row's main element: arrows and row focus land here; the buttons follow. */}
                <h3 className="approval-summary list-row-open" tabIndex={-1}>
                  {approval.summary}
                </h3>
                <p className="approval-meta">
                  <code className="approval-action">{approval.action}</code>
                  <Where approval={approval} />
                </p>
              </div>
              <span className="approval-chip" data-status={status}>
                {approvalStatusLabel(status)}
              </span>
            </div>
            {actsAs === null ? null : <p className="approval-acts">{actsAs}</p>}
            <Tooltip
              content={formatFull(approval.decidedAt ?? approval.createdAt)}
              describe={false}
            >
              <p className="approval-decision" data-status={status}>
                {decisionLine(approval, deciderName, now)}
              </p>
            </Tooltip>
            {approval.decisionNote === null ? null : (
              <blockquote className="approval-note">{approval.decisionNote}</blockquote>
            )}
            {decides ? <Decide approval={approval} onDecide={onDecide} /> : null}
            {adminOnly === null ? null : <p className="approval-admin-only">{adminOnly}</p>}
          </article>
        </Beam>
      </div>
    </ListRow>
  );
}
