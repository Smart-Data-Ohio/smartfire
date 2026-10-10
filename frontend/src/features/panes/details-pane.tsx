import { Link } from "@tanstack/react-router";
import { type ReactNode, useId } from "react";
import type { RoomKind } from "../../gen/RoomKind.ts";
import { useStore } from "../../store/store.ts";
import { Badge } from "../../ui/badge.tsx";
import { Icon, type IconName } from "../../ui/icons/icon.tsx";
import { Menu } from "../../ui/menu.tsx";
import { useDirectActions } from "../directs/direct-header-actions.tsx";
import { raisedHandsLabel, useRaisedHands } from "../huddle/stage-button.tsx";
import { identityOf } from "../people/agent-identity.ts";
import { useUser } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { RoomGlyph } from "../rooms/room-glyph.tsx";
import { preloadRoomSettings, settingsOverState } from "../rooms/room-settings-host.tsx";
import { RoomTopic } from "../rooms/room-topic.tsx";
import { useNotificationTarget } from "../sidebar/notifications-button.tsx";
import { NotificationItems } from "../sidebar/room-menu.tsx";
import { PaneFrame, RoomName } from "./pane-frame.tsx";
import { PaneListSkeleton } from "./pane-states.tsx";
import { usePaneNavigation } from "./use-right-pane.ts";
import "./details-pane.css";

const KIND_LABELS = {
  open: "Public channel",
  closed: "Private channel",
  direct: "Direct message",
  voice: "Voice room",
  stage: "Stage",
  board: "Board",
} as const satisfies Record<RoomKind, string>;

/** A row's inside: the glyph, the label, a muted value (a count, a level) and a chevron. */
function RowContent({
  icon,
  label,
  value,
}: {
  readonly icon: IconName;
  readonly label: string;
  readonly value?: ReactNode;
}) {
  return (
    <>
      <Icon name={icon} size={20} className="details-row-icon" />
      <span className="details-row-label">{label}</span>
      {value === undefined ? null : <span className="details-row-value">{value}</span>}
      <Icon name="chevron-right" size={16} className="details-row-chevron" />
    </>
  );
}

function PaneRow({
  icon,
  label,
  value,
  onClick,
}: {
  readonly icon: IconName;
  readonly label: string;
  readonly value?: ReactNode;
  readonly onClick: () => void;
}) {
  return (
    <li>
      <button type="button" className="details-row" onClick={onClick}>
        <RowContent icon={icon} label={label} value={value} />
      </button>
    </li>
  );
}

/** The notification level, as a row whose menu changes it (the bell's menu). */
function NotificationsRow({ roomId }: { readonly roomId: number }) {
  const target = useNotificationTarget(roomId);

  if (target === null) {
    return null;
  }

  return (
    <li>
      <Menu
        label="Notifications"
        placement="bottom-end"
        trigger={(props) => (
          <button {...props} type="button" className="details-row">
            <RowContent
              icon={target.choice.icon}
              label="Notifications"
              value={target.choice.label}
            />
          </button>
        )}
      >
        <NotificationItems row={target.row} level={target.level} />
      </Menu>
    </li>
  );
}

/**
 * A 1:1 DM's way to the other side's page, where a wide header's subtitle leads: an agent's
 * profile, or a person's.
 */
function ProfileRow({ userId }: { readonly userId: number }) {
  const identity = identityOf(useUser(userId));

  return (
    <li>
      {identity.kind === "agent" ? (
        <Link
          to="/agents/$agentId"
          params={{ agentId: identity.agentId }}
          className="details-row"
          preload={false}
        >
          <RowContent icon="bot" label="Agent profile" />
        </Link>
      ) : (
        <Link to="/people/$userId" params={{ userId }} className="details-row" preload={false}>
          <RowContent icon="user" label="Profile" />
        </Link>
      )}
    </li>
  );
}

/**
 * A room's details, the screen a phone's room title opens (Slack's channel details, Discord's
 * channel info): who and what it is, then a row per thing about it. Members, threads, pins, files
 * and the stage open their panes over this one (Back returns here); the notification level opens
 * the bell's menu, events the room's calendar, and settings the room's settings dialog. A DM has
 * its people actions instead of members, threads and settings, and a 1:1 DM the other side's
 * profile.
 */
export function DetailsPane({ roomId }: { readonly roomId: number }) {
  const aboutId = useId();
  const detail = useStore((state) => state.rooms[roomId]?.detail ?? null);
  const { push } = usePaneNavigation(roomId);
  const direct = useDirectActions(roomId);
  const hands = useRaisedHands(roomId);

  if (detail === null) {
    return (
      <PaneFrame title="Details">
        <PaneListSkeleton rows={6} />
      </PaneFrame>
    );
  }

  const kind = detail.room.kind;
  const isDirect = kind === "direct";

  const otherId =
    isDirect && detail.directMemberIds.length === 1 ? detail.directMemberIds[0] : undefined;

  const members = `${detail.memberCount} ${detail.memberCount === 1 ? "member" : "members"}`;
  const pins = detail.pinsCount;

  return (
    <PaneFrame title="Details" subtitle={<RoomName roomId={roomId} />}>
      <div className="details">
        <div className="details-hero">
          {otherId === undefined ? (
            <span className="details-glyph">
              <RoomGlyph kind={kind} iconName={detail.room.iconName} size={28} />
            </span>
          ) : (
            <UserAvatar userId={otherId} size={64} presence decorative />
          )}
          <h3 className="details-name">{detail.displayName}</h3>
          <p className="details-kind">
            {isDirect ? KIND_LABELS[kind] : `${KIND_LABELS[kind]} · ${members}`}
          </p>
        </div>
        {isDirect || detail.room.topic === null ? null : (
          <section className="details-about" aria-labelledby={aboutId}>
            <h4 id={aboutId}>About</h4>
            <p className="details-topic">
              <RoomTopic topic={detail.room.topic} />
            </p>
          </section>
        )}
        <ul className="details-rows" aria-label="About this conversation">
          {otherId === undefined ? null : <ProfileRow userId={otherId} />}
          <NotificationsRow roomId={roomId} />
          {kind === "stage" ? (
            <PaneRow
              icon="radio"
              label="Stage"
              value={
                hands > 0 ? <Badge count={hands} label={raisedHandsLabel(hands)} /> : undefined
              }
              onClick={() => push("stage")}
            />
          ) : null}
          {isDirect ? null : (
            <PaneRow
              icon="users"
              label="Members"
              value={detail.memberCount}
              onClick={() => push("members")}
            />
          )}
          {isDirect ? null : (
            <PaneRow icon="thread" label="Threads" onClick={() => push("threads")} />
          )}
          <PaneRow
            icon="pin"
            label="Pinned messages"
            value={pins > 0 ? pins : undefined}
            onClick={() => push("pins")}
          />
          <PaneRow icon="file" label="Files" onClick={() => push("files")} />
          <li>
            <Link to="/r/$roomId/events" params={{ roomId }} className="details-row">
              <RowContent icon="calendar" label="Events" />
            </Link>
          </li>
        </ul>
        {direct.actions.length === 0 && isDirect ? null : (
          <ul className="details-rows" aria-label="Manage this conversation">
            {direct.actions.map((action) => (
              <PaneRow
                key={action.key}
                icon={action.icon}
                label={action.label}
                onClick={action.onSelect}
              />
            ))}
            {isDirect ? null : (
              <li>
                <Link
                  to="/r/$roomId/settings"
                  params={{ roomId }}
                  state={settingsOverState(roomId)}
                  className="details-row"
                  onPointerEnter={preloadRoomSettings}
                  onFocus={preloadRoomSettings}
                >
                  <RowContent
                    icon="settings"
                    label={kind === "open" || kind === "closed" ? "Channel settings" : "Settings"}
                  />
                </Link>
              </li>
            )}
          </ul>
        )}
        {direct.dialogs}
      </div>
    </PaneFrame>
  );
}
