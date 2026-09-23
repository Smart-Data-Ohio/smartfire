# Installed app

Smartfire installs as a PWA (manifest + icons from `PwaController`) and
registers its service worker (`/service-worker.js`, served from
`app/views/pwa/service_worker.js`) on every page load, so both installed
and tabbed use get push notifications and the offline shell.

## Offline shell and caching policy

When a navigation fails because the network is unreachable, the worker
serves the cached offline shell (`public/offline.html`): "You're offline
— reconnecting…", with a retry button and an automatic reload when the
browser reports it is back online. The shell is a static file with
everything inline — no sign-in, no session, no extra requests — so it
renders straight from the install-time cache.

The worker's caching policy is deliberately narrow:

- Only same-origin `GET` requests for fingerprinted static assets under
  `/assets/` and the offline shell itself are ever written to the cache.
- Navigations always go to the network; the shell is a fallback for a
  failed network, never a substitute for a response.
- API calls, Turbo Stream bodies, authenticated HTML fragments, and
  everything else pass through to the network untouched and are never
  cached.

`test/system/service_worker_test.rb` pins this from the browser side: it
browses rooms with a controlling worker, inventories Cache Storage, and
asserts every entry is `/offline.html` or under `/assets/`. The worker's
fetch, push, and click branches are additionally driven through a Node
harness (`test/scripts/service_worker_harness.mjs`, run by
`PwaControllerTest`) against the real worker source.

## Notifications

Push payloads carry a `tag` so notifications group instead of stacking:

- Room and thread messages: `room-<id>` — the latest notification per
  room wins.
- Event reminders: `event-<id>`; saved-item reminders: `saved-<id>`;
  huddle invitations: `huddle-<room id>`.

The tag is required on `WebPush::Notification`: every payload builder
passes one. Clicking a notification focuses the existing workspace tab
(navigating it to the notification's path) instead of opening a
duplicate window, and opens a new window only when no workspace tab
exists.
