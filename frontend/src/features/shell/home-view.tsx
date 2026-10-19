import { Link, Navigate } from "@tanstack/react-router";
import { useEffect, useState, useSyncExternalStore } from "react";
import { PHONE_QUERY } from "../../lib/breakpoints.ts";
import { useStore } from "../../store/store.ts";
import { PageFrame } from "../destinations/page-frame.tsx";
import { WelcomeCard } from "../welcome/welcome-card.tsx";
import { useWelcomeDismissed } from "../welcome/welcome-dismissal.ts";
import { hasWelcomeCard, useWelcome } from "../welcome/welcome-store.ts";
import { PageLoading } from "./page-loading.tsx";

function subscribePhone(onChange: () => void): () => void {
  const media = window.matchMedia(PHONE_QUERY);

  media.addEventListener("change", onChange);

  return () => media.removeEventListener("change", onChange);
}

function isPhone(): boolean {
  return "matchMedia" in window && window.matchMedia(PHONE_QUERY).matches;
}

/**
 * How long Home holds the redirect for a welcome still being read. Past it (or once the read
 * fails), Home goes on as if there were none; a welcome that turns up later doesn't pull anyone
 * back from the room they went to.
 */
export const WELCOME_WAIT_MS = 1200;

/** Whether a wait has lasted `ms`; once it has, it stays so while this view is up. */
function useWaitedTooLong(waiting: boolean, ms: number): boolean {
  const [expired, setExpired] = useState(false);

  useEffect(() => {
    if (!waiting) {
      return;
    }

    const timer = window.setTimeout(() => setExpired(true), ms);

    return () => window.clearTimeout(timer);
  }, [waiting, ms]);

  return expired;
}

/** Home with the workspace's welcome on top, and the way back to the last conversation. */
function WelcomeHome({ lastRoomId }: { readonly lastRoomId: number | null }) {
  const { welcome } = useWelcome();

  const lastRoom = useStore((state) =>
    lastRoomId === null ? null : (state.sidebar.rows[lastRoomId] ?? null),
  );

  if (!hasWelcomeCard(welcome)) {
    return null;
  }

  return (
    <PageFrame title="Home" icon="home">
      <div className="welcome-home">
        <WelcomeCard welcome={welcome} />
        {lastRoom === null ? null : (
          <p className="welcome-home-continue text-muted">
            Pick up where you left off in{" "}
            <Link to="/r/$roomId" params={{ roomId: lastRoom.room.id }}>
              {lastRoom.room.kind === "direct" ? "" : "#"}
              {lastRoom.roomDisplayName}
            </Link>
          </p>
        )}
      </div>
    </PageFrame>
  );
}

/**
 * `/app/`: on wide screens, straight into the last room (`me.lastRoomId`), unless the workspace
 * has a welcome the person hasn't dismissed: then Home shows it. On phones the list is the home
 * screen (the welcome sits at its top), so this pane stays empty and the shell shows the sidebar.
 */
export function HomeView() {
  const phone = useSyncExternalStore(subscribePhone, isPhone, () => false);
  const me = useStore((state) => state.me);
  const { status, welcome } = useWelcome();
  const welcomeDismissed = useWelcomeDismissed();
  const reading = !phone && !welcomeDismissed && (status === "idle" || status === "loading");
  const gaveUp = useWaitedTooLong(reading, WELCOME_WAIT_MS);

  if (phone) {
    return null;
  }

  if (!welcomeDismissed) {
    // A read still under way holds the redirect, briefly, so the welcome doesn't flash past.
    if (reading && !gaveUp) {
      return <PageLoading />;
    }

    if (hasWelcomeCard(welcome)) {
      return <WelcomeHome lastRoomId={me?.lastRoomId ?? null} />;
    }
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
