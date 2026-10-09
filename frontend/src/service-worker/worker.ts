/// <reference lib="webworker" />
import { pageBuildMessage, REQUEST_PAGE_BUILD } from "./messages.ts";
import { installNetworkRoute } from "./routing.ts";
import { type CacheCoordinator, type WorkerBuild, WorkerCache } from "./runtime.ts";

declare const self: ServiceWorkerGlobalScope & { readonly smartfireBuild: WorkerBuild };

const coordinator: CacheCoordinator | null =
  self.navigator.locks === undefined
    ? null
    : { run: (work) => self.navigator.locks.request("smartfire-spa-cache", work) };

const cache = new WorkerCache(
  self.smartfireBuild,
  self.location.origin,
  caches,
  fetch,
  coordinator,
);

const pageBuilds = new Map<string, string>();

let maintenance: Promise<void> = Promise.resolve();

/** The snapshot, rather than remembered IDs, decides which pages are still alive. */
async function livePages(): Promise<readonly (string | null)[]> {
  const windows = await self.clients.matchAll({ type: "window", includeUncontrolled: true });
  const ids = new Set(windows.map((client) => client.id));

  for (const id of pageBuilds.keys()) {
    if (!ids.has(id)) {
      pageBuilds.delete(id);
    }
  }

  return windows.map((client) => {
    const page = pageBuilds.get(client.id) ?? null;

    if (page === null) {
      client.postMessage({ kind: REQUEST_PAGE_BUILD });
    }

    return page;
  });
}

/** Activation, fetch and message pruning must not delete a cache from a stale client snapshot. */
function maintain(work: () => Promise<void>): Promise<void> {
  const pending = maintenance.then(work);

  maintenance = pending.catch(() => undefined);

  return pending;
}

self.addEventListener("install", (event) => {
  event.waitUntil(
    Promise.all([
      installNetworkRoute(event, self.location.origin, self.smartfireBuild),
      cache.install(),
    ]).then(() => self.skipWaiting()),
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    maintain(async () => {
      await self.clients.claim();
      await cache.activate(await livePages());
    }),
  );
});

self.addEventListener("message", (event) => {
  const page = pageBuildMessage(event, self.location.origin);
  const source = event.source;

  if (page === null || source === null || !("id" in source) || source.type !== "window") {
    return;
  }

  event.waitUntil(
    maintain(async () => {
      // A reload reports its own entry and replaces any earlier identity for this client.
      pageBuilds.set(source.id, page);
      await cache.prune(await livePages());
    }),
  );
});

self.addEventListener("fetch", (event) => {
  if (!cache.handles(event.request)) {
    return;
  }

  const pruning = maintain(async () => cache.prune(await livePages()));

  event.waitUntil(pruning);
  event.respondWith(
    pruning
      .then(() => cache.response(event.request))
      .then((response) => response ?? fetch(event.request)),
  );
});

// The classic push payload and badge behaviour stay shared with installed subscriptions.
self.addEventListener("push", async (event) => {
  const data = await event.data?.json();

  event.waitUntil(
    Promise.all([
      self.registration.showNotification(data.title, data.options),
      updateBadgeCount(data.options),
    ]),
  );
});

async function updateBadgeCount(options: { readonly data: { readonly badge: number | null } }) {
  return self.navigator.setAppBadge?.(options.data.badge || 0);
}

self.addEventListener("notificationclick", (event) => {
  event.notification.close();
  event.waitUntil(openNotification(event.notification.data.path));
});

/** Exported in the worker's IIFE for browser-context verification; there is no message hook. */
export async function openNotification(path: string): Promise<WindowClient | null> {
  // The server's aliases select the person's UI, including an explicit classic preference.
  const url = new URL(path, self.location.origin).href;
  const clients = await self.clients.matchAll({ type: "window", includeUncontrolled: true });

  const existing = clients.find((client) => {
    try {
      return new URL(client.url).origin === self.location.origin;
    } catch {
      return false;
    }
  });

  if (existing !== undefined) {
    try {
      await existing.navigate(url);
    } catch {
      // A closing-tab race still focuses the existing tab, just as the classic worker does.
    }

    return existing.focus();
  }

  return self.clients.openWindow(url);
}
