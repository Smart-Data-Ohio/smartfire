import { Link, useMatchRoute } from "@tanstack/react-router";
import type { IconName } from "../../ui/icons/icon.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { useWelcome } from "../welcome/welcome-store.ts";
import "./destinations.css";

interface DestinationLinkProps {
  readonly to: "/guide" | "/saved" | "/scheduled" | "/work" | "/agents";
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
 * The server guide first, when the administrators have written one (Discord keeps it at the top
 * of the channel list); then Saved and Scheduled, as Slack keeps "Later" and "Scheduled", then
 * Work (every tracked thread) and the Agents directory.
 */
export function SidebarDestinations() {
  const { welcome } = useWelcome();
  const guide = (welcome?.guide.length ?? 0) > 0;

  return (
    <ul className="sidebar-destinations" aria-label="Destinations">
      {guide ? <DestinationLink to="/guide" icon="map-pin" label="Server guide" /> : null}
      <DestinationLink to="/saved" icon="bookmark" label="Saved" />
      <DestinationLink to="/scheduled" icon="clock" label="Scheduled" />
      <DestinationLink to="/work" icon="briefcase" label="Work" />
      <DestinationLink to="/agents" icon="bot" label="Agents" />
    </ul>
  );
}
