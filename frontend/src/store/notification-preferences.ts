import type { Involvement } from "../gen/Involvement.ts";
import type { NotificationLevel } from "../gen/NotificationLevel.ts";
import type { NotificationSettings } from "../gen/NotificationSettings.ts";
import type { SidebarRow } from "./model.ts";

export const NOTIFICATION_LEVELS = [
  { value: "everything", label: "All messages" },
  { value: "mentions", label: "Only mentions" },
  { value: "nothing", label: "No notifications" },
] as const satisfies readonly { readonly value: NotificationLevel; readonly label: string }[];

export function roomMuted(
  preferences: NotificationSettings | undefined,
  roomId: number,
  now: number,
): boolean {
  const until = preferences?.roomMuteUntil[String(roomId)];

  return until === null || (until !== undefined && Date.parse(until) > now);
}

export function roomNotificationLevel(
  preferences: NotificationSettings | undefined,
  roomId: number,
  stored: Involvement,
): Involvement {
  const level = preferences?.roomNotificationLevels[String(roomId)];

  return level === null ? (preferences?.defaultNotificationLevel ?? stored) : (level ?? stored);
}

export function notificationRow(
  row: SidebarRow,
  preferences: NotificationSettings | undefined,
  now: number,
): SidebarRow {
  if (preferences === undefined) return row;
  const muted = roomMuted(preferences, row.room.id, now);
  const level = roomNotificationLevel(preferences, row.room.id, row.membership.involvement);

  return {
    ...row,
    membership: {
      ...row.membership,
      involvement: muted ? "muted" : level,
      unreadAt: muted ? null : row.membership.unreadAt,
    },
    mentionCount: muted ? 0 : row.mentionCount,
    notificationCount: muted ? 0 : row.notificationCount,
    threadNotificationCount: muted ? 0 : row.threadNotificationCount,
  };
}
