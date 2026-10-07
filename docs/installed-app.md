# Installed app

Smartfire has a classic worker at `/service-worker.js` and an SPA worker at
`/app/service-worker.js`. Both register with scope `/`. The request's effective UI
selects the worker: `SPA_ENABLED` off always means classic; otherwise the person's
stored preference wins over `SPA_DEFAULT`. Signed-out requests use `SPA_DEFAULT`.

Classic pages register the selected URL from their layout, including pages that
remain classic for a person using the SPA. SPA pages register only for the next UI.
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
claims clients, then retains the current cache and the immediately preceding
activated version. Activation records determine this order, including when a
failed installation or a rollback creates caches out of order. Tabs still running
the previous build can load their cached chunks after a deploy. The following
activation removes that previous version.

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

There is no SPA flow for enabling push in this browser yet. Its settings screen
lists and removes existing devices.

Unit tests cover URL mapping, request policy, no-store responses and cache
retention. Rust tests cover selection, manifests, embedding and response headers.
The production-preview Playwright suite exercises offline navigation, worker
click handling and successive worker updates. It runs in the `Frontend` CI job;
the separate `Frontend e2e` jobs continue to run the mock suite.
