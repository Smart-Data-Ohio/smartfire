import type { Settings } from "../gen/Settings.ts";
import { applyAccountAppearance } from "../lib/appearance.ts";
import { compareSnapshots } from "../store/snapshot-order.ts";
import { mutations, store } from "../store/store.ts";

let latestSettings: Settings | null = null;

/** Settings pages and membership writes share the same server revision ordering. */
export function applySettingsSnapshot(next: Settings): Settings {
  if (
    latestSettings?.profile.userId !== next.profile.userId ||
    compareSnapshots(next, latestSettings) >= 0
  ) {
    latestSettings = next;
    const held = store.getState().me;

    if (held?.user.id === next.profile.userId) {
      mutations.setMe({
        ...held,
        preferences: {
          ...held.preferences,
          theme: next.appearance.theme,
          textSize: next.appearance.textSize,
          appearancePreferences: next.appearance.appearancePreferences,
        },
      });
      applyAccountAppearance(next.appearance);
    }

    mutations.setNotificationPreferences(next.notifications, next.evaluatedAt);
  }

  return latestSettings;
}
