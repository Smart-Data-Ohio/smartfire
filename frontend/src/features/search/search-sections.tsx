import { Link } from "@tanstack/react-router";
import { useState } from "react";
import type { ConversationName } from "../../gen/ConversationName.ts";
import type { SearchSection } from "../../gen/SearchSection.ts";
import type { SearchSectionRow } from "../../gen/SearchSectionRow.ts";
import { formatFull } from "../../lib/time.ts";
import { conversationKey } from "../../store/search.ts";
import { Button } from "../../ui/button.tsx";
import { Icon, type IconName } from "../../ui/icons/icon.tsx";
import { hitDay, SECTION_TITLE, WORK_STATUS_LABEL } from "./format.ts";

const SECTION_ICON = {
  board_posts: "boards",
  work_threads: "thread",
  events: "calendar-clock",
} as const satisfies Record<SearchSection["kind"], IconName>;

/** Rows a section shows before "Show all". */
const COLLAPSED_ROWS = 3;

interface RowProps {
  readonly kind: SearchSection["kind"];
  readonly row: SearchSectionRow;
  readonly roomName: string | null;
  readonly now: number;
}

function RowBody({ kind, row, roomName, now }: RowProps) {
  const day = hitDay(row.time, now);
  const when = ["Today", "Yesterday", "Tomorrow"].includes(day) ? day.toLowerCase() : day;

  return (
    <>
      <span className="search-section-icon" data-kind={kind}>
        <Icon name={SECTION_ICON[kind]} size={16} />
      </span>
      <span className="search-section-text">
        <span className="search-section-title" data-cancelled={row.cancelled || undefined}>
          {row.title}
        </span>
        <span className="search-section-meta">
          {row.cancelled ? (
            <span className="search-status" data-status="cancelled">
              Cancelled
            </span>
          ) : null}
          {row.workStatus === null ? null : (
            <span className="search-status" data-status={row.workStatus}>
              {WORK_STATUS_LABEL[row.workStatus]}
            </span>
          )}
          {roomName === null ? null : <span>{roomName}</span>}
          <time dateTime={row.time} title={formatFull(row.time)}>
            {kind !== "events"
              ? `Active ${when}`
              : Date.parse(row.time) > now
                ? `Starts ${when}`
                : `Started ${when}`}
          </time>
        </span>
      </span>
    </>
  );
}

/**
 * One section row. A work thread opens in the SPA's thread pane; board posts and events have no
 * SPA page yet, so they open the classic one.
 */
function SectionRow(props: RowProps) {
  const { kind, row } = props;

  if (kind === "work_threads") {
    return (
      <Link
        to="/r/$roomId/t/$threadId"
        params={{ roomId: row.roomId, threadId: row.id }}
        className="search-section-row"
      >
        <RowBody {...props} />
      </Link>
    );
  }

  const href =
    kind === "events"
      ? `/rooms/${row.roomId}/events/${row.id}`
      : `/rooms/${row.roomId}/threads/${row.id}`;

  return (
    <a href={href} className="search-section-row">
      <RowBody {...props} />
    </a>
  );
}

function Section({
  section,
  conversations,
  now,
}: {
  readonly section: SearchSection;
  readonly conversations: Readonly<Record<string, ConversationName>>;
  readonly now: number;
}) {
  const [expanded, setExpanded] = useState(false);
  const rows = expanded ? section.rows : section.rows.slice(0, COLLAPSED_ROWS);
  const hidden = section.rows.length - rows.length;

  return (
    <section className="search-section" aria-label={SECTION_TITLE[section.kind]}>
      <h3 className="search-section-heading">
        {SECTION_TITLE[section.kind]}
        <span className="search-section-count">{section.rows.length}</span>
      </h3>
      <ul className="search-section-list">
        {rows.map((row) => (
          <li key={row.id}>
            <SectionRow
              kind={section.kind}
              row={row}
              roomName={
                conversations[conversationKey(row.roomId, null)]?.roomName ??
                conversations[conversationKey(row.roomId, row.id)]?.roomName ??
                null
              }
              now={now}
            />
          </li>
        ))}
      </ul>
      {hidden > 0 || expanded ? (
        <Button variant="link" size="sm" onClick={() => setExpanded(!expanded)}>
          {expanded ? "Show fewer" : `Show all ${section.rows.length}`}
        </Button>
      ) : null}
    </section>
  );
}

/** The first page's board posts, work threads and events, above the messages. */
export function SearchSections({
  sections,
  conversations,
  now,
}: {
  readonly sections: readonly SearchSection[];
  readonly conversations: Readonly<Record<string, ConversationName>>;
  readonly now: number;
}) {
  if (sections.length === 0) {
    return null;
  }

  return (
    <div className="search-sections">
      {sections.map((section) => (
        <Section key={section.kind} section={section} conversations={conversations} now={now} />
      ))}
    </div>
  );
}
