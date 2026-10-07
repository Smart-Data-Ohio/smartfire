import { organizedSidebar } from "../../store/organize.ts";
import { useStore } from "../../store/store.ts";
import { IconButton } from "../../ui/icon-button.tsx";
import { Menu } from "../../ui/menu.tsx";
import { involvementChoice } from "./organize-model.ts";
import { NotificationItems } from "./room-menu.tsx";

/**
 * The room header's bell: the viewer's notification level for this room, as an icon, and a menu
 * to change it. A pending change shows at once; a hidden room (no sidebar row) reads the header's
 * own copy of the membership.
 */
export function NotificationsButton({ roomId }: { readonly roomId: number }) {
  const row = useStore((state) => organizedSidebar(state.sidebar).rows[roomId]);
  const detail = useStore((state) => state.rooms[roomId]?.detail ?? null);
  const room = row?.room ?? detail?.room;
  const level = row?.membership.involvement ?? detail?.membership.involvement;

  if (room === undefined || level === undefined) {
    return null;
  }

  const choice = involvementChoice(level);
  const displayName = row?.displayName ?? detail?.displayName ?? "";

  return (
    <Menu
      label="Notifications"
      placement="bottom-end"
      trigger={(props) => (
        <IconButton
          {...props}
          icon={choice.icon}
          label={`Notifications: ${choice.label}`}
          tooltipPlacement="bottom"
          className="pane-button room-notifications"
          data-level={level}
        />
      )}
    >
      <NotificationItems row={{ room, displayName }} level={level} />
    </Menu>
  );
}
