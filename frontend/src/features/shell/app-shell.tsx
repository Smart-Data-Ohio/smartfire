import { Outlet, useMatchRoute, useParams } from "@tanstack/react-router";
import { useEffect } from "react";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Toaster } from "../../ui/toast.tsx";
import { sidebarTotals } from "../sidebar/sections.ts";
import { Sidebar } from "../sidebar/sidebar.tsx";
import { GlobalOverlays } from "../switcher/global-overlays.tsx";
import { ConnectionBanner } from "./connection-banner.tsx";
import { Rail } from "./rail.tsx";
import "./app-shell.css";

/** "(3) #general · Smartfire": unread mentions first, as Slack's tab title does. */
function useDocumentTitle(roomId: number | null): void {
  const mentions = useStore((state) => sidebarTotals(state.sidebar).mentions);
  const unread = useStore((state) => sidebarTotals(state.sidebar).unreadRooms > 0);
  const room = useStore((state) => (roomId === null ? null : (state.sidebar.rows[roomId] ?? null)));

  useEffect(() => {
    const prefix = mentions > 0 ? `(${mentions}) ` : unread ? "• " : "";

    const name =
      room === null
        ? "Smartfire"
        : `${room.room.kind === "direct" ? "" : "#"}${room.displayName} · Smartfire`;

    document.title = `${prefix}${name}`;
  }, [mentions, unread, room]);
}

/**
 * Which column a phone shows: the conversation list, a tab's page beside the tab bar (the
 * activity inbox), or a pushed full screen (a conversation, Saved, Scheduled).
 */
function usePhoneView(roomId: number | null): "list" | "tab" | "room" {
  const matchRoute = useMatchRoute();

  const pushed =
    matchRoute({ to: "/saved" }) !== false || matchRoute({ to: "/scheduled" }) !== false;

  if (roomId !== null || pushed) {
    return "room";
  }

  return matchRoute({ to: "/activity" }) === false ? "list" : "tab";
}

/**
 * The signed-in app: rail, sidebar and the routed pane. Starts the sync engine once. On phones
 * only one column shows (`data-view`): the conversation list, a tab page, or a full screen.
 */
export function AppShell() {
  const params = useParams({ strict: false });
  const roomId = params.roomId ?? null;

  useEffect(() => {
    void actions.start();
  }, []);

  useDocumentTitle(roomId);

  const view = usePhoneView(roomId);

  const viewerId = useStore((state) => state.me?.user.id ?? state.boot?.user.id ?? null);

  return (
    <div className="app-shell" data-view={view}>
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
