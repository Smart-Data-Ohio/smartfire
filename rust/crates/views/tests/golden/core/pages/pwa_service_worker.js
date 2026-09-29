// Smartfire service worker: push notifications and the offline shell.
//
// Caching policy (pinned by test/system/service_worker_test.rb): the only
// requests ever written to the cache are same-origin GETs for fingerprinted
// static assets under /assets/ and the offline shell page itself.
// Navigations always go to the network and fall back to the cached offline
// shell only when the network fails; API responses, authenticated HTML, and
// everything else are never cached and never served from the cache.
const STATIC_CACHE = "smartfire-static-v1"
const OFFLINE_URL = "/offline.html"

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(STATIC_CACHE)
      .then((cache) => cache.add(OFFLINE_URL))
      .then(() => self.skipWaiting())
  )
})

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys()
      .then((keys) => Promise.all(keys.filter((key) => key !== STATIC_CACHE).map((key) => caches.delete(key))))
      .then(() => self.clients.claim())
  )
})

self.addEventListener("fetch", (event) => {
  const { request } = event
  if (request.method !== "GET") return

  const url = new URL(request.url)
  if (url.origin !== self.location.origin) return

  if (isCacheableStaticAsset(url)) {
    event.respondWith(cacheFirst(request))
    return
  }

  if (request.mode === "navigate") {
    event.respondWith(networkThenOffline(request))
  }
  // Anything else (API calls, Turbo stream Response bodies, authenticated
  // HTML fragments) passes through to the network untouched.
})

// Only fingerprinted static assets and the offline shell are cacheable.
// Never extend this to authenticated HTML or API responses.
function isCacheableStaticAsset(url) {
  return url.pathname === OFFLINE_URL || url.pathname.startsWith("/assets/")
}

async function cacheFirst(request) {
  const cached = await caches.match(request, { cacheName: STATIC_CACHE })
  if (cached) return cached

  const response = await fetch(request)
  if (response.ok) {
    const cache = await caches.open(STATIC_CACHE)
    await cache.put(request, response.clone())
  }
  return response
}

async function networkThenOffline(request) {
  try {
    return await fetch(request)
  } catch (error) {
    const offline = await caches.match(OFFLINE_URL, { cacheName: STATIC_CACHE })
    if (offline) return offline
    throw error
  }
}

self.addEventListener("push", async (event) => {
  const data = await event.data.json()
  event.waitUntil(Promise.all([ showNotification(data), updateBadgeCount(data.options) ]))
})

async function showNotification({ title, options }) {
  return self.registration.showNotification(title, options)
}

async function updateBadgeCount({ data: { badge } }) {
  return self.navigator.setAppBadge?.(badge || 0)
}

self.addEventListener("notificationclick", (event) => {
  event.notification.close()

  const url = new URL(event.notification.data.path, self.location.origin).href
  event.waitUntil(openURL(url))
})

async function openURL(url) {
  const clients = await self.clients.matchAll({ type: "window", includeUncontrolled: true })
  const existing = clients.find((client) => {
    try {
      return new URL(client.url).origin === self.location.origin
    } catch {
      return false
    }
  })

  if (existing) {
    try {
      await existing.navigate(url)
    } catch {
      // A failed navigation (a race with the tab closing) still leaves a
      // tab to focus rather than opening a duplicate.
    }
    return existing.focus()
  }

  return self.clients.openWindow(url)
}
