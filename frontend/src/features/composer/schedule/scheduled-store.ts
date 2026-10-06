/**
 * The viewer's pending scheduled messages per room, shared by the room's composer and its thread
 * composers. No sync event announces a scheduled send, so the list refreshes when a composer
 * mounts, after each change, and shortly after the soonest one falls due (the scheduler checks
 * every 30 s).
 */
import { useEffect } from "react";
import { useStore as useZustand } from "zustand";
import { createStore } from "zustand/vanilla";
import type { CreateScheduledMessage } from "../../../gen/CreateScheduledMessage.ts";
import type { ScheduledMessage } from "../../../gen/ScheduledMessage.ts";
import { composerActions } from "../../../sync/composer-actions.ts";

interface ScheduledState {
  readonly byRoom: ReadonlyMap<number, readonly ScheduledMessage[]>;
}

const scheduledStore = createStore<ScheduledState>()(() => ({ byRoom: new Map() }));

const EMPTY: readonly ScheduledMessage[] = [];

/** How long after a send time the list looks again (the scheduler's 30 s tick, plus slack). */
const DUE_GRACE_MS = 35_000;

function put(roomId: number, items: readonly ScheduledMessage[]): void {
  scheduledStore.setState((state) => ({ byRoom: new Map(state.byRoom).set(roomId, items) }));
}

function bySendAt(items: readonly ScheduledMessage[]): readonly ScheduledMessage[] {
  return items.toSorted((a, b) => a.sendAt.localeCompare(b.sendAt));
}

/** Fetches the room's list; a failure keeps what's shown. */
export async function refreshScheduled(roomId: number): Promise<void> {
  try {
    put(roomId, bySendAt(await composerActions.scheduled(roomId)));
  } catch {
    // The affordance is a convenience: keep the last list rather than flashing an error.
  }
}

function replace(roomId: number, item: ScheduledMessage): void {
  const current = scheduledStore.getState().byRoom.get(roomId) ?? EMPTY;
  const pending = item.sentAt === null && item.droppedAt === null;
  const others = current.filter((entry) => entry.id !== item.id);

  put(roomId, bySendAt(pending ? [...others, item] : others));
}

function drop(roomId: number, id: number): void {
  const current = scheduledStore.getState().byRoom.get(roomId) ?? EMPTY;

  put(
    roomId,
    current.filter((entry) => entry.id !== id),
  );
}

/** The scheduled-message writes; each updates the shared list, and rejects with a message. */
export const scheduled = {
  async create(roomId: number, body: CreateScheduledMessage): Promise<ScheduledMessage> {
    const created = await composerActions.schedule(roomId, body);

    replace(roomId, created);

    return created;
  },
  async reschedule(item: ScheduledMessage, sendAt: string): Promise<void> {
    replace(
      item.roomId,
      await composerActions.updateScheduled(item.id, {
        markdownSource: item.markdownSource,
        sendAt,
      }),
    );
  },
  async sendNow(item: ScheduledMessage): Promise<void> {
    await composerActions.sendScheduledNow(item.id);
    drop(item.roomId, item.id);
  },
  async cancel(item: ScheduledMessage): Promise<void> {
    await composerActions.cancelScheduled(item.id);
    drop(item.roomId, item.id);
  },
};

/**
 * This conversation's pending scheduled messages, soonest first: the room's root ones, or one
 * thread's. Loads the room's list on mount and re-checks once the soonest falls due.
 */
export function useScheduled(
  roomId: number,
  threadId: number | null,
  enabled: boolean,
): readonly ScheduledMessage[] {
  const all = useZustand(scheduledStore, (state) => state.byRoom.get(roomId) ?? EMPTY);

  useEffect(() => {
    if (enabled) {
      void refreshScheduled(roomId);
    }
  }, [roomId, enabled]);

  const soonest = all[0]?.sendAt ?? null;

  useEffect(() => {
    if (!enabled || soonest === null) {
      return;
    }

    const wait = Math.max(0, Date.parse(soonest) - Date.now()) + DUE_GRACE_MS;

    // Timers beyond ~24.8 days overflow; a later mount will look again anyway.
    const timer = window.setTimeout(
      () => void refreshScheduled(roomId),
      Math.min(wait, 2 ** 31 - 1),
    );

    return () => window.clearTimeout(timer);
  }, [roomId, soonest, enabled]);

  return all.filter((item) => item.threadId === threadId);
}
