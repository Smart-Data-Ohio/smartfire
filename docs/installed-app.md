# Installed app

Smartfire has a classic worker at `/service-worker.js` and an SPA worker at
`/app/service-worker.js`. Both register with scope `/`. The request's effective UI
selects the worker: `SPA_ENABLED` off always means classic; otherwise the person's
stored preference wins over `SPA_DEFAULT`. Signed-out requests use `SPA_DEFAULT`.

Classic pages register the selected URL from their layout, including pages that
remain classic for a person using the SPA. SPA pages register only for the next UI.
The classic selection lives in a provisional head meta element, which Turbo replaces
on each visit. Registration reconciles on both `load` and `turbo:load`, so signing
in as someone with a different preference also replaces the selected script. It
never unregisters the root registration.
The boot JSON carries that URL, or null. Mock boot data leave it null, and
development builds skip registration. Classic automatic registration still respects `data-service-worker="false"`
in the test environment. The SPA quietly calls `registration.update()` on startup.

Registering a different script at the same scope updates the existing registration;
it does not create a second one or unregister the first. Existing push subscriptions
remain associated with that registration. Switching to classic registers the
classic script again. These behaviours follow the
[Service Worker registration algorithm](https://w3c.github.io/ServiceWorker/#register-algorithm)
and the [Push API's registration relationship](https://w3c.github.io/push-api/#relationship-to-service-worker-registrations).

The SPA shell links to `/app/manifest.webmanifest`; classic pages keep their classic
manifest. The SPA manifest reuses the classic name, icons, colours and shortcut fields.
Its start URL and scope share one setting derived from `campfire_spa::PREFIX`.
The profile shortcut maps to `/app/settings`. The classic New chat room URL is
outside the SPA's scope, so browsers discard that shortcut under the
[manifest shortcut rules](https://www.w3.org/TR/appmanifest/#processing-shortcut-items).
It remains in the shared manifest data until room creation has a mapped SPA screen.

## Offline shell and caching policy

The classic worker retains its existing offline shell at `/offline.html` and
runtime caching for `/assets/`. The SPA worker uses the page built from
`frontend/offline.html`, served at `/app/offline.html`. Its module scripts and CSS
are same-origin content-hashed build assets, cached during installation. This page
does not contain boot JSON or an authenticated shell.

The Vite build generates the precache list from every hashed file under `assets/`
and `offline.html`. The stable `service-worker.js` imports a hashed worker runtime.
A hashed copy of the offline HTML also contributes to the list, so worker-only or
HTML-only changes produce a new version. The version comes from the sorted file
names; identical builds produce the same version. No list is maintained by hand.

The SPA worker handles only same-origin GET requests:

- Precached assets use cache first, including assets retained from the previous build.
- Navigations use the network. Only a failed network request falls back to the
  precached offline page. Navigation responses and the authenticated shell are
  never cached.
- Classic static assets under `/assets/` retain cache-first runtime caching.
- API requests, `/cable`, `/rails/`, uploads, account data, transfer links, QR codes
  and other non-navigation requests pass through untouched.
- A response with `Cache-Control: no-store` is never written to a cache.
- Partial responses and `Vary: *` responses also pass through, because the
  [Cache API cannot store them](https://w3c.github.io/ServiceWorker/#cache-put).

The worker response has `Service-Worker-Allowed: /`, JavaScript content type and
`Cache-Control: no-cache`. The offline page uses the app's revalidating file policy.
Neither stable URL is immutable. Same-origin scripts work with the app's content
security policy.

Installation precaches the new build before calling `skipWaiting`. Activation
claims clients, then retains the current cache, the immediately preceding activated
version, and caches needed by every live window. Pages report the identity of their
actual bundled entry script, including after a controller change. If an entry is
shared by several builds, all matching caches survive. An unknown client keeps
retention conservative, including tabs loaded before this reporting protocol.
Activation records determine the previous version, including when a failed
installation or a rollback creates caches out of order. Activation and later
fetches/messages check live clients again; closed tabs stop retaining their builds.
An unactivated cache also survives: another worker may still be installing it.
This conservatively retains failed-install remnants rather than interrupting a
concurrent installation. There is no worker timer to keep it alive.

Installations pin their cache before fetching, including when a rollback reuses
an activated cache. Short [Web Locks](https://www.w3.org/TR/web-locks/) serialize
that pin and activation metadata with pruning across worker instances; the lock
does not span network fetching. Failed installation pins stay retained. Browsers
without Web Locks keep serving and installing but skip cache pruning.

If a lazy module still fails to load, the SPA records that an update is required.
The `useAppUpdateRequired()` hook and `reloadForUpdate()` function are the interface
for a future shell prompt. Module-import guards and `vite:preloadError` detection
keep a failed import from crashing the view. They do not catch unrelated component
render errors or API failures. No automatic reload or visible prompt is added here.

The SPA worker cleans only its own cache namespace. It leaves the classic
`smartfire-static-v1` cache in place so classic assets remain available and
switching workers does not discard the classic cache. The classic worker's
existing activation cleanup is unchanged.

## Notifications

Both workers use the classic push handler to show notifications and update the
app badge. Push tags group room messages, event reminders, saved-item reminders
and huddle invitations rather than stacking repeated notifications.

The SPA worker maps a notification's classic path through the generated screen
map. Only ported rows map to SPA routes, and the query string is retained. Unknown,
unported and foreign-origin paths keep their original destination. Unmapped
classic destinations can still redirect on the server according to the effective
UI. The click handler navigates and focuses an existing workspace window, or
opens one when none exists.

The SPA exposes `enablePushNotifications()` for an explicit click. It requests
permission only through that call, registers the selected script at root scope,
subscribes with the same VAPID public key as the classic page, and saves through
the classic subscription creation path. `usePushEnrollment()` exposes permission,
subscription and busy state for the future Devices control. The flow refreshes the
existing Devices list. Removing the last saved subscription with the current
browser's endpoint also unsubscribes it locally; another saved key triple for
that endpoint keeps the browser subscribed. No enrollment control is rendered yet.

`GET /api/v1/settings/push_subscriptions/key` returns the public key, and
`POST /api/v1/settings/push_subscriptions` accepts endpoint, p256dhKey and authKey.
These personal settings routes require a human session. Creation preserves the
classic key-triple deduplication, user-agent and user-ownership rules; the existing
subscription table has no session link. Deletion preserves the classic destroy
behaviour.

Unit tests cover URL mapping, request policy, no-store responses and cache
retention. Rust tests cover selection, manifests, embedding and response headers.
The production-preview Playwright suite exercises offline navigation, worker
click handling and successive worker updates. It runs in the `Frontend` CI job;
the separate `Frontend e2e` jobs continue to run the mock suite.
The `pwa` Rust correctness job serves the embedded production build from a frozen
seed and drives the real registration code through Turbo sign-in and both UI
switches. It checks a persistent Chromium profile's root registration identity
and native push subscription. Locally, Chromium created a subscription and kept
its endpoint and keys through SPA → classic → SPA → classic. A browser that
cannot reach its push service reports that limitation and still checks the root
registration's identity and scope; it does not substitute a fake subscription.
