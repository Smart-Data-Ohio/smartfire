/**
 * The composer's view of the viewer's pending scheduled messages: the room's list in the shared
 * store (`roomScheduledKey`), which the room's composer and its thread composers read, kept live
 * by `scheduled.changed`/`scheduled.removed` and by every write's reply. As a fallback for a
 * missed event, the list also looks again shortly after the soonest one falls due (the scheduler
 * checks every 30 s).
 */
import { useEffect, useMemo } from "react";
import type { CreateScheduledMessage } from "../../../gen/CreateScheduledMessage.ts";
import type { ScheduledMessage } from "../../../gen/ScheduledMessage.ts";
import { useScheduledList } from "../../../store/inbox-hooks.ts";
import { roomScheduledKey } from "../../../store/scheduled.ts";
import { actions } from "../../../sync/runtime.ts";

/** How long after a send time the list looks again (the scheduler's 30 s tick, plus slack). */
const DUE_GRACE_MS = 35_000;

/** Loads the room's list again; a failure keeps what's shown. */
export function refreshScheduled(roomId: number): Promise<void> {
  return actions.scheduled.load(roomScheduledKey(roomId));
}

/** The scheduled-message writes; each updates every list, and rejects with a message. */
export const scheduled = {
  create: (roomId: number, body: CreateScheduledMessage): Promise<ScheduledMessage> =>
    actions.scheduled.create(roomId, body),
  async reschedule(item: ScheduledMessage, sendAt: string): Promise<void> {
    await actions.scheduled.update(item.id, { markdownSource: item.markdownSource, sendAt });
  },
  async sendNow(item: ScheduledMessage): Promise<void> {
    await actions.scheduled.sendNow(item.id);
  },
  cancel: (item: ScheduledMessage): Promise<void> => actions.scheduled.cancel(item.id),
};

/**
 * This conversation's pending scheduled messages, soonest first: the room's root ones, or one
 * thread's. Loads the room's list when first shown and re-checks once the soonest falls due.
 */
export function useScheduled(
  roomId: number,
  threadId: number | null,
  enabled: boolean,
): readonly ScheduledMessage[] {
  const { rows } = useScheduledList(roomScheduledKey(roomId), enabled);
  const soonest = rows[0]?.message.sendAt ?? null;

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

  return useMemo(
    () => rows.flatMap((row) => (row.message.threadId === threadId ? [row.message] : [])),
    [rows, threadId],
  );
}
