import type { KeyboardEvent } from "react";
import type { WorkListRow } from "../../gen/WorkListRow.ts";
import { formatFull } from "../../lib/time.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { Tooltip } from "../../ui/tooltip.tsx";
import type { RowMotion } from "../destinations/list-motion.ts";
import { focusSiblingRow, ListRow } from "../destinations/list-row.tsx";
import { threadTitle, timeAgo } from "../threads/thread-format.ts";
import { WorkLinks, WorkOwner, WorkStatusPill } from "./work-facts.tsx";
import { workStatusLabel } from "./work-format.ts";

interface WorkRowProps {
  readonly row: WorkListRow;
  readonly now: number;
  readonly motion: RowMotion;
  readonly onOpen: (row: WorkListRow) => void;
}

/**
 * One work thread: its name (and a Board marker for a board post), where it lives, its status
 * and owner, its links, and when it last changed. The whole row opens it: a channel thread in
 * its room's pane, a board post on its classic page. Arrow keys move between rows.
 */
export function WorkRow({ row, now, motion, onOpen }: WorkRowProps) {
  const { thread } = row;
  const work = thread.work;
  const name = threadTitle(thread);
  const updated = timeAgo(row.updatedAt, now);

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

  const status = work === null ? "" : `, ${workStatusLabel(work.status)}`;
  const owner = work === null ? "" : `, ${work.owner?.name ?? "unassigned"}`;

  return (
    <ListRow motion={motion} state={row.board ? "board" : undefined} onKeyDown={onKeyDown}>
      <button
        type="button"
        className="list-row-open list-row-cover"
        aria-label={`${name}${row.board ? ", board post" : ""} in ${row.roomName}${status}${owner}. Updated ${updated}`}
        onClick={() => onOpen(row)}
      />
      <div className="work-row">
        <span className="work-row-glyph" aria-hidden="true">
          <Icon name={row.board ? "boards" : "briefcase"} size={16} />
        </span>
        <div className="work-row-main">
          <div className="work-row-line">
            <span className="work-row-name">{name}</span>
            {row.board ? <span className="work-board-marker">Board</span> : null}
            <Tooltip content={formatFull(row.updatedAt)} describe={false}>
              <time className="work-row-time list-row-time" dateTime={row.updatedAt}>
                {updated}
              </time>
            </Tooltip>
          </div>
          <div className="work-row-line work-row-meta">
            <span className="work-row-room">
              <Icon name={row.board ? "boards" : "hash"} size={12} />
              {row.roomName}
            </span>
            {work === null ? null : (
              <>
                <WorkStatusPill status={work.status} />
                <WorkOwner owner={work.owner} active={work.ownerActive} />
              </>
            )}
          </div>
          {work === null ? null : (
            <WorkLinks links={work.links} runUrl={work.runUrl} label={`Links for ${name}`} />
          )}
        </div>
      </div>
    </ListRow>
  );
}
