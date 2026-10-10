import { useNavigate } from "@tanstack/react-router";
import type { ReactElement } from "react";
import type { Placement } from "../../lib/anchor.ts";
import { shortcutKeys } from "../../lib/shortcuts.ts";
import { Menu, MenuGroup, MenuItem, MenuSeparator, type MenuTriggerProps } from "../../ui/menu.tsx";
import { restartTour } from "../help/tour.tsx";
import { openOverlay } from "../switcher/overlay-store.ts";

/**
 * The signed-in person's menu, opened from their panel at the foot of the sidebar or, on phones,
 * the tab bar's You tab (`trigger` renders either): their profile and settings, the people
 * directory, the workspace (its people, and for administrators the rest of the classic account
 * pages), and classic's help menu (the keyboard shortcuts and restarting the product tour).
 */
export function UserMenu({
  trigger,
  placement = "top-start",
}: {
  readonly trigger: (props: MenuTriggerProps) => ReactElement;
  readonly placement?: Placement;
}) {
  const navigate = useNavigate();

  return (
    <Menu label="Your account" placement={placement} trigger={trigger}>
      <MenuItem icon="settings" onSelect={() => void navigate({ to: "/settings" })}>
        Profile and settings
      </MenuItem>
      <MenuItem icon="users" onSelect={() => void navigate({ to: "/people" })}>
        People
      </MenuItem>
      <MenuItem icon="home" onSelect={() => void navigate({ to: "/admin" })}>
        Workspace and people
      </MenuItem>
      <MenuSeparator />
      <MenuGroup label="Help">
        <MenuItem
          icon="keyboard"
          shortcut={shortcutKeys("shortcuts")}
          onSelect={() => openOverlay("shortcuts")}
        >
          Keyboard shortcuts
        </MenuItem>
        <MenuItem icon="refresh-cw" onSelect={restartTour}>
          Restart tour
        </MenuItem>
      </MenuGroup>
    </Menu>
  );
}
