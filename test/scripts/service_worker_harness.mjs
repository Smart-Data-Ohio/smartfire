// Behavioral tests for app/views/pwa/service_worker.js, executed by
// PwaControllerTest ("service worker fetch and notification logic").
// Builds a minimal ServiceWorkerGlobalScope stand-in, evaluates the real
// worker source, and drives synthetic install/fetch/push/notificationclick
// events through it. No dependencies; run with `node
// test/scripts/service_worker_harness.mjs` from the repo root.
import { readFileSync } from "node:fs"

const ORIGIN = "http://test.host"
const OFFLINE_BODY = "<h1>offline shell</h1>"
const ROOM_BODY = "<h1>a room</h1>"

let failures = 0
function check(name, condition, detail = "") {
  if (condition) {
    console.log(`ok - ${name}`)
  } else {
    failures += 1
    console.log(`FAIL - ${name} ${detail}`)
  }
}

// --- Minimal Cache Storage stand-in -------------------------------------
function absoluteKey(key) {
  const raw = typeof key === "string" ? key : key.url
  return new URL(raw, ORIGIN).href
}

const cacheStore = new Map()
function openCache(name) {
  if (!cacheStore.has(name)) cacheStore.set(name, new Map())
  const entries = cacheStore.get(name)
  return {
    async add(key) {
      const response = await stubFetch(key)
      if (!response.ok) throw new Error(`cache.add of ${key} got a bad response`)
      entries.set(absoluteKey(key), response)
    },
    async put(key, response) {
      entries.set(absoluteKey(key), response)
    },
    async match(key) {
      return entries.get(absoluteKey(key))
    },
    async keys() {
      return [...entries.keys()].map((href) => ({ url: href }))
    }
  }
}

// --- Fetch stub with per-test behavior ------------------------------------
let fetchBehavior = async () => ({ ok: true, body: ROOM_BODY, clone() { return this } })
let fetchCalls = []
async function stubFetch(input) {
  const url = typeof input === "string" ? new URL(input, ORIGIN).href : input.url
  fetchCalls.push(url)
  return fetchBehavior(url, input)
}

// --- ServiceWorkerGlobalScope stand-in -------------------------------------
const listeners = {}
const clientsState = { list: [], opened: [] }
const notificationsShown = []
let badgeValue = null

globalThis.caches = {
  open: async (name) => openCache(name),
  keys: async () => [...cacheStore.keys()],
  match: async (key, { cacheName } = {}) => {
    if (cacheName) return (await openCache(cacheName)).match(key)
    for (const name of await globalThis.caches.keys()) {
      const hit = await (await openCache(name)).match(key)
      if (hit) return hit
    }
    return undefined
  }
}
globalThis.fetch = stubFetch
globalThis.self = {
  location: new URL(`${ORIGIN}/service-worker.js`),
  skipWaiting: async () => {},
  clients: {
    matchAll: async () => clientsState.list,
    openWindow: async (url) => {
      clientsState.opened.push(url)
      return { url }
    },
    claim: async () => {}
  },
  registration: {
    showNotification: async (title, options) => {
      notificationsShown.push({ title, options })
    }
  },
  navigator: {
    setAppBadge: async (value) => { badgeValue = value }
  },
  addEventListener: (type, handler) => {
    listeners[type] ??= []
    listeners[type].push(handler)
  }
}

// --- Load the real worker ---------------------------------------------------
const source = readFileSync(new URL("../../app/views/pwa/service_worker.js", import.meta.url), "utf8")
eval(source)

// --- Event drivers -----------------------------------------------------------
async function fireLifecycle(type) {
  const pending = []
  for (const handler of listeners[type] ?? []) {
    handler({ waitUntil: (promise) => pending.push(promise) })
  }
  await Promise.all(pending)
}

async function fireFetch(url, { mode = "navigate", method = "GET" } = {}) {
  const request = { url: new URL(url, ORIGIN).href, method, mode, clone() { return this } }
  let responsePromise = null
  const event = { request, respondWith: (promise) => { responsePromise = promise } }
  for (const handler of listeners.fetch ?? []) handler(event)
  if (!responsePromise) return "PASSTHROUGH"
  return await responsePromise
}

async function firePush(data) {
  const pending = []
  const event = {
    data: { json: async () => data },
    waitUntil: (promise) => pending.push(promise)
  }
  for (const handler of listeners.push ?? []) await handler(event)
  await Promise.all(pending)
}

async function fireNotificationClick(path) {
  const pending = []
  let closed = false
  const event = {
    notification: { close: () => { closed = true }, data: { path } },
    waitUntil: (promise) => pending.push(promise)
  }
  for (const handler of listeners.notificationclick ?? []) await handler(event)
  await Promise.all(pending)
  return closed
}

function cachedPaths() {
  const paths = []
  for (const entries of cacheStore.values()) {
    for (const href of entries.keys()) paths.push(new URL(href).pathname)
  }
  return paths.sort()
}

// --- The tests ----------------------------------------------------------------
fetchBehavior = async (url) => {
  if (new URL(url).pathname === "/offline.html") {
    return { ok: true, body: OFFLINE_BODY, clone() { return this } }
  }
  return { ok: true, body: `network:${new URL(url).pathname}`, clone() { return this } }
}

await fireLifecycle("install")
check("install precaches the offline shell", cachedPaths().includes("/offline.html"))

await fireLifecycle("activate")
check("activate keeps the static cache", (await caches.keys()).includes("smartfire-static-v1"))

const roomResponse = await fireFetch("/rooms/123", { mode: "navigate" })
check("navigations serve the network response", roomResponse.body === "network:/rooms/123")
check(
  "navigations are never cached",
  !cachedPaths().includes("/rooms/123"),
  JSON.stringify(cachedPaths())
)

fetchBehavior = async () => { throw new TypeError("Failed to fetch") }
const offlineResponse = await fireFetch("/rooms/123", { mode: "navigate" })
check("failed navigations fall back to the offline shell", offlineResponse.body === OFFLINE_BODY)

fetchBehavior = async (url) => ({ ok: true, body: `network:${new URL(url).pathname}`, clone() { return this } })
fetchCalls = []
await fireFetch("/assets/application-abc123.js")
await fireFetch("/assets/application-abc123.js")
check("static assets are cached after the first fetch", fetchCalls.length === 1, JSON.stringify(fetchCalls))
check("static assets stay cached", cachedPaths().includes("/assets/application-abc123.js"))

for (const [ name, init ] of [
  [ "API responses pass through", [ "/rooms/123/messages.json", { mode: "cors" } ] ],
  [ "same-origin fetches pass through", [ "/users/me/sidebar", { mode: "cors" } ] ],
  [ "non-GET navigations pass through", [ "/rooms/123", { mode: "navigate", method: "POST" } ] ]
]) {
  check(name, (await fireFetch(...init)) === "PASSTHROUGH")
}
check(
  "cross-origin requests pass through",
  (await fireFetch("https://cdn.example.com/app.js", { mode: "no-cors" })) === "PASSTHROUGH"
)
check(
  "only static entries are ever cached",
  cachedPaths().every((path) => path === "/offline.html" || path.startsWith("/assets/")),
  JSON.stringify(cachedPaths())
)

await firePush({ title: "All Talk", options: { body: "hi", tag: "room-1", data: { path: "/rooms/1", badge: 3 } } })
check("push shows the notification with its tag", notificationsShown.at(-1)?.options.tag === "room-1")
check("push updates the badge count", badgeValue === 3)

const focusedTabs = []
clientsState.list = [
  {
    url: `${ORIGIN}/rooms/9`,
    navigate: async function (url) { this.url = url },
    focus: async function () { focusedTabs.push(this.url); return this }
  }
]
const closed = await fireNotificationClick("/rooms/1")
check("notification click closes the notification", closed === true)
check("notification click navigates the existing tab", clientsState.list[0].url === `${ORIGIN}/rooms/1`)
check("notification click focuses the existing tab", focusedTabs.length === 1)
check("notification click opens no new window", clientsState.opened.length === 0)

clientsState.list = []
clientsState.opened = []
await fireNotificationClick("/rooms/2")
check(
  "notification click opens a window when none exists",
  clientsState.opened.length === 1 && clientsState.opened[0] === `${ORIGIN}/rooms/2`
)

if (failures > 0) {
  console.log(`${failures} service worker harness check(s) failed`)
  process.exitCode = 1
} else {
  console.log("service worker harness: all checks passed")
}
