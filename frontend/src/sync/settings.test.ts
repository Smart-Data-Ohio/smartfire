import { afterAll, afterEach, expect, it, vi } from "vitest";
import { SEED_IDS } from "../../mock/server.ts";
import { me, sidebar } from "../api/endpoints.ts";
import { Validation } from "../api/errors.ts";
import {
  appearanceSnapshot,
  applyAccountAppearance,
  restoreAppearance,
  setPalette,
  setPersonalAppearanceOverride,
} from "../lib/appearance.ts";
import { roomMuted } from "../store/notification-preferences.ts";
import { organizedSidebar } from "../store/organize.ts";
import { mutations, sidebarRowClock, store } from "../store/store.ts";
import { installMockNetwork } from "../test/mock-network.ts";
import * as activity from "./activity-actions.ts";
import { setInvolvement } from "./organize-actions.ts";
import { runAction } from "./runtime.ts";
import {
  followNotificationPreferences,
  saveAccountPersonalAppearance,
  settings,
} from "./settings.ts";
import { emitResync } from "./signals.ts";

const network = installMockNetwork();

const fetch = globalThis.fetch;

let intercept: typeof fetch = fetch;

vi.spyOn(globalThis, "fetch").mockImplementation((input, init) => intercept(input, init));

afterAll(() => {
  vi.restoreAllMocks();
  network.restore();
});

afterEach(() => {
  intercept = fetch;
  mutations.reset();
});

async function personalAccount(
  preferences: NonNullable<
    Parameters<typeof settings.updateAppearance>[0]["appearancePreferences"]
  >,
) {
  localStorage.clear();
  restoreAppearance();
  await settings.updateAppearance({ appearancePreferences: preferences });
  const account = await runAction(me());
  mutations.setMe(account);
  applyAccountAppearance(account.preferences);
}

it("saves personal choices while keeping semantic tokens and a device override", async () => {
  await personalAccount({ version: 1, palette: "rose", tokens: { "--accent": "#123abc" } });
  setPalette("ember");
  await saveAccountPersonalAppearance({ palette: "ocean", font: "mono" });
  expect((await settings.load()).appearance.appearancePreferences).toEqual({
    version: 1,
    palette: "ocean",
    font: "mono",
    tokens: { "--accent": "#123abc" },
  });
  expect(appearanceSnapshot()).toMatchObject({ palette: "ember", font: "mono" });
  setPersonalAppearanceOverride(false);
  expect(appearanceSnapshot().palette).toBe("ocean");
  expect(document.documentElement.style.getPropertyValue("--accent")).toBe("#123abc");
});

it("serializes rapid personal saves without losing earlier choices", async () => {
  await personalAccount({ version: 1 });
  await Promise.all([
    saveAccountPersonalAppearance({ palette: "forest" }),
    saveAccountPersonalAppearance({ font: "serif", density: "compact", motion: "reduce" }),
  ]);
  expect((await settings.load()).appearance.appearancePreferences).toEqual({
    version: 1,
    palette: "forest",
    font: "serif",
    density: "compact",
    motion: "reduce",
  });
});

it("restores the previous account appearance after a rejected save", async () => {
  await personalAccount({ version: 1, palette: "ocean" });
  intercept = async (input, init) =>
    String(input).endsWith("/settings/appearance")
      ? new Response(
          JSON.stringify({
            error: new Validation({
              message: "Appearance rejected",
              fields: { appearancePreferences: ["not allowed"] },
            }),
          }),
          { status: 422, headers: { "content-type": "application/json" } },
        )
      : fetch(input, init);
  await expect(saveAccountPersonalAppearance({ palette: "rose" })).rejects.toMatchObject({
    message: "Appearance rejected",
  });
  expect(appearanceSnapshot().palette).toBe("ocean");
});

it("leaves a newer appearance version intact rather than replacing it with v1", async () => {
  const future = { version: 99, future: { retain: [1, 2, 3] } };
  await personalAccount(future);
  await expect(saveAccountPersonalAppearance({ palette: "rose" })).rejects.toThrow(
    "newer Smartfire client",
  );
  expect((await settings.load()).appearance.appearancePreferences).toEqual(future);
});

function held<A>() {
  let release: (value: A) => void = () => undefined;

  const promise = new Promise<A>((resolve) => {
    release = resolve;
  });

  return { promise, release };
}

it("refreshes a missed room mute on user-topic resync and rejects an older held read", async () => {
  const roomId = SEED_IDS.rooms.general;
  await settings.updateNotifications({ roomMute: { roomId, duration: "off" } });
  mutations.setMe(await runAction(me()));
  const started = held<void>();
  const gate = held<void>();
  let delay = true;
  intercept = async (input, init) => {
    const response = await fetch(input, init);

    if (String(input).endsWith("/settings") && delay) {
      delay = false;
      started.release();
      await gate.promise;
    }

    return response;
  };

  const stop = followNotificationPreferences();

  try {
    await started.promise;

    const changed = await fetch("/api/v1/settings/notifications", {
      method: "PATCH",
      headers: {
        "content-type": "application/json",
        "x-csrf-token": network.server.csrfToken(),
      },
      body: JSON.stringify({ roomMute: { roomId, duration: "forever" } }),
    });

    expect(changed.status).toBe(200);
    emitResync(["user"]);
    await vi.waitFor(() =>
      expect(roomMuted(store.getState().sidebar.notificationPreferences, roomId, Date.now())).toBe(
        true,
      ),
    );
    gate.release();
    await new Promise<void>((resolve) => setTimeout(resolve, 0));
    expect(roomMuted(store.getState().sidebar.notificationPreferences, roomId, Date.now())).toBe(
      true,
    );
  } finally {
    gate.release();
    stop();
    await new Promise<void>((resolve) => setTimeout(resolve, 0));
    await settings.updateNotifications({ roomMute: { roomId, duration: "off" } });
  }
});

it("keeps an explicit room choice when a held settings GET arrives without another fetch", async () => {
  const roomId = SEED_IDS.rooms.general;
  const since = sidebarRowClock();

  mutations.loadSidebar(await runAction(sidebar()), since);
  await settings.updateNotifications({
    defaultNotificationLevel: "nothing",
    roomNotification: { roomId, level: null },
  });
  expect(organizedSidebar(store.getState().sidebar).rows[roomId]?.membership.involvement).toBe(
    "nothing",
  );
  const started = held<void>();
  const gate = held<void>();
  intercept = async (input, init) => {
    const response = await fetch(input, init);

    if (String(input).endsWith("/settings")) {
      started.release();
      await gate.promise;
    }

    return response;
  };

  const older = settings.load();
  await started.promise;
  await runAction(setInvolvement(roomId, "everything"));
  expect(organizedSidebar(store.getState().sidebar).rows[roomId]?.membership.involvement).toBe(
    "everything",
  );
  gate.release();
  const answered = await older;
  expect(answered.notifications.defaultNotificationLevel).toBe("nothing");
  expect(answered.notifications.roomNotificationLevels[String(roomId)]).toBeUndefined();
  expect(
    store.getState().sidebar.notificationPreferences?.roomNotificationLevels[String(roomId)],
  ).toBeUndefined();
  expect(organizedSidebar(store.getState().sidebar).rows[roomId]?.membership.involvement).toBe(
    "everything",
  );
});

it("keeps expired mutes out of settings when an earlier equal-revision GET arrives last", async () => {
  const roomId = SEED_IDS.rooms.general;
  const saved = await settings.updateNotifications({ roomMute: { roomId, duration: "minutes15" } });
  const started = held<void>();
  const gate = held<void>();
  let delay = true;
  intercept = async (input, init) => {
    if (String(input).endsWith("/settings")) {
      const older = delay;
      delay = false;

      const response = Response.json({
        ...saved,
        evaluatedAt: older ? "2035-01-01T12:14:59.999999999Z" : "2035-01-01T12:15:00.000000000Z",
        notifications: {
          ...saved.notifications,
          roomMuteUntil: older ? saved.notifications.roomMuteUntil : {},
        },
      });

      if (older) {
        started.release();
        await gate.promise;
      }

      return response;
    }

    return fetch(input, init);
  };

  const older = settings.load();
  await started.promise;
  await settings.load();
  gate.release();
  expect((await older).notifications.roomMuteUntil).toEqual({});
  expect(store.getState().sidebar.notificationPreferences?.roomMuteUntil).toEqual({});
});

it("applies a mute committed after a GET even when its PATCH started first", async () => {
  const roomId = SEED_IDS.rooms.general;
  await settings.updateNotifications({ roomMute: { roomId, duration: "off" } });
  const started = held<void>();
  const gate = held<void>();
  intercept = async (input, init) => {
    if (String(input).endsWith("/settings/notifications")) {
      started.release();
      await gate.promise;
    }

    return fetch(input, init);
  };

  const pending = settings.updateNotifications({ roomMute: { roomId, duration: "forever" } });
  await started.promise;
  const before = await settings.load();
  expect(roomMuted(before.notifications, roomId, Date.now())).toBe(false);
  gate.release();
  const saved = await pending;
  expect(roomMuted(store.getState().sidebar.notificationPreferences, roomId, Date.now())).toBe(
    true,
  );
  expect(roomMuted(saved.notifications, roomId, Date.now())).toBe(true);
  expect(roomMuted((await settings.load()).notifications, roomId, Date.now())).toBe(true);
});

it.each(["PATCH", "GET"])(
  "keeps a room mute when an older settings PATCH lands after a newer %s",
  async (newerMethod) => {
    const roomId = SEED_IDS.rooms.general;
    await settings.updateNotifications({ roomMute: { roomId, duration: "off" } });
    const started = held<void>();
    const gate = held<void>();
    let delay = true;
    intercept = async (input, init) => {
      const response = await fetch(input, init);

      if (String(input).endsWith("/settings/notifications") && delay) {
        delay = false;
        started.release();
        await gate.promise;
      }

      return response;
    };

    const older = settings.updateNotifications({ defaultNotificationLevel: "mentions" });
    await started.promise;
    await settings.updateNotifications({ roomMute: { roomId, duration: "minutes15" } });

    if (newerMethod === "GET") await settings.load();
    expect(roomMuted(store.getState().sidebar.notificationPreferences, roomId, Date.now())).toBe(
      true,
    );
    gate.release();
    const answered = await older;
    expect(roomMuted(store.getState().sidebar.notificationPreferences, roomId, Date.now())).toBe(
      true,
    );
    expect(roomMuted(answered.notifications, roomId, Date.now())).toBe(true);
    expect(roomMuted((await settings.load()).notifications, roomId, Date.now())).toBe(true);
  },
);

it("keeps newer settings GET results when an earlier GET arrives last", async () => {
  const roomId = SEED_IDS.rooms.general;
  await settings.updateNotifications({ roomMute: { roomId, duration: "off" } });
  const started = held<void>();
  const gate = held<void>();
  let delay = true;
  intercept = async (input, init) => {
    const response = await fetch(input, init);

    if (String(input).endsWith("/settings") && delay) {
      delay = false;
      started.release();
      await gate.promise;
    }

    return response;
  };

  const older = settings.load();
  await started.promise;

  const changed = await fetch("/api/v1/settings/notifications", {
    method: "PATCH",
    headers: {
      "content-type": "application/json",
      "x-csrf-token": network.server.csrfToken(),
    },
    body: JSON.stringify({ roomMute: { roomId, duration: "forever" } }),
  });

  expect(changed.status).toBe(200);
  await settings.load();
  gate.release();
  const answered = await older;
  expect(roomMuted(store.getState().sidebar.notificationPreferences, roomId, Date.now())).toBe(
    true,
  );
  expect(roomMuted(answered.notifications, roomId, Date.now())).toBe(true);
});

it.each(["policy", "badge"])(
  "keeps a newer %s refresh over a delayed mute count at an older server revision",
  async (newerKind) => {
    const started = held<void>();
    const gate = held<void>();
    const released = held<void>();
    let countRequests = 0;
    intercept = async (input, init) => {
      if (String(input).endsWith("/activity/unread_count")) {
        countRequests += 1;

        const first = countRequests === 1;

        const response = Response.json({
          unreadCount: first ? 0 : 1,
          unreadRevision: first ? 8 : 9,
          evaluatedAt: "2026-10-10T12:00:00.000000000Z",
        });

        if (first) {
          started.release();
          await gate.promise;
          released.release();
        }

        return response;
      }

      return fetch(input, init);
    };

    mutations.setActivityUnreadCount({
      unreadCount: 1,
      unreadRevision: 7,
      evaluatedAt: "2026-10-10T12:00:00.000000000Z",
    });
    const stop = followNotificationPreferences();

    try {
      const roomId = SEED_IDS.rooms.general;
      await settings.updateNotifications({ roomMute: { roomId, duration: "minutes15" } });
      await started.promise;
      await settings.updateNotifications({ roomMute: { roomId, duration: "off" } });
      await vi.waitFor(() => expect(countRequests).toBe(2));
      await vi.waitFor(() => expect(store.getState().activity.serverUnread?.unreadCount).toBe(1));

      if (newerKind === "badge") await runAction(activity.loadUnreadCount());
      gate.release();
      await released.promise;
      await new Promise<void>((resolve) => setTimeout(resolve, 0));
      expect(store.getState().activity.unreadCount).toBe(1);
    } finally {
      gate.release();
      stop();
    }
  },
);

it("keeps a successful room mute when an older settings GET lands", async () => {
  const started = held<void>();
  const gate = held<void>();
  intercept = async (input, init) => {
    const response = await fetch(input, init);

    if (String(input).endsWith("/settings") && (init?.method ?? "GET") === "GET") {
      started.release();
      await gate.promise;
    }

    return response;
  };

  const older = settings.load();
  await started.promise;
  const roomId = SEED_IDS.rooms.general;
  await settings.updateNotifications({ roomMute: { roomId, duration: "minutes15" } });
  expect(roomMuted(store.getState().sidebar.notificationPreferences, roomId, Date.now())).toBe(
    true,
  );
  gate.release();
  await older;
  expect(roomMuted(store.getState().sidebar.notificationPreferences, roomId, Date.now())).toBe(
    true,
  );
});

it("accepts settings reads started after the successful write", async () => {
  const roomId = SEED_IDS.rooms.general;
  await settings.updateNotifications({ roomMute: { roomId, duration: "forever" } });

  const changed = await fetch("/api/v1/settings/notifications", {
    method: "PATCH",
    headers: {
      "content-type": "application/json",
      "x-csrf-token": network.server.csrfToken(),
    },
    body: JSON.stringify({ roomMute: { roomId, duration: "off" } }),
  });

  expect(changed.status).toBe(200);
  await settings.load();
  expect(roomMuted(store.getState().sidebar.notificationPreferences, roomId, Date.now())).toBe(
    false,
  );
});

it.each(["pending", "failed"])(
  "applies an earlier successful PATCH while a newer settings request is %s",
  async (newerOutcome) => {
    const roomId = SEED_IDS.rooms.general;
    await settings.updateNotifications({ roomMute: { roomId, duration: "off" } });
    const started = held<void>();
    const olderGate = held<void>();
    const newerStarted = held<void>();
    const newerGate = held<void>();
    intercept = async (input, init) => {
      const response = await fetch(input, init);

      if (String(input).endsWith("/settings/notifications")) {
        started.release();
        await olderGate.promise;
      } else if (String(input).endsWith("/settings")) {
        newerStarted.release();
        await newerGate.promise;

        if (newerOutcome === "failed") throw new TypeError("network unavailable");
      }

      return response;
    };

    const older = settings.updateNotifications({ roomMute: { roomId, duration: "forever" } });
    await started.promise;
    const newer = settings.load();
    await newerStarted.promise;

    if (newerOutcome === "failed") {
      const rejected = expect(newer).rejects.toThrow();
      newerGate.release();
      await rejected;
    }

    olderGate.release();
    await older;
    expect(roomMuted(store.getState().sidebar.notificationPreferences, roomId, Date.now())).toBe(
      true,
    );

    if (newerOutcome === "pending") {
      newerGate.release();
      await newer;
    }
  },
);
