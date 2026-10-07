import { Link, useMatchRoute } from "@tanstack/react-router";
import type { IconName } from "../../ui/icons/icon.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import "./destinations.css";

interface DestinationLinkProps {
  readonly to: "/saved" | "/scheduled" | "/agents";
  readonly icon: IconName;
  readonly label: string;
}

function DestinationLink({ to, icon, label }: DestinationLinkProps) {
  const matchRoute = useMatchRoute();
  // Fuzzy, so an agent's profile keeps Agents selected.
  const selected = matchRoute({ to, fuzzy: true }) !== false;

  return (
    <li className="sidebar-destination">
      <Link
        to={to}
        className="sidebar-row"
        data-state={selected ? "selected" : undefined}
        aria-current={selected ? "page" : undefined}
        preload={false}
      >
        <span className="sidebar-row-glyph">
          <Icon name={icon} size={16} className="sidebar-row-icon" />
        </span>
        <span className="sidebar-row-name">{label}</span>
      </Link>
    </li>
  );
}

/**
 * Saved and Scheduled, at the top of the sidebar as Slack keeps "Later" and "Scheduled", then the
 * Agents directory.
 */
export function SidebarDestinations() {
  return (
    <ul className="sidebar-destinations" aria-label="Destinations">
      <DestinationLink to="/saved" icon="bookmark" label="Saved" />
      <DestinationLink to="/scheduled" icon="clock" label="Scheduled" />
      <DestinationLink to="/agents" icon="bot" label="Agents" />
    </ul>
  );
}
