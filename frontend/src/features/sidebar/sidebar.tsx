import { useParams } from "@tanstack/react-router";
import { type ReactNode, useId, useState } from "react";
import { setTheme, useAppearance } from "../../lib/appearance.ts";
import { shortcutKeys } from "../../lib/shortcuts.ts";
import { useStore } from "../../store/store.ts";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { ariaKeyShortcuts, Kbd } from "../../ui/kbd.tsx";
import { Menu, MenuItem, MenuSeparator } from "../../ui/menu.tsx";
import { Skeleton, SkeletonReveal } from "../../ui/skeleton.tsx";
import { SidebarDestinations } from "../destinations/sidebar-destinations.tsx";
import { HuddleDock } from "../huddle/huddle-dock.tsx";
import { UNKNOWN_NAME } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { useDestination } from "../shell/view-store.ts";
import { openOverlay } from "../switcher/overlay-store.ts";
import { type SidebarSection, sidebarSections } from "./sections.ts";
import { SidebarRow } from "./sidebar-row.tsx";
import "./sidebar.css";

const COLLAPSED_KEY = "smartfire.sidebar.collapsed";

function readCollapsed(): ReadonlySet<string> {
  try {
    const saved = localStorage.getItem(COLLAPSED_KEY);

    return new Set(saved === null ? [] : saved.split(","));
  } catch {
    return new Set();
  }
}

function writeCollapsed(keys: ReadonlySet<string>): void {
  try {
    localStorage.setItem(COLLAPSED_KEY, [...keys].join(","));
  } catch {
    // Unavailable storage only means the choice lasts for this page.
  }
}

interface SectionProps {
  readonly section: SidebarSection;
  readonly open: boolean;
  readonly selectedRoomId: number | null;
  readonly onToggle: () => void;
  /** A button at the heading's end (Direct messages' "+"). */
  readonly action?: ReactNode;
  /** What an empty open section says, or offers. */
  readonly empty: ReactNode;
}

/**
 * A collapsible group (the transitions.dev accordion). Collapsed, it still lists its unread rows
 * and the open conversation, as Slack does, so nothing new hides behind a chevron.
 */
function Section({ section, open, selectedRoomId, onToggle, action, empty }: SectionProps) {
  const id = useId();

  const peeking = open
    ? []
    : section.rows.filter(
        (row) => row.room.id === selectedRoomId || row.membership.unreadAt !== null,
      );

  return (
    <div className="sidebar-section t-acc" data-open={open}>
      <div className="sidebar-section-heading">
        <h2 className="sidebar-section-title">
          <button
            type="button"
            id={`${id}-trigger`}
            className="sidebar-section-trigger"
            aria-expanded={open}
            aria-controls={`${id}-panel`}
            onClick={onToggle}
          >
            <span className="t-acc-chevron">
              <Icon name="chevron-down" size={12} />
            </span>
            {section.title}
          </button>
        </h2>
        {action}
      </div>
      <section
        id={`${id}-panel`}
        className="t-acc-panel"
        aria-labelledby={`${id}-trigger`}
        inert={!open}
      >
        <div className="t-acc-panel-inner">
          <ul className="sidebar-rows">
            {section.rows.map((row) => (
              <SidebarRow key={row.room.id} row={row} selected={row.room.id === selectedRoomId} />
            ))}
            {section.rows.length === 0 ? <li className="sidebar-empty">{empty}</li> : null}
          </ul>
        </div>
      </section>
      {peeking.length > 0 ? (
        <ul className="sidebar-rows">
          {peeking.map((row) => (
            <SidebarRow key={row.room.id} row={row} selected={row.room.id === selectedRoomId} />
          ))}
        </ul>
      ) : null}
    </div>
  );
}

/** The first-load placeholder: two sections' worth of rows, shaped like the real ones. */
function SidebarSkeleton() {
  const group = (key: string, widths: readonly number[], avatar: boolean) => (
    <div key={key} className="sidebar-skeleton-group">
      <Skeleton width={72} height={8} />
      {widths.map((width) => (
        // The widths are distinct within a group, so each one names its row.
        <span key={width} className="sidebar-skeleton-row">
          <Skeleton
            width={avatar ? 20 : 14}
            height={avatar ? 20 : 14}
            radius={avatar ? "md" : "sm"}
          />
          <Skeleton width={`${width}%`} height={9} />
        </span>
      ))}
    </div>
  );

  return (
    <div className="sidebar-skeleton">
      {group("channels", [62, 48, 70, 54, 40], false)}
      {group("direct", [58, 44, 66], true)}
    </div>
  );
}

/** The workspace header: the account's menu (shortcuts live there) and a new-message button. */
function WorkspaceHeader({ title }: { readonly title: string }) {
  return (
    <header className="sidebar-header">
      <Menu
        label={`${title} menu`}
        trigger={(props) => (
          <Button
            {...props}
            variant="ghost"
            size="sm"
            trailingIcon="chevron-down"
            className="sidebar-workspace"
          >
            <span className="sidebar-workspace-name">{title}</span>
          </Button>
        )}
      >
        <MenuItem
          icon="square-pen"
          shortcut={shortcutKeys("new-direct")}
          onSelect={() => openOverlay("new-direct")}
        >
          New message
        </MenuItem>
        <MenuItem
          icon="search"
          shortcut={shortcutKeys("switcher")}
          onSelect={() => openOverlay("switcher")}
        >
          Jump to…
        </MenuItem>
        <MenuSeparator />
        <MenuItem
          icon="keyboard"
          shortcut={shortcutKeys("shortcuts")}
          onSelect={() => openOverlay("shortcuts")}
        >
          Keyboard shortcuts
        </MenuItem>
      </Menu>
      <IconButton
        icon="square-pen"
        label="New message"
        shortcut={shortcutKeys("new-direct")}
        tooltipPlacement="bottom"
        className="sidebar-compose"
        onClick={() => openOverlay("new-direct")}
      />
    </header>
  );
}

/** The search bar that opens the quick switcher, with its shortcut, as Slack's top bar does. */
function JumpButton() {
  return (
    <div className="sidebar-jump-wrap">
      <Button
        variant="secondary"
        size="sm"
        icon="search"
        className="sidebar-jump"
        aria-keyshortcuts={ariaKeyShortcuts(shortcutKeys("switcher"))}
        onClick={() => openOverlay("switcher")}
      >
        <span className="sidebar-jump-label">Jump to…</span>
        <Kbd keys={shortcutKeys("switcher")} className="sidebar-jump-kbd" />
      </Button>
    </div>
  );
}

const THEME_NEXT = { system: "light", light: "dark", dark: "system" } as const;

const THEME_ICON = { system: "monitor", light: "sun", dark: "moon" } as const;

/** Discord's user panel: who you are, your presence, and the appearance switch. */
function YouPanel() {
  const me = useStore((state) => state.me);
  const bootUser = useStore((state) => state.boot?.user ?? null);
  const status = useStore((state) => (me === null ? null : (state.presence[me.user.id] ?? null)));
  const { theme } = useAppearance();
  const userId = me?.user.id ?? bootUser?.id;

  if (userId === undefined) {
    return null;
  }

  return (
    <footer className="sidebar-you">
      <UserAvatar userId={userId} size={32} presence decorative />
      <span className="sidebar-you-text">
        <span className="sidebar-you-name">{me?.user.name ?? bootUser?.name ?? UNKNOWN_NAME}</span>
        <span className="sidebar-you-status">{status?.statusText ?? "Active"}</span>
      </span>
      <IconButton
        icon={THEME_ICON[theme]}
        label={`Theme: ${theme}`}
        size="sm"
        onClick={() => setTheme(THEME_NEXT[theme])}
      />
    </footer>
  );
}

/**
 * The conversation list: the workspace header, then Favourites, your categories, Channels, Voice
 * and Direct messages, then your own panel. The header's menu, the compose button, the jump bar
 * and Direct messages' "+" open the switcher, shortcuts and new-message overlays.
 */
export function Sidebar() {
  const sidebar = useStore((state) => state.sidebar);
  const accountName = useStore((state) => state.boot?.account.name ?? null);
  const params = useParams({ strict: false });
  const [collapsed, setCollapsed] = useState(readCollapsed);
  const selectedRoomId = params.roomId ?? null;
  const destination = useDestination();
  const all = sidebarSections(sidebar);
  const sections = destination === "dms" ? all.filter((section) => section.key === "direct") : all;

  // A category the server keeps collapsed opens with an "open:" override; everything else
  // collapses with its own key.
  const overrideKey = (section: SidebarSection) =>
    section.collapsed ? `open:${section.key}` : section.key;

  const isOpen = (section: SidebarSection) =>
    collapsed.has(overrideKey(section)) === section.collapsed;

  const toggle = (section: SidebarSection) => {
    const next = new Set(collapsed);
    const key = overrideKey(section);

    if (next.has(key)) {
      next.delete(key);
    } else {
      next.add(key);
    }

    setCollapsed(next);
    writeCollapsed(next);
  };

  const newMessage = (
    <IconButton
      icon="plus"
      label="New message"
      shortcut={shortcutKeys("new-direct")}
      size="sm"
      className="sidebar-section-action"
      onClick={() => openOverlay("new-direct")}
    />
  );

  const emptyFor = (section: SidebarSection) =>
    section.key === "direct" ? (
      <Button
        variant="ghost"
        size="sm"
        icon="plus"
        className="sidebar-empty-action"
        onClick={() => openOverlay("new-direct")}
      >
        Start a conversation
      </Button>
    ) : (
      <span className="text-faint">No channels yet</span>
    );

  return (
    <aside className="sidebar" aria-label="Conversations">
      <WorkspaceHeader
        title={destination === "dms" ? "Direct messages" : (accountName ?? "Smartfire")}
      />
      <JumpButton />
      <div className="sidebar-scroll">
        <SidebarDestinations />
        {sidebar.status === "error" ? (
          <p className="sidebar-error text-meta">Couldn't load your conversations.</p>
        ) : (
          <SkeletonReveal loading={sidebar.status !== "ready"} skeleton={<SidebarSkeleton />}>
            {sections.map((section) => (
              <Section
                key={section.key}
                section={section}
                open={isOpen(section)}
                selectedRoomId={selectedRoomId}
                onToggle={() => toggle(section)}
                action={section.key === "direct" ? newMessage : undefined}
                empty={emptyFor(section)}
              />
            ))}
          </SkeletonReveal>
        )}
      </div>
      <HuddleDock />
      <YouPanel />
    </aside>
  );
}
