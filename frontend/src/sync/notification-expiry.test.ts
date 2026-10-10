import { afterEach, expect, it, vi } from "vitest";
import { initialState } from "../store/state.ts";
import { store } from "../store/store.ts";
import { notificationPreferencesFixture } from "../test/notification-fixtures.ts";
import { followNotificationPreferences } from "./settings.ts";

let stop: (() => void) | undefined;

afterEach(() => {
  stop?.();
  vi.useRealTimers();
  store.setState(initialState, true);
});

it("advances the SPA notification clock when a mute ends", async () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date("2026-10-10T12:00:00Z"));
  store.setState(
    {
      ...initialState,
      sidebar: {
        ...initialState.sidebar,
        notificationPreferences: notificationPreferencesFixture,
        notificationClock: Date.now(),
      },
    },
    true,
  );
  stop = followNotificationPreferences();
  await vi.advanceTimersByTimeAsync(15 * 60_000);
  expect(store.getState().sidebar.notificationClock).toBe(Date.parse("2026-10-10T12:15:00Z"));
});
