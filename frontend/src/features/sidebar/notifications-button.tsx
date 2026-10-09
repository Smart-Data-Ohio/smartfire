import { useMatchRoute, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import type { SidebarRow } from "../../store/model.ts";
import { organizedSidebar } from "../../store/organize.ts";
import { useStore } from "../../store/store.ts";
import { IconButton } from "../../ui/icon-button.tsx";
import { Menu, SubMenu } from "../../ui/menu.tsx";
import { type InvolvementChoice, involvementChoice } from "./organize-model.ts";
import { NotificationItems } from "./room-menu.tsx";

/** A room's notification level, with what `NotificationItems` needs to change it. */
export interface NotificationTarget {
  readonly row: Pick<SidebarRow, "room" | "displayName">;
  readonly level: SidebarRow["membership"]["involvement"];
  readonly choice: InvolvementChoice;
}

/**
 * The viewer's notification level for this room. A pending change shows at once; a hidden room
 * (no sidebar row) reads the header's own copy of the membership.
 */
export function useNotificationTarget(roomId: number): NotificationTarget | null {
  const row = useStore((state) => organizedSidebar(state.sidebar).rows[roomId]);
  const detail = useStore((state) => state.rooms[roomId]?.detail ?? null);
  const room = row?.room ?? detail?.room;
  const level = row?.membership.involvement ?? detail?.membership.involvement;

  if (room === undefined || level === undefined) {
    return null;
  }

  const displayName = row?.displayName ?? detail?.displayName ?? "";

  return { row: { room, displayName }, level, choice: involvementChoice(level) };
}

/** A controlled menu's state: `Menu`'s `open` and `onOpenChange`. */
export interface MenuOpenState {
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}

/**
 * The open state of the menu holding a room's notification levels: open on the classic page's
 * URL (`/r/$roomId/notifications`), and closing it there goes back to the room.
 */
export function useNotificationsMenuOpen(roomId: number): MenuOpenState {
  const matchRoute = useMatchRoute();
  const navigate = useNavigate();
  const routeOpen = matchRoute({ to: "/r/$roomId/notifications" }) !== false;
  const [localOpen, setLocalOpen] = useState(false);

  return {
    open: routeOpen || localOpen,
    onOpenChange: (open) => {
      setLocalOpen(open && !routeOpen);

      if (!open && routeOpen) {
        void navigate({ to: "/r/$roomId", params: { roomId } });
      }
    },
  };
}

/** The room header's bell: the viewer's notification level for this room, and a menu to change it. */
export function NotificationsButton({ roomId }: { readonly roomId: number }) {
  const target = useNotificationTarget(roomId);
  const menu = useNotificationsMenuOpen(roomId);

  if (target === null) {
    return null;
  }

  return (
    <Menu
      label="Notifications"
      placement="bottom-end"
      open={menu.open}
      onOpenChange={menu.onOpenChange}
      trigger={(props) => (
        <IconButton
          {...props}
          icon={target.choice.icon}
          label={`Notifications: ${target.choice.label}`}
          tooltipPlacement="bottom"
          className="pane-button room-notifications"
          data-level={target.level}
        />
      )}
    >
      <NotificationItems row={target.row} level={target.level} />
    </Menu>
  );
}

/** The same levels as a submenu, for a menu that holds the bell (a phone's ⋯ menu). */
export function NotificationsSubMenu({ roomId }: { readonly roomId: number }) {
  const target = useNotificationTarget(roomId);

  return target === null ? null : (
    <SubMenu label="Notifications" icon={target.choice.icon}>
      <NotificationItems row={target.row} level={target.level} />
    </SubMenu>
  );
}

/**
 * The levels alone, for a menu the classic notification URL opens in place of its usual items (a
 * phone's ⋯ menu, which has no bell to open).
 */
export function NotificationLevels({ roomId }: { readonly roomId: number }) {
  const target = useNotificationTarget(roomId);

  return target === null ? null : <NotificationItems row={target.row} level={target.level} />;
}
