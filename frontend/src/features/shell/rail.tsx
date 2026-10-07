import { Link, useMatchRoute, useNavigate } from "@tanstack/react-router";
import type { ReactNode } from "react";
import { useActivityUnread } from "../../store/inbox-hooks.ts";
import { useStore } from "../../store/store.ts";
import { Badge } from "../../ui/badge.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Tooltip } from "../../ui/tooltip.tsx";
import { rowPillCount, sidebarTotals } from "../sidebar/sections.ts";
import { type Destination, setDestination, useDestination } from "./view-store.ts";

interface RailItemProps {
  readonly label: string;
  readonly active: boolean;
  readonly unread?: boolean;
  readonly count?: number;
  readonly onSelect: () => void;
  readonly children: ReactNode;
}

function RailItem({ label, active, unread = false, count = 0, onSelect, children }: RailItemProps) {
  return (
    <div className="rail-item" data-active={active || undefined} data-unread={unread || undefined}>
      <span className="rail-pill" aria-hidden="true" />
      <Tooltip content={label} placement="right" describe={false}>
        <button
          type="button"
          className="rail-button"
          aria-label={label}
          aria-pressed={active}
          onClick={onSelect}
        >
          {children}
          <Badge count={count} floating label={`${count} unread`} />
        </button>
      </Tooltip>
      <span className="rail-caption" aria-hidden="true">
        {label}
      </span>
    </div>
  );
}

function initialsOf(name: string): string {
  const words = name.trim().split(/\s+/);

  return words
    .slice(0, 2)
    .map((word) => [...word][0] ?? "")
    .join("")
    .toUpperCase();
}

/** Unread direct messages across the sidebar, for the DMs destination. */
function useDirectUnread(): number {
  return useStore((state) => {
    let total = 0;

    for (const roomId of state.sidebar.order) {
      const row = state.sidebar.rows[roomId];

      if (row !== undefined && row.room.kind === "direct") {
        total += rowPillCount(row);
      }
    }

    return total;
  });
}

/**
 * The workspace rail: Discord's spatial model. The workspace tile, then the destinations, each
 * with Discord's pill (a nub when unread, taller on hover, full when selected). On phones the
 * same items become the bottom tab bar.
 */
export function Rail() {
  const destination = useDestination();
  const accountName = useStore((state) => state.boot?.account.name ?? "Smartfire");
  const unreadRooms = useStore((state) => sidebarTotals(state.sidebar).unreadRooms);
  const mentions = useStore((state) => sidebarTotals(state.sidebar).mentions);
  const directUnread = useDirectUnread();
  const activityUnread = useActivityUnread() ?? 0;
  const navigate = useNavigate();
  const matchRoute = useMatchRoute();
  const onActivity = matchRoute({ to: "/activity" }) !== false;

  // From the inbox, Home and DMs lead back to the conversations.
  const select = (next: Destination) => () => {
    setDestination(next);

    if (onActivity) {
      void navigate({ to: "/" });
    }
  };

  return (
    <nav className="rail" aria-label="Destinations">
      <Tooltip content={accountName} placement="right" describe={false}>
        <Link to="/" className="rail-workspace" aria-label={accountName}>
          {initialsOf(accountName)}
        </Link>
      </Tooltip>
      <span className="rail-divider" aria-hidden="true" />
      <RailItem
        label="Home"
        active={!onActivity && destination === "home"}
        unread={unreadRooms > 0}
        count={mentions - directUnread}
        onSelect={select("home")}
      >
        <Icon name="home" size={20} />
      </RailItem>
      <RailItem
        label="DMs"
        active={!onActivity && destination === "dms"}
        unread={directUnread > 0}
        count={directUnread}
        onSelect={select("dms")}
      >
        <Icon name="dms" size={20} />
      </RailItem>
      <RailItem
        label="Activity"
        active={onActivity}
        unread={activityUnread > 0}
        count={activityUnread}
        onSelect={() => void navigate({ to: "/activity" })}
      >
        <Icon name="inbox" size={20} />
      </RailItem>
    </nav>
  );
}
