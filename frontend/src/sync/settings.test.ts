import { afterAll, afterEach, expect, it, vi } from "vitest";
import { SEED_IDS } from "../../mock/server.ts";
import { roomMuted } from "../store/notification-preferences.ts";
import { mutations, store } from "../store/store.ts";
import { installMockNetwork } from "../test/mock-network.ts";
import * as activity from "./activity-actions.ts";
import { runAction } from "./runtime.ts";
import { followNotificationPreferences, settings } from "./settings.ts";

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

function held<A>() {
  let release: (value: A) => void = () => undefined;

  const promise = new Promise<A>((resolve) => {
    release = resolve;
  });

  return { promise, release };
}

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
  "keeps a newer %s refresh over a delayed mute count at the same activity revision",
  async (newerKind) => {
    const started = held<void>();
    const gate = held<void>();
    const released = held<void>();
    let countRequests = 0;
    intercept = async (input, init) => {
      if (String(input).endsWith("/activity/unread_count")) {
        countRequests += 1;
        const first = countRequests === 1;
        const response = Response.json({ unreadCount: first ? 0 : 1, unreadRevision: 7 });

        if (first) {
          started.release();
          await gate.promise;
          released.release();
        }

        return response;
      }

      return fetch(input, init);
    };

    mutations.setActivityUnreadCount({ unreadCount: 1, unreadRevision: 7 });
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
