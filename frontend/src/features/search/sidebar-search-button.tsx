import { useNavigate } from "@tanstack/react-router";
import { shortcutKeys } from "../../lib/shortcuts.ts";
import { IconButton } from "../../ui/icon-button.tsx";

/** The sidebar header's search button: opens the search page (a phone's way in to search). */
export function SidebarSearchButton() {
  const navigate = useNavigate();

  return (
    <IconButton
      icon="search"
      label="Search messages"
      shortcut={shortcutKeys("search")}
      tooltipPlacement="bottom"
      className="sidebar-search"
      onClick={() => void navigate({ to: "/search" })}
    />
  );
}
