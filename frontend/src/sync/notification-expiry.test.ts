import { afterEach, expect, it, vi } from "vitest";
import { sidebarRowFixture } from "../api/testing.ts";
import { organizedSidebar } from "../store/organize.ts";
import { initialState } from "../store/state.ts";
import { mutations, store } from "../store/store.ts";
import { notificationPreferencesFixture } from "../test/notification-fixtures.ts";
import { followNotificationPreferences } from "./settings.ts";

let stop: (() => void) | undefined;

afterEach(() => {
  stop?.();
  vi.useRealTimers();
  store.setState(initialState, true);
});

it("advances the SPA notification clock when a mute ends", async () => {
  vi.useFakeTimers({ toFake: ["Date", "performance", "setTimeout", "clearTimeout"] });
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
  mutations.setNotificationPreferences(notificationPreferencesFixture, "2026-10-10T12:00:00Z");
  stop = followNotificationPreferences();
  await vi.advanceTimersByTimeAsync(15 * 60_000);
  expect(store.getState().sidebar.notificationClock).toBe(Date.parse("2026-10-10T12:15:00Z"));
});

it.each(["2026-10-10T12:30:00Z", "2026-10-10T11:30:00Z"])(
  "expires a mute after server elapsed time with browser time %s",
  async (browserTime) => {
    vi.useFakeTimers({ toFake: ["Date", "performance", "setTimeout", "clearTimeout"] });
    vi.setSystemTime(new Date(browserTime));
    store.setState(
      {
        ...initialState,
        sidebar: {
          ...initialState.sidebar,
          rows: { 4: sidebarRowFixture(4, "general") },
        },
      },
      true,
    );
    mutations.setNotificationPreferences(notificationPreferencesFixture, "2026-10-10T12:00:00Z");
    stop = followNotificationPreferences();
    expect(organizedSidebar(store.getState().sidebar).rows[4]?.membership.involvement).toBe(
      "muted",
    );
    await vi.advanceTimersByTimeAsync(15 * 60_000 - 1);
    expect(organizedSidebar(store.getState().sidebar).rows[4]?.membership.involvement).toBe(
      "muted",
    );
    vi.setSystemTime(new Date("2035-01-01T00:00:00Z"));
    await vi.advanceTimersByTimeAsync(1);
    expect(store.getState().sidebar.notificationClock).toBe(Date.parse("2026-10-10T12:15:00Z"));
    expect(organizedSidebar(store.getState().sidebar).rows[4]?.membership.involvement).toBe(
      "mentions",
    );
  },
);
