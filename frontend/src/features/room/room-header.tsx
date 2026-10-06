import { Link } from "@tanstack/react-router";
import { useStore } from "../../store/store.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { Skeleton } from "../../ui/skeleton.tsx";
import { DirectHeaderActions } from "../directs/direct-header-actions.tsx";
import { PaneButtons } from "../panes/pane-buttons.tsx";
import { usePresenceStatus, useUser } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { ROOM_KIND_ICON } from "./room-icon.ts";

const PRESENCE_TEXT = {
  online: "Active",
  away: "Away",
  dnd: "Do not disturb",
  offline: "Offline",
} as const;

/** A DM's subtitle: the other person's status line or presence. */
function DirectSubtitle({ userId }: { readonly userId: number }) {
  const status = usePresenceStatus(userId);
  const statusText = useStore((state) => state.presence[userId]?.statusText ?? null);
  const user = useUser(userId);

  if (user?.role === "bot") {
    return <span className="room-header-topic room-header-agent">Agent</span>;
  }

  const text = statusText ?? (status === undefined ? null : PRESENCE_TEXT[status]);

  return text === null ? null : <span className="room-header-topic">{text}</span>;
}

/**
 * The 48 px pane header, Slack's: the room's glyph and name (a DM shows the person, their
 * presence and status) on the left, the tools on the right (a DM's people actions, then the pane
 * toggles: threads, pins, files and the member stack). On phones a back button returns to the
 * list.
 */
export function RoomHeader({ roomId }: { readonly roomId: number }) {
  const detail = useStore((state) => state.rooms[roomId]?.detail ?? null);
  const row = useStore((state) => state.sidebar.rows[roomId] ?? null);
  const kind = detail?.room.kind ?? row?.room.kind ?? null;
  const name = detail?.displayName ?? row?.displayName ?? null;
  const directIds = detail?.directMemberIds ?? row?.directMemberIds ?? [];
  const otherId = kind === "direct" && directIds.length === 1 ? directIds[0] : undefined;

  return (
    <header className="room-header">
      <Link to="/" className="room-back" aria-label="Back to conversations">
        <Icon name="chevron-left" size={20} />
      </Link>
      {name === null || kind === null ? (
        <Skeleton width={140} height={14} />
      ) : (
        <div className="room-title enter-fade">
          {otherId === undefined ? (
            <Icon name={ROOM_KIND_ICON[kind]} size={18} className="room-title-icon" />
          ) : (
            <UserAvatar userId={otherId} size={24} presence decorative />
          )}
          <h1 className="room-title-name">{name}</h1>
          {otherId === undefined ? null : <DirectSubtitle userId={otherId} />}
        </div>
      )}
      <div className="room-header-tools">
        {kind === "direct" ? <DirectHeaderActions roomId={roomId} /> : null}
        <PaneButtons roomId={roomId} />
      </div>
    </header>
  );
}
