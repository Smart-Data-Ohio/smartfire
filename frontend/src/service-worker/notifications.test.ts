import { afterEach, describe, expect, it, vi } from "vitest";

afterEach(() => vi.unstubAllGlobals());

interface NotificationOptions {
  readonly data: { readonly badge: number | null; readonly path: string };
  readonly tag: string;
}

interface WorkerEvent {
  readonly data?: { json(): { title: string; options: NotificationOptions } };
  readonly notification?: { close(): void; readonly data: { readonly path: string } };
  readonly waitUntil?: (completion: Promise<unknown>) => void;
}

async function worker(
  client = { url: "https://smartfire.test/app/r/12", visibilityState: "hidden", focused: false },
) {
  const handlers = new Map<string, (event: WorkerEvent) => Promise<void> | void>();
  const navigate = vi.fn(async () => undefined);
  const focus = vi.fn(async () => null);
  const showNotification = vi.fn(async () => undefined);
  const setAppBadge = vi.fn(async () => undefined);
  const openWindow = vi.fn(async () => null);

  vi.stubGlobal("self", {
    location: new URL("https://smartfire.test/service-worker.js"),
    smartfireBuild: {
      version: "a",
      page: "/app/assets/index-aaaaaaaa.js",
      offline: "/offline.html",
      precache: [],
    },
    navigator: { setAppBadge },
    registration: { showNotification },
    clients: {
      matchAll: vi.fn(async () => [{ ...client, navigate, focus }]),
      openWindow,
    },
    addEventListener: (name: string, handler: (event: WorkerEvent) => Promise<void> | void) => {
      handlers.set(name, handler);
    },
  });
  vi.stubGlobal("caches", {});
  vi.resetModules();
  await import("./worker.ts");

  async function dispatch(name: string, event: WorkerEvent) {
    const completions: Promise<unknown>[] = [];
    const handler = handlers.get(name);

    if (handler === undefined) throw new Error(`Missing ${name} worker handler`);

    await handler({ ...event, waitUntil: (work: Promise<unknown>) => completions.push(work) });
    await Promise.all(completions);
  }

  return { dispatch, navigate, focus, openWindow, showNotification, setAppBadge };
}

describe("shared worker notifications", () => {
  it("suppresses the OS notification for a visible focused client and still updates its badge", async () => {
    const f = await worker({
      url: "https://smartfire.test/app/r/99",
      visibilityState: "visible",
      focused: true,
    });
    const options = { data: { badge: 7, path: "/rooms/12" }, tag: "room-12" };

    await f.dispatch("push", { data: { json: () => ({ title: "New message", options }) } });

    expect(f.showNotification).not.toHaveBeenCalled();
    expect(f.setAppBadge).toHaveBeenCalledExactlyOnceWith(7);
  });

  it.each([
    { url: "https://smartfire.test/app/", visibilityState: "visible", focused: false },
    { url: "https://smartfire.test/app/", visibilityState: "hidden", focused: true },
    { url: "https://other.test/app/", visibilityState: "visible", focused: true },
  ])("still shows a push for $url $visibilityState focused=$focused", async (client) => {
    const f = await worker(client);
    const options = { data: { badge: 3, path: "/rooms/12" }, tag: "room-12" };

    await f.dispatch("push", { data: { json: () => ({ title: "New message", options }) } });

    expect(f.showNotification).toHaveBeenCalledExactlyOnceWith("New message", options);
  });

  it("opens an old classic click URL unchanged, for the server to send on to the SPA", async () => {
    const f = await worker();
    const close = vi.fn();

    await f.dispatch("notificationclick", {
      notification: { close, data: { path: "/rooms/12/@34?classic=1&highlight=1#message" } },
    });

    expect(close).toHaveBeenCalledOnce();
    expect(f.navigate).toHaveBeenCalledExactlyOnceWith(
      "https://smartfire.test/rooms/12/@34?classic=1&highlight=1#message",
    );
    expect(f.focus).toHaveBeenCalledOnce();
    expect(f.openWindow).not.toHaveBeenCalled();
  });

  it.each([7, 0, null])("preserves the push payload and badge value %s", async (badge) => {
    const f = await worker();
    const options = { data: { badge, path: "/rooms/12" }, tag: "room-12" };

    await f.dispatch("push", { data: { json: () => ({ title: "New message", options }) } });

    expect(f.showNotification).toHaveBeenCalledExactlyOnceWith("New message", options);
    expect(f.setAppBadge).toHaveBeenCalledExactlyOnceWith(badge || 0);
  });
});
