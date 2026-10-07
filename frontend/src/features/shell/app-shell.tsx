import { Outlet, useMatches, useMatchRoute, useParams } from "@tanstack/react-router";
import { useEffect } from "react";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Toaster } from "../../ui/toast.tsx";
import { HuddleDock } from "../huddle/huddle-dock.tsx";
import { HuddleRoot } from "../huddle/huddle-root.tsx";
import { SearchHotkey } from "../search/search-hotkey.tsx";
import { sidebarTotals } from "../sidebar/sections.ts";
import { Sidebar } from "../sidebar/sidebar.tsx";
import { GlobalOverlays } from "../switcher/global-overlays.tsx";
import { useAppBadge } from "./app-badge.ts";
import { useBootFlash } from "./boot-flash.ts";
import { useClassicLinks } from "./classic-links.ts";
import { ConnectionBanner } from "./connection-banner.tsx";
import { Rail } from "./rail.tsx";
import "./app-shell.css";

/** A full-column page that isn't a conversation: its tab title, or `null` for a room. */
type Page = "Settings" | "Workspace" | "People" | null;

/**
 * "(3) #general · Smartfire": unread mentions first, as Slack's tab title does. Settings, the
 * workspace pages and the people pages name themselves instead of a room.
 */
function useDocumentTitle(roomId: number | null, page: Page): void {
  const mentions = useStore((state) => sidebarTotals(state.sidebar).mentions);
  const unread = useStore((state) => sidebarTotals(state.sidebar).unreadRooms > 0);
  const room = useStore((state) => (roomId === null ? null : (state.sidebar.rows[roomId] ?? null)));

  useEffect(() => {
    const prefix = mentions > 0 ? `(${mentions}) ` : unread ? "• " : "";

    const name =
      page !== null
        ? `${page} · Smartfire`
        : room === null
          ? "Smartfire"
          : `${room.room.kind === "direct" ? "" : "#"}${room.displayName} · Smartfire`;

    document.title = `${prefix}${name}`;
  }, [mentions, unread, room, page]);
}

/**
 * Which column a phone shows: the conversation list, a tab's page beside the tab bar (the
 * activity inbox), or a pushed full screen (a conversation, Saved, Scheduled, Search).
 */
function usePhoneView(roomId: number | null): "list" | "tab" | "room" {
  const matchRoute = useMatchRoute();

  const pushed =
    matchRoute({ to: "/saved" }) !== false ||
    matchRoute({ to: "/scheduled" }) !== false ||
    matchRoute({ to: "/search" }) !== false ||
    matchRoute({ to: "/m/$messageId" }) !== false;

  if (roomId !== null || pushed) {
    return "room";
  }

  return matchRoute({ to: "/activity" }) === false ? "list" : "tab";
}

/**
 * The signed-in app: rail, sidebar and the routed pane. Starts the sync engine once. On phones
 * only one column shows (`data-view`): the conversation list, a tab page, or a full screen
 * (settings and the workspace pages are full screens too).
 */
export function AppShell() {
  const params = useParams({ strict: false });
  const roomId = params.roomId ?? null;

  // Settings, the workspace pages and the people pages fill the main column, so phones show them
  // rather than the conversation list.
  const page = useMatches({
    select: (matches): Page =>
      matches.some((match) => match.routeId.startsWith("/shell/settings"))
        ? "Settings"
        : matches.some((match) => match.routeId.startsWith("/shell/admin"))
          ? "Workspace"
          : matches.some((match) => match.routeId.startsWith("/shell/people"))
            ? "People"
            : null,
  });

  useEffect(() => {
    void actions.start();
  }, []);

  useDocumentTitle(roomId, page);
  useAppBadge();
  useClassicLinks();
  useBootFlash();

  const view = usePhoneView(roomId);

  const viewerId = useStore((state) => state.me?.user.id ?? state.boot?.user.id ?? null);

  return (
    <div className="app-shell" data-view={page === null ? view : "room"}>
      <Rail />
      <Sidebar />
      {viewerId === null ? null : (
        // Mentions of you are amber (Slack's); the server tags each chip with its user's id.
        <style>{`.message-body .mention--user-${viewerId} .profile-card-name{background:var(--mention-chip-bg);color:var(--mention-text)}`}</style>
      )}
      <main className="app-main">
        <ConnectionBanner />
        <div className="app-main-dock">
          <HuddleDock compact />
        </div>
        <Outlet />
      </main>
      <HuddleRoot />
      <GlobalOverlays />
      <SearchHotkey />
      <Toaster />
    </div>
  );
}
