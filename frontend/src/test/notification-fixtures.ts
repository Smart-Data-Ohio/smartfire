import type { NotificationSettings } from "../gen/NotificationSettings.ts";

export const notificationPreferencesFixture: NotificationSettings = {
  defaultNotificationLevel: "mentions",
  roomNotificationLevels: { "4": null, "5": "nothing" },
  roomMuteUntil: { "4": "2026-10-10T12:15:00Z", "5": null },
  dndEnabled: false,
  quietHoursEnabled: false,
  quietHoursStart: null,
  quietHoursEnd: null,
  meetingDndEnabled: false,
  oooNotifyEnabled: false,
  allowedPeople: [],
  keywordAlerts: [],
  inbox: [],
};
