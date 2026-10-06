import { Link } from "@tanstack/react-router";
import type { SidebarRow as Row } from "../../store/model.ts";
import { useStore } from "../../store/store.ts";
import { AgentThinking } from "../../ui/agent-thinking.tsx";
import { Badge } from "../../ui/badge.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { isAgent, useUser } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { ROOM_KIND_ICON } from "../room/room-icon.ts";
import { rowPillCount } from "./sections.ts";

/** Whether someone (other than the viewer) is typing in the room right now. */
function useSomeoneTyping(roomId: number): number | null {
  return useStore((state) => {
    const typists = state.typing[`room:${roomId}`];

    if (typists === undefined) {
      return null;
    }

    const first = Object.keys(typists)[0];

    return first === undefined ? null : Number(first);
  });
}

function DirectGlyph({ row }: { readonly row: Row }) {
  const otherId = row.directMemberIds[0];
  const typistId = useSomeoneTyping(row.room.id);
  const typist = useUser(typistId ?? undefined);

  if (typistId !== null && isAgent(typist)) {
    return (
      <span className="sidebar-row-glyph">
        <AgentThinking
          size={20}
          state="composing"
          label={`${typist?.name ?? "Agent"} is replying`}
        />
      </span>
    );
  }

  if (row.directMemberIds.length > 1) {
    return (
      <span className="sidebar-row-glyph sidebar-row-group" aria-hidden="true">
        {row.directMemberIds.length}
      </span>
    );
  }

  return (
    <span className="sidebar-row-glyph">
      {otherId === undefined ? null : <UserAvatar userId={otherId} size={20} presence decorative />}
    </span>
  );
}

interface SidebarRowProps {
  readonly row: Row;
  readonly selected: boolean;
}

/**
 * One conversation in the sidebar. Unread rows are bold, muted rows faint, mentions (and every
 * unread direct message) carry a count that pops in, and the selected row is tinted.
 */
export function SidebarRow({ row, selected }: SidebarRowProps) {
  const { room, membership } = row;
  const unread = membership.unreadAt !== null;
  const muted = membership.involvement === "muted";
  const pill = rowPillCount(row);
  const state = selected ? "selected" : muted ? "muted" : unread ? "unread" : undefined;

  return (
    <li>
      <Link
        to="/r/$roomId"
        params={{ roomId: room.id }}
        className="sidebar-row"
        data-state={state}
        aria-current={selected ? "page" : undefined}
        preload={false}
      >
        {room.kind === "direct" ? (
          <DirectGlyph row={row} />
        ) : (
          <span className="sidebar-row-glyph">
            <Icon name={ROOM_KIND_ICON[room.kind]} size={16} className="sidebar-row-icon" />
          </span>
        )}
        <span className="sidebar-row-name">{row.displayName}</span>
        {muted ? <Icon name="bell-off" size={14} className="sidebar-row-muted" /> : null}
        <Badge
          count={muted ? 0 : pill}
          tone="danger"
          label={`${pill} ${room.kind === "direct" ? "unread" : "mentions"}`}
        />
      </Link>
    </li>
  );
}
