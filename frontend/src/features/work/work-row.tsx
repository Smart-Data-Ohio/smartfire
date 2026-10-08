import { Link } from "@tanstack/react-router";
import type { KeyboardEvent } from "react";
import type { WorkLink } from "../../gen/WorkLink.ts";
import type { WorkListRow } from "../../gen/WorkListRow.ts";
import { formatFull } from "../../lib/time.ts";
import { useStore } from "../../store/store.ts";
import { Icon, type IconName } from "../../ui/icons/icon.tsx";
import { ownerLabel, WORK_STATUS_LABEL } from "../boards/board-format.ts";
import { OwnerLine, StatusChip, TagList } from "../boards/board-parts.tsx";
import { focusSiblingRow, ListRow } from "../destinations/list-row.tsx";
import { ROOM_KIND_ICON } from "../room/room-icon.ts";
import { threadTitle, timeAgo } from "../threads/thread-format.ts";

/** How many linked items a row names before "+N". */
const SHOWN_LINKS = 3;

const LINK_ICON = {
  pull_request: "git-pull-request",
  event: "calendar",
  drive_file: "file-text",
} as const satisfies Record<WorkLink["kind"], IconName>;

/** "12 messages", as the classic row counts them (every message in the thread). */
export function messageCountLabel(count: number): string {
  return `${count} ${count === 1 ? "message" : "messages"}`;
}

/** A link goes into an `href` only when it's `https://` or a site path, as the contract says. */
function linkHref(url: string): string | null {
  return url.startsWith("https://") || (url.startsWith("/") && !url.startsWith("//")) ? url : null;
}

/** The thread's linked items as small chips, as the classic row's links box lists them. */
function LinkChips({ links }: { readonly links: readonly WorkLink[] }) {
  if (links.length === 0) {
    return null;
  }

  const more = links.length - SHOWN_LINKS;

  return (
    <ul className="work-links" aria-label="Linked">
      {links.slice(0, SHOWN_LINKS).map((link) => {
        const href = linkHref(link.url);
        const icon: IconName = LINK_ICON[link.kind] ?? "link";

        const content = (
          <>
            <Icon name={icon} size={12} />
            <span className="work-link-label">{link.label}</span>
            {link.pullRequestState === null ? null : (
              <span className="work-link-state" data-state={link.pullRequestState}>
                {link.pullRequestState}
              </span>
            )}
          </>
        );

        return (
          <li key={link.id}>
            {href === null ? (
              <span className="work-link">{content}</span>
            ) : (
              <a
                className="work-link"
                href={href}
                title={link.title ?? link.label}
                {...(href.startsWith("https://") ? { target: "_blank", rel: "noreferrer" } : {})}
              >
                {content}
              </a>
            )}
          </li>
        );
      })}
      {more > 0 ? <li className="work-link-more">+{more}</li> : null}
    </ul>
  );
}

/** Where the work lives: the room's glyph (a board's for a post) and its name. */
function RoomLabel({ row }: { readonly row: WorkListRow }) {
  const kind = useStore(
    (state) =>
      state.sidebar.rows[row.thread.roomId]?.room.kind ??
      state.rooms[row.thread.roomId]?.detail?.room.kind ??
      "open",
  );

  return (
    <span className="work-room">
      <Icon
        name={ROOM_KIND_ICON[row.board ? "board" : kind]}
        size={12}
        className="work-room-glyph"
      />
      <span className="work-room-name">{row.roomName}</span>
    </span>
  );
}

/**
 * One tracked thread (`work_threads/_thread.html`): its name and status, the room, the owner (an
 * agent tagged as one), how many messages it has and when it was last updated, and what's linked
 * to it. The whole row opens the thread in its room's right pane (a board post in its board);
 * ↑/↓ move between rows.
 */
export function WorkRow({ row, now }: { readonly row: WorkListRow; readonly now: number }) {
  const { thread } = row;
  const work = thread.work;

  if (work === null) {
    return null;
  }

  const title = threadTitle(thread);
  const status = WORK_STATUS_LABEL[work.status] ?? work.status;

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const plain = !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey;

    if (plain && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
      event.preventDefault();
      focusSiblingRow(event.currentTarget, event.key === "ArrowDown" ? 1 : -1);
    }
  };

  return (
    <ListRow motion={undefined} state={work.status} onKeyDown={onKeyDown}>
      <Link
        to="/r/$roomId/t/$threadId"
        params={{ roomId: thread.roomId, threadId: thread.id }}
        className="list-row-open list-row-cover"
        aria-label={`${title}, ${status}, in ${row.roomName}, ${ownerLabel(work)}`}
      />
      <div className="work-row" data-status={work.status}>
        <div className="work-main">
          <div className="work-head">
            <span className="work-title">{title}</span>
            <TagList tags={work.tags} />
          </div>
          <div className="work-meta">
            <RoomLabel row={row} />
            <span className="work-meta-dot" aria-hidden="true" />
            <OwnerLine work={work} />
            <span className="work-meta-dot" aria-hidden="true" />
            <span className="work-meta-item tabular">{messageCountLabel(work.messageCount)}</span>
          </div>
          <LinkChips links={work.links} />
        </div>
        <div className="work-side">
          <StatusChip status={work.status} />
          <time className="work-time" dateTime={row.updatedAt} title={formatFull(row.updatedAt)}>
            Updated {timeAgo(row.updatedAt, now)}
          </time>
        </div>
      </div>
    </ListRow>
  );
}
