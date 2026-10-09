import type { ChatSounds } from "../gen/ChatSounds.ts";
import { emptyTimeline } from "../store/state.ts";
import { store } from "../store/store.ts";
import { onSyncEvents } from "./signals.ts";

export function chatSoundsMuted(policy: ChatSounds, now: Date): boolean {
  const second = Math.floor(now.getTime() / 1000);

  if (policy.muted || policy.quietWindows.some(([start, end]) => start <= second && second < end)) {
    return true;
  }

  const quiet = policy.quietHours;

  if (quiet === null || quiet.startMinute === quiet.endMinute) {
    return false;
  }

  try {
    const parts = new Intl.DateTimeFormat("en-US", {
      timeZone: policy.timeZone,
      hour: "numeric",
      minute: "numeric",
      hourCycle: "h23",
    }).formatToParts(now);

    const hour = Number(parts.find((part) => part.type === "hour")?.value);
    const minute = hour * 60 + Number(parts.find((part) => part.type === "minute")?.value);

    return quiet.startMinute < quiet.endMinute
      ? minute >= quiet.startMinute && minute < quiet.endMinute
      : minute >= quiet.startMinute || minute < quiet.endMinute;
  } catch {
    return false;
  }
}

/** Manual and automatic playback share classic's quiet gates, checked at the time of play. */
export function playChatSound(url: string): void {
  const me = store.getState().me;

  if (me === null || chatSoundsMuted(me.chatSounds, new Date())) {
    return;
  }

  void new Audio(url).play().catch(() => undefined);
}

/** Listen on the timeline, so virtualized rows and history mounts never trigger playback. */
export function listenForChatSounds(roomId: number, viewingLatestPage: () => boolean): () => void {
  const seen = new Set<number>(
    Object.values(store.getState().messages).map((message) => message.id),
  );

  return onSyncEvents((events, liveAfter) => {
    for (const event of events) {
      if (event.type !== "message.created" || seen.has(event.data.id)) {
        continue;
      }

      const message = event.data;

      seen.add(message.id);

      const state = store.getState();
      const timeline = state.timelines[roomId] ?? emptyTimeline;

      if (
        event.seq > liveAfter &&
        message.sound !== null &&
        message.threadId === null &&
        message.roomId === roomId &&
        timeline.status === "ready" &&
        timeline.after === null &&
        viewingLatestPage() &&
        timeline.ids.includes(message.id)
      ) {
        playChatSound(message.sound.url);
      }
    }
  });
}
