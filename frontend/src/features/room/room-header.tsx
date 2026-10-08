import { Link } from "@tanstack/react-router";
import { useStore } from "../../store/store.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { Skeleton } from "../../ui/skeleton.tsx";
import { useWorkingPresence } from "../agents/working.ts";
import { DirectHeaderActions } from "../directs/direct-header-actions.tsx";
import { EventsLink } from "../events/events-link.tsx";
import { HuddleLauncher } from "../huddle/huddle-launcher.tsx";
import { PaneButtons } from "../panes/pane-buttons.tsx";
import { agentKindLabel, agentTone, identityOf, toneLabel } from "../people/agent-identity.ts";
import { usePresenceStatus, useUser } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { RoomGlyph } from "../rooms/room-glyph.tsx";
import { preloadRoomSettings, settingsOverState } from "../rooms/room-settings-host.tsx";
import { HeaderSearch } from "../search/header-search.tsx";
import { NotificationsButton } from "../sidebar/notifications-button.tsx";

const PRESENCE_TEXT = {
  online: "Active",
  away: "Away",
  dnd: "Do not disturb",
  offline: "Offline",
} as const;

/**
 * An agent DM's subtitle: its kind and status ("Workspace agent · Working"), linking to its
 * profile; a bot without an agent row is just "Bot".
 */
function AgentSubtitle({ userId }: { readonly userId: number }) {
  const identity = identityOf(useUser(userId));
  const working = useWorkingPresence(userId);

  if (identity.kind !== "agent") {
    return <span className="room-header-topic room-header-agent">Bot</span>;
  }

  const tone = agentTone(identity) ?? "idle";
  const status = working ?? toneLabel(tone, identity.status);

  return (
    <Link
      to="/agents/$agentId"
      params={{ agentId: identity.agentId }}
      className="room-header-topic room-header-agent"
      data-tone={tone}
      preload={false}
    >
      {agentKindLabel(identity.agentKind)} · {status}
    </Link>
  );
}

/** A DM's subtitle: the other person's status line or presence. */
function DirectSubtitle({ userId }: { readonly userId: number }) {
  const status = usePresenceStatus(userId);
  const statusText = useStore((state) => state.presence[userId]?.statusText ?? null);
  const user = useUser(userId);

  if (user?.role === "bot") {
    return <AgentSubtitle userId={userId} />;
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
  const iconName = detail?.room.iconName ?? row?.room.iconName ?? null;
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
          {kind === "direct" ? (
            <>
              {otherId === undefined ? (
                <RoomGlyph kind={kind} iconName={null} size={18} className="room-title-icon" />
              ) : (
                <UserAvatar userId={otherId} size={24} presence decorative />
              )}
              <h1 className="room-title-name">{name}</h1>
              {otherId === undefined ? null : <DirectSubtitle userId={otherId} />}
            </>
          ) : (
            // Slack's channel name button: the name opens the room's settings.
            <Link
              to="/r/$roomId/settings"
              params={{ roomId }}
              state={settingsOverState(roomId)}
              className="room-title-button"
              aria-label={`${name}, room settings`}
              onPointerEnter={preloadRoomSettings}
              onFocus={preloadRoomSettings}
            >
              <RoomGlyph kind={kind} iconName={iconName} size={18} className="room-title-icon" />
              <h1 className="room-title-name">{name}</h1>
              <Icon name="chevron-down" size={14} className="room-title-chevron" />
            </Link>
          )}
        </div>
      )}
      <div className="room-header-tools">
        {kind === "direct" ? <DirectHeaderActions roomId={roomId} /> : null}
        <HuddleLauncher roomId={roomId} />
        <NotificationsButton roomId={roomId} />
        {kind === null ? null : <EventsLink roomId={roomId} />}
        <PaneButtons roomId={roomId} />
        <HeaderSearch />
      </div>
    </header>
  );
}
