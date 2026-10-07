import { Navigate } from "@tanstack/react-router";
import { useSyncExternalStore } from "react";
import { useStore } from "../../store/store.ts";
import { PageLoading } from "./page-loading.tsx";

const PHONE_QUERY = "(width < 720px)";

function subscribePhone(onChange: () => void): () => void {
  const media = window.matchMedia(PHONE_QUERY);

  media.addEventListener("change", onChange);

  return () => media.removeEventListener("change", onChange);
}

function isPhone(): boolean {
  return "matchMedia" in window && window.matchMedia(PHONE_QUERY).matches;
}

/**
 * `/app/`: on wide screens, straight into the last room (`me.lastRoomId`); on phones the list is
 * the home screen, so this pane stays empty and the shell shows the sidebar.
 */
export function HomeView() {
  const phone = useSyncExternalStore(subscribePhone, isPhone, () => false);
  const me = useStore((state) => state.me);

  if (phone) {
    return null;
  }

  if (me?.lastRoomId != null) {
    return <Navigate to="/r/$roomId" params={{ roomId: me.lastRoomId }} replace />;
  }

  if (me === null) {
    return <PageLoading />;
  }

  return (
    <div className="home-empty">
      <p className="text-title">No conversations yet</p>
      <p className="text-muted">When someone adds you to a room, it shows up on the left.</p>
    </div>
  );
}
