# Installed app

Smartfire uses one service worker at `/service-worker.js`, registered with scope `/` by the SPA
and retained auth pages. `campfire_spa::pwa` owns the manifest, fallback worker, offline page and
illustrations. A production build supplies the SPA worker; Cargo-only builds use the standalone
fallback. `/app/service-worker.js` remains a compatibility alias with the same bytes.

Retained auth pages register on load. The SPA boot names `/service-worker.js` and quietly calls
`registration.update()` on startup; development builds skip registration. Updating the script
preserves the registration and existing push subscription.

The manifest at `/webmanifest.json` preserves its absent `id`, `start_url: "/"` and `scope: "/"`.
`/app/manifest.webmanifest` redirects there. Shortcuts always open SPA destinations.
Manifests use private, revalidating caching.

## Offline shell and caching policy

The root offline page uses the page built from `frontend/offline.html`, with a standalone
fallback when no dist exists.
`/app/offline.html` remains a compatibility alias. Its module scripts and CSS
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
- Retained media under `/assets/` retain cache-first runtime caching.
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
Activation records retain their original `/app/offline.html.activation` key across the
move to the root offline URL. Activation records determine the previous version, including when a failed
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

If a lazy module still fails to load, the SPA records that an update is required
(`useAppUpdateRequired()`) and asks the server whether a newer build exists: it
fetches `/app/` with `cache: "no-store"` and compares that page's entry module
with the one this tab runs (`newerBuildAvailable()`). A strip then slides open over
the panes with a Reload button (`reloadForUpdate()`). When the server names a
different entry it says "Smartfire has been updated. Reload to get the latest
version." Otherwise, including offline and failed checks, it says "Couldn't load
part of Smartfire. Reload to try again." A later failure re-checks, so a deploy
after an offline blip still gets the update wording. The page never reloads by
itself, so drafts and open panes survive until the person chooses. Module-import guards and `vite:preloadError`
detection keep a failed import from crashing the view. They do not catch unrelated
component render errors or API failures.

The SPA worker cleans only its own cache namespace. It leaves the classic
`smartfire-static-v1` cache in place so classic assets remain available and
configuration rollback does not discard the classic cache.

The standalone fallback is the Rails worker byte for byte except for one exception,
kept as a documented patch (`SERVICE_WORKER_SPA_PATCH` in `crates/spa/src/pwa.rs`)
that the parity tests apply to the Rails source before comparing. Its activation
cleanup keeps `smartfire-spa-*` caches, and it answers `/app/assets/` requests from
them (ignoring Vary, like the SPA worker) before falling back to the network. It
never writes to those caches. An SPA tab still open on an older build can keep
loading its unloaded chunks after the SPA is disabled.

## Notifications

Both workers use the classic push handler to show notifications and update the
app badge. Push tags group room messages, event reminders, saved-item reminders
and huddle invitations rather than stacking repeated notifications.

Notification clicks open the payload's path unchanged. Existing server aliases select the
person's UI and preserve old room, message and email links, including explicit classic
choices. The handler navigates and focuses an existing workspace window, or opens one
when none exists. Push handlers retain the same app badge behavior.

Settings → Push devices has a "This browser" row: notifications off (with an
"Enable notifications" button), on, blocked by the browser, or unsupported. "On"
needs the browser's permission, a subscription and a saved device whose endpoint
matches it. A browser that subscribed but whose save failed shows "Notifications
aren't set up yet" with a "Finish setting up" button, which saves the existing
subscription without prompting again. While the device list loads the row says it is
checking. The
button calls `enablePushNotifications()`, the only place that requests permission.
It registers the selected script at root scope,
subscribes with the same VAPID public key as the classic page, and saves through
the classic subscription creation path. `usePushEnrollment()` exposes permission,
subscription, endpoint and busy state for that row. The flow refreshes the
existing Devices list. Removing the last saved subscription with the current
browser's endpoint also unsubscribes it locally; another saved key triple for
that endpoint keeps the browser subscribed.

`GET /api/v1/settings/push_subscriptions/key` returns the public key, and
`POST /api/v1/settings/push_subscriptions` accepts endpoint, p256dhKey and authKey.
These personal settings routes require a human session. Creation preserves the
classic key-triple deduplication, user-agent and user-ownership rules; the existing
subscription table has no session link. Deletion preserves the classic destroy
behaviour.

Unit tests cover root registration, legacy activation metadata, request policy, no-store responses and cache
retention. Rust tests cover selection, manifests, embedding and response headers.
The production-preview Playwright suite exercises offline navigation, worker
click handling and successive worker updates. It runs in the first Frontend e2e
shard beside the mock suite. The separate Rust PWA browser suite has been removed.
A healthy deployment checks the public auth pages and the SPA's offline
shell, including their JS and CSS responses, without a browser.
