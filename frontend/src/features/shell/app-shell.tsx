import { Outlet, useMatches, useParams } from "@tanstack/react-router";
import { useEffect } from "react";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Toaster } from "../../ui/toast.tsx";
import { sidebarTotals } from "../sidebar/sections.ts";
import { Sidebar } from "../sidebar/sidebar.tsx";
import { GlobalOverlays } from "../switcher/global-overlays.tsx";
import { useClassicLinks } from "./classic-links.ts";
import { ConnectionBanner } from "./connection-banner.tsx";
import { Rail } from "./rail.tsx";
import "./app-shell.css";

/** A full-column page that isn't a conversation: its tab title, or `null` for a room. */
type Page = "Settings" | "Workspace" | null;

/**
 * "(3) #general · Smartfire": unread mentions first, as Slack's tab title does. Settings and the
 * workspace pages name themselves instead of a room.
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
 * The signed-in app: rail, sidebar and the routed pane. Starts the sync engine once. On phones
 * only one column shows: the conversation list, or the open conversation or settings (`data-view`).
 */
export function AppShell() {
  const params = useParams({ strict: false });
  const roomId = params.roomId ?? null;

  // Settings and the workspace pages fill the main column, so phones show them rather than the
  // conversation list.
  const page = useMatches({
    select: (matches): Page =>
      matches.some((match) => match.routeId.startsWith("/shell/settings"))
        ? "Settings"
        : matches.some((match) => match.routeId.startsWith("/shell/admin"))
          ? "Workspace"
          : null,
  });

  useEffect(() => {
    void actions.start();
  }, []);

  useDocumentTitle(roomId, page);
  useClassicLinks();

  const viewerId = useStore((state) => state.me?.user.id ?? state.boot?.user.id ?? null);

  return (
    <div className="app-shell" data-view={roomId === null && page === null ? "list" : "room"}>
      <Rail />
      <Sidebar />
      {viewerId === null ? null : (
        // Mentions of you are amber (Slack's); the server tags each chip with its user's id.
        <style>{`.message-body .mention--user-${viewerId} .profile-card-name{background:var(--mention-chip-bg);color:var(--mention-text)}`}</style>
      )}
      <main className="app-main">
        <ConnectionBanner />
        <Outlet />
      </main>
      <GlobalOverlays />
      <Toaster />
    </div>
  );
}
