import { Link, useMatchRoute, useNavigate } from "@tanstack/react-router";
import type { RoomKind } from "../../gen/RoomKind.ts";
import { useStore } from "../../store/store.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { MenuItem, MenuSeparator } from "../../ui/menu.tsx";
import { PageHeader } from "../../ui/page-header.tsx";
import { Skeleton } from "../../ui/skeleton.tsx";
import { useWorkingPresence } from "../agents/working.ts";
import { DirectHeaderActions, useDirectActions } from "../directs/direct-header-actions.tsx";
import { EventsLink } from "../events/events-link.tsx";
import { HuddleLauncher } from "../huddle/huddle-launcher.tsx";
import { PaneButtons, PaneMenuItems } from "../panes/pane-buttons.tsx";
import { usePaneNavigation, usePhoneLayout } from "../panes/use-right-pane.ts";
import { agentKindLabel, agentTone, identityOf, toneLabel } from "../people/agent-identity.ts";
import { usePresenceStatus, useUser } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { RoomGlyph } from "../rooms/room-glyph.tsx";
import { preloadRoomSettings, settingsOverState } from "../rooms/room-settings-host.tsx";
import { HeaderSearch } from "../search/header-search.tsx";
import {
  NotificationLevels,
  NotificationsButton,
  NotificationsSubMenu,
  useNotificationsMenuOpen,
} from "../sidebar/notifications-button.tsx";

const PRESENCE_TEXT = {
  online: "Active",
  away: "Away",
  dnd: "Do not disturb",
  offline: "Offline",
} as const;

/**
 * An agent DM's subtitle: its kind and status ("Workspace agent · Working"), linking to its
 * profile (`linked`: not inside a phone's title button, which opens the details instead); a bot
 * without an agent row is just "Bot".
 */
function AgentSubtitle({ userId, linked }: { readonly userId: number; readonly linked: boolean }) {
  const identity = identityOf(useUser(userId));
  const working = useWorkingPresence(userId);

  if (identity.kind !== "agent") {
    return <span className="room-header-topic room-header-agent">Bot</span>;
  }

  const tone = agentTone(identity) ?? "idle";
  const status = working ?? toneLabel(tone, identity.status);
  const text = `${agentKindLabel(identity.agentKind)} · ${status}`;

  return linked ? (
    <Link
      to="/agents/$agentId"
      params={{ agentId: identity.agentId }}
      className="room-header-topic room-header-agent"
      data-tone={tone}
      preload={false}
    >
      {text}
    </Link>
  ) : (
    <span className="room-header-topic room-header-agent" data-tone={tone}>
      {text}
    </span>
  );
}

/** A DM's subtitle: the other person's status line or presence. */
function DirectSubtitle({
  userId,
  linked = true,
}: {
  readonly userId: number;
  readonly linked?: boolean;
}) {
  const status = usePresenceStatus(userId);
  const statusText = useStore((state) => state.presence[userId]?.statusText ?? null);
  const user = useUser(userId);

  if (user?.role === "bot") {
    return <AgentSubtitle userId={userId} linked={linked} />;
  }

  const text = statusText ?? (status === undefined ? null : PRESENCE_TEXT[status]);

  return text === null ? null : <span className="room-header-topic">{text}</span>;
}

interface TitleProps {
  readonly roomId: number;
  readonly kind: RoomKind;
  readonly name: string;
  readonly iconName: string | null;
  readonly otherId: number | undefined;
}

/** Wide screens: the glyph and name (a DM: the person and their status inline). */
function WideTitle({ roomId, kind, name, iconName, otherId }: TitleProps) {
  return (
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
  );
}

/**
 * Phones: the name over a muted line (a channel's member count, a person's status), all one
 * button that opens the room's details, as Slack's and Discord's mobile headers do.
 */
function PhoneTitle({ roomId, kind, name, iconName, otherId }: TitleProps) {
  const { toggle } = usePaneNavigation(roomId);
  const memberCount = useStore((state) => state.rooms[roomId]?.detail?.memberCount ?? null);

  const meta =
    kind === "direct" && otherId !== undefined ? (
      <DirectSubtitle userId={otherId} linked={false} />
    ) : memberCount === null ? null : (
      <span className="room-header-topic">
        {memberCount} {memberCount === 1 ? "member" : "members"}
      </span>
    );

  return (
    <h1 className="room-title enter-fade">
      <button
        type="button"
        className="room-title-button"
        aria-label={`${name}, details`}
        onClick={() => toggle("details")}
      >
        {kind === "direct" && otherId !== undefined ? (
          <UserAvatar userId={otherId} size={32} presence decorative />
        ) : (
          <RoomGlyph kind={kind} iconName={iconName} size={20} className="room-title-icon" />
        )}
        <span className="room-title-text">
          <span className="room-title-name">{name}</span>
          {meta}
        </span>
      </button>
    </h1>
  );
}

/**
 * A phone's ⋯ menu: everything the wide header shows as buttons beside the call button. A DM's
 * people actions, the panes, the calendar, the notification level and search.
 */
function PhoneOverflowItems({
  roomId,
  directActions,
}: {
  readonly roomId: number;
  readonly directActions: ReturnType<typeof useDirectActions>["actions"];
}) {
  const navigate = useNavigate();

  return (
    <>
      {directActions.map((action) => (
        <MenuItem key={action.key} icon={action.icon} onSelect={action.onSelect}>
          {action.label}
        </MenuItem>
      ))}
      {directActions.length === 0 ? null : <MenuSeparator />}
      <PaneMenuItems roomId={roomId} />
      <MenuSeparator />
      <NotificationsSubMenu roomId={roomId} />
      <MenuItem
        icon="calendar"
        onSelect={() => void navigate({ to: "/r/$roomId/events", params: { roomId } })}
      >
        Events
      </MenuItem>
      <MenuItem icon="search" onSelect={() => void navigate({ to: "/search" })}>
        Search
      </MenuItem>
    </>
  );
}

/**
 * The conversation's page header, Slack's: the room's glyph and name (a DM shows the person,
 * their presence and status), then the tools: on wide screens a DM's people actions, the call,
 * the bell, the calendar, the pane toggles (threads, pins, files and the member stack) and
 * search; on phones the back button, the call and a ⋯ menu holding the rest, and the name opens
 * the room's details.
 */
export function RoomHeader({ roomId }: { readonly roomId: number }) {
  const detail = useStore((state) => state.rooms[roomId]?.detail ?? null);
  const row = useStore((state) => state.sidebar.rows[roomId] ?? null);
  const phone = usePhoneLayout();
  const kind = detail?.room.kind ?? row?.room.kind ?? null;
  const name = detail?.displayName ?? row?.displayName ?? null;
  const iconName = detail?.room.iconName ?? row?.room.iconName ?? null;
  const directIds = detail?.directMemberIds ?? row?.directMemberIds ?? [];
  const otherId = kind === "direct" && directIds.length === 1 ? directIds[0] : undefined;
  // A DM's people actions for the phone's ⋯ menu (none in other rooms); wide screens show
  // DirectHeaderActions' buttons instead.
  const direct = useDirectActions(roomId);
  // The classic notification URL opens the bell's menu on wide screens; on phones it opens the ⋯
  // menu holding just the levels, as that menu.
  const notificationsMenu = useNotificationsMenuOpen(roomId);
  const notificationsRoute = useMatchRoute()({ to: "/r/$roomId/notifications" }) !== false;

  const title =
    name === null || kind === null ? (
      <Skeleton width={140} height={14} />
    ) : phone ? (
      <PhoneTitle roomId={roomId} kind={kind} name={name} iconName={iconName} otherId={otherId} />
    ) : (
      <WideTitle roomId={roomId} kind={kind} name={name} iconName={iconName} otherId={otherId} />
    );

  return (
    <>
      <PageHeader
        className="room-header"
        back={{
          label: "Back to conversations",
          link: (props) => <Link to="/" {...props} />,
        }}
        title={title}
        actions={
          phone ? (
            <HuddleLauncher roomId={roomId} />
          ) : (
            <>
              {kind === "direct" ? <DirectHeaderActions roomId={roomId} /> : null}
              <HuddleLauncher roomId={roomId} />
              <NotificationsButton roomId={roomId} />
              {kind === null ? null : <EventsLink roomId={roomId} />}
              <PaneButtons roomId={roomId} />
              <HeaderSearch />
            </>
          )
        }
        overflow={
          !phone || kind === null ? undefined : notificationsRoute ? (
            <NotificationLevels roomId={roomId} />
          ) : (
            <PhoneOverflowItems roomId={roomId} directActions={direct.actions} />
          )
        }
        overflowLabel={phone && notificationsRoute ? "Notifications" : "More"}
        overflowMenu={phone ? notificationsMenu : undefined}
      />
      {phone ? direct.dialogs : null}
    </>
  );
}
