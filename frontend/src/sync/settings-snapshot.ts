import type { Settings } from "../gen/Settings.ts";
import { compareSnapshots } from "../store/snapshot-order.ts";
import { mutations } from "../store/store.ts";

let latestSettings: Settings | null = null;

/** Settings pages and membership writes share the same server revision ordering. */
export function applySettingsSnapshot(next: Settings): Settings {
  if (
    latestSettings?.profile.userId !== next.profile.userId ||
    compareSnapshots(next, latestSettings) >= 0
  ) {
    latestSettings = next;
    mutations.setNotificationPreferences(next.notifications, next.evaluatedAt);
  }

  return latestSettings;
}
