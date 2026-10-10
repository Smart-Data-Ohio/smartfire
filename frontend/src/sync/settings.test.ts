import { afterAll, afterEach, expect, it, vi } from "vitest";
import { SEED_IDS } from "../../mock/server.ts";
import { roomMuted } from "../store/notification-preferences.ts";
import { mutations, store } from "../store/store.ts";
import { installMockNetwork } from "../test/mock-network.ts";
import { settings } from "./settings.ts";

const network = installMockNetwork();

afterAll(() => network.restore());

afterEach(() => {
  vi.restoreAllMocks();
  mutations.reset();
});

function held<A>() {
  let release: (value: A) => void = () => undefined;

  const promise = new Promise<A>((resolve) => {
    release = resolve;
  });

  return { promise, release };
}

it("keeps a successful room mute when an older settings GET lands", async () => {
  const fetch = globalThis.fetch;
  const started = held<void>();
  const gate = held<void>();
  vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
    const response = await fetch(input, init);

    if (String(input).endsWith("/settings") && (init?.method ?? "GET") === "GET") {
      started.release();
      await gate.promise;
    }

    return response;
  });

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
