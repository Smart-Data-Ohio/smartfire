/// <reference lib="webworker" />
import { classicToSpaUrl } from "../lib/screens.ts";
import { type WorkerBuild, WorkerCache } from "./runtime.ts";

declare const self: ServiceWorkerGlobalScope & { readonly smartfireBuild: WorkerBuild };

const cache = new WorkerCache(self.smartfireBuild, self.location.origin, caches, fetch);

self.addEventListener("install", (event) => {
  event.waitUntil(cache.install().then(() => self.skipWaiting()));
});

self.addEventListener("activate", (event) => {
  event.waitUntil(self.clients.claim().then(() => cache.activate()));
});

self.addEventListener("fetch", (event) => {
  if (!cache.handles(event.request)) {
    return;
  }

  event.respondWith(
    cache.response(event.request).then((response) => response ?? fetch(event.request)),
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
  const mapped = classicToSpaUrl(path, self.location.origin) ?? path;
  const url = new URL(mapped, self.location.origin).href;
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
