import { Link, useMatchRoute } from "@tanstack/react-router";
import type { IconName } from "../../ui/icons/icon.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import "./destinations.css";

interface DestinationLinkProps {
  readonly to: "/work" | "/saved" | "/scheduled";
  readonly icon: IconName;
  readonly label: string;
}

function DestinationLink({ to, icon, label }: DestinationLinkProps) {
  const matchRoute = useMatchRoute();
  const selected = matchRoute({ to }) !== false;

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
 * Work, Saved and Scheduled, at the top of the sidebar as Slack keeps "Later" and "Scheduled"
 * (the classic workspace list's order).
 */
export function SidebarDestinations() {
  return (
    <ul className="sidebar-destinations" aria-label="Your workspace">
      <DestinationLink to="/work" icon="list-checks" label="Work" />
      <DestinationLink to="/saved" icon="bookmark" label="Saved" />
      <DestinationLink to="/scheduled" icon="clock" label="Scheduled" />
    </ul>
  );
}
