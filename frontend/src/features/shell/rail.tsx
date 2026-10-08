import { Link, useMatchRoute, useNavigate } from "@tanstack/react-router";
import { type ReactNode, useState } from "react";
import { useReducedMotion } from "../../motion/reduced-motion.ts";
import { useActivityUnread } from "../../store/inbox-hooks.ts";
import { organizedSidebar } from "../../store/organize.ts";
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

interface WorkspaceTileProps {
  readonly name: string;
  readonly logoUrl: string | null;
  /** An animated logo's first frame, shown at rest; `null` for a still logo. */
  readonly stillUrl: string | null;
}

/**
 * The workspace's tile at the top of the rail: its logo when one is uploaded (and loads), else
 * its initials. Round at rest, it squares off under the pointer, as a Discord server does. An
 * animated logo rests on its first frame and plays only while pointed at or focused (never under
 * reduced motion), starting from the top each time.
 */
function WorkspaceTile({ name, logoUrl, stillUrl }: WorkspaceTileProps) {
  const [broken, setBroken] = useState<string | null>(null);
  const [live, setLive] = useState(false);
  const reduced = useReducedMotion();

  const rest = stillUrl ?? logoUrl;
  const logo = rest !== null && rest !== broken ? rest : null;
  const playing = live && !reduced && stillUrl !== null && logoUrl !== null && logoUrl !== broken;

  return (
    <Tooltip content={name} placement="right" describe={false}>
      <Link
        to="/"
        className="rail-workspace"
        aria-label={name}
        data-logo={logo !== null || undefined}
        onPointerEnter={() => setLive(true)}
        onPointerLeave={() => setLive(false)}
        onFocus={() => setLive(true)}
        onBlur={() => setLive(false)}
      >
        {logo === null ? (
          initialsOf(name)
        ) : (
          <img
            className="rail-workspace-logo"
            src={logo}
            alt=""
            width={40}
            height={40}
            draggable={false}
            onError={() => setBroken(logo)}
          />
        )}
        {playing ? (
          <img
            className="rail-workspace-logo"
            data-animated=""
            src={logoUrl}
            alt=""
            width={40}
            height={40}
            draggable={false}
            onError={() => setBroken(logoUrl)}
          />
        ) : null}
      </Link>
    </Tooltip>
  );
}

/** Unread direct messages across the sidebar, for the DMs destination. */
function useDirectUnread(): number {
  return useStore((state) => {
    // Through pending changes, like the other totals: a DM being muted or hidden stops counting.
    const view = organizedSidebar(state.sidebar);
    let total = 0;

    for (const roomId of view.order) {
      const row = view.rows[roomId];

      if (
        row !== undefined &&
        row.room.kind === "direct" &&
        row.membership.involvement !== "invisible"
      ) {
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
  const logoUrl = useStore((state) => state.boot?.account.logoUrl ?? null);
  const logoStillUrl = useStore((state) => state.boot?.account.logoStillUrl ?? null);
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
      <WorkspaceTile name={accountName} logoUrl={logoUrl} stillUrl={logoStillUrl} />
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
