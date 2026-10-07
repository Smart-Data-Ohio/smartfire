/**
 * Who's in which call, kept fresh: `huddle.presence` events carry joins, leaves and revocations,
 * and a 15-second poll while the tab is visible catches people who dropped out of the in-call
 * window without leaving (a closed laptop), as the classic `huddle_presence` controller did. A
 * 503 `Unavailable` (no LiveKit configured) hides every huddle control.
 */
import { useStore } from "zustand";
import { createStore } from "zustand/vanilla";
import type { HuddleParticipant } from "../../gen/HuddleParticipant.ts";
import { useStore as useAppStore } from "../../store/store.ts";
import { huddles } from "../../sync/huddles.ts";
import { ActionError } from "../../sync/run.ts";

export const PRESENCE_POLL_MS = 15_000;

interface Availability {
  /** Huddles work on this server (until a poll says `Unavailable`). */
  readonly available: boolean;
}

export const huddleAvailability = createStore<Availability>()(() => ({ available: true }));

export function useHuddlesAvailable(): boolean {
  return useStore(huddleAvailability, (state) => state.available);
}

const NO_PARTICIPANTS: readonly HuddleParticipant[] = [];

/** The people in a room's call, in name order (empty when nobody is). */
export function useCallParticipants(roomId: number): readonly HuddleParticipant[] {
  return useAppStore((state) => state.huddles[roomId]?.participants ?? NO_PARTICIPANTS);
}

/** A stage room is streaming (the sidebar's live dot). */
export function useCallLive(roomId: number): boolean {
  return useAppStore((state) => state.huddles[roomId]?.live === true);
}

async function refresh(poll: () => Promise<void>): Promise<void> {
  try {
    await poll();
    huddleAvailability.setState({ available: true });
  } catch (error) {
    if (error instanceof ActionError && error.tag === "Unavailable") {
      huddleAvailability.setState({ available: false });
    }
  }
}

/**
 * Polls while the tab is visible (and once on becoming visible); returns the stop. `poll` is
 * the aggregate presence request (tests stand in for it).
 */
export function startPresencePolling(
  poll: () => Promise<void> = () => huddles.refreshPresence(),
): () => void {
  let timer: ReturnType<typeof setInterval> | null = null;

  const stop = () => {
    if (timer !== null) {
      clearInterval(timer);
      timer = null;
    }
  };

  const start = () => {
    stop();
    void refresh(poll);
    timer = setInterval(() => {
      void refresh(poll);
    }, PRESENCE_POLL_MS);
  };

  const onVisibility = () => {
    if (document.visibilityState === "visible") {
      start();
    } else {
      stop();
    }
  };

  document.addEventListener("visibilitychange", onVisibility);
  onVisibility();

  return () => {
    document.removeEventListener("visibilitychange", onVisibility);
    stop();
  };
}
