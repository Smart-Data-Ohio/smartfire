# Smartfire screen oracle

The reference is the Rails app at the repository root, pinned to this checkout. The upstream
`rust/reference/` corpus is not the Smartfire reference. `screens.yml` describes **172 states in
23 feature areas**. Both targets receive copies of one built seed at 2026-03-02T16:00:00Z and
appear under the same browser origin, http://localhost:3999.

## Feature inventory

| Area | States | Primary states and interactions |
|---|---:|---|
| Channels | 19 | Markdown/media timeline, empty/busy/open channel, deep link, open/closed creation/settings, members, files, pins, categories, sidebar/unread, switcher, refresh frame |
| Composer | 7 | Markdown draft, Drive attachment menu, schedule dialog, mention suggestions, mobile header overflow, shortcut dialog, quick switcher |
| Threads | 13 | List, active/closed/locked, panel browser/create/conversation, content frame, message JSON, reply actions/forward source |
| DMs | 4 | Direct/group conversation, new DM, group settings |
| Voice | 7 | New/edit/empty/occupied, participants, unconfigured gateway responses |
| Stage | 7 | New/edit/empty/live, open participant panel, unconfigured gateway responses |
| Boards | 12 | Board/new/edit, new post, all four work states, owner/tag/state filters, automations |
| Work | 3 | Cross-board work list, links, handoff form |
| Events | 7 | List/new/edit/show/cancelled, attendance frame, live stage venue |
| Saved / scheduled | 2 / 2 | Pending/completed saved items; pending/history scheduled rows |
| Activity | 5 | Unread/all/mentions, unread count, inbox live frame |
| Search | 8 | Recent/results/empty/filtered, live frame, user/icon/command autocomplete |
| Agents | 3 | Directory, event ledger, approvals |
| Admin | 11 | Workspace settings, people frame, bots/new/edit/credentials/grants, icons, custom styles, audit, integration health |
| Integrations | 19 | Recorded GitHub/Fizzy/link/LinkedIn cards/frames; disconnected Drive; Slack settings/list/workspace and personal preview/plan/status/failure |
| Account | 10 | Profile with notification/status/security/integration preferences, sessions, directory, self/other user, human/agent cards, presence/huddle presence, push subscriptions |
| Auth | 12 | Sign-in/join/first-run/welcome, password-to-2FA/rejection, transfer form, pending 2FA/setup/rejection, sudo prompt/rejection |
| Public / PWA | 7 / 3 | About/privacy/terms, 404/422/500/502, offline page, manifest, service worker |
| Messages / polls | 7 / 2 | Message/actions/edit, forward source/destinations, boosts/form, open/closed poll |
| Realtime | 2 | Send channel message and reply through the thread panel on freshly reset servers |

Aliases of these screens do not multiply screenshots. Protocol routes (agent APIs, webhooks,
huddle authorization, OAuth round trips, direct uploads, media delivery) are captured in
`rust/vectors/campfire_routes.json`; page network logs also observe asset/media delivery. Rails
`resources` declares some actions with no implementation/template; `rooms/settings#show`, for
example, has no controller. A declared route is not proof of a reference screen.

`coverage/templates.txt` lists current templates and explicit `covers` declarations. UNCOVERED
stays visible. This breadth-first inventory does not claim every conditional partial,
destructive action, validation branch, or provider integration.

## Per-PR and nightly matrices

The lean gate has **288 cells**:

| Seed | States | Lean cells | Full cells before breakpoint sweep |
|---|---:|---:|---:|
| default | 159 | 261 | 2648 |
| unread | 1 | 8 | 24 |
| first_run | 1 | 1 | 24 |
| live_rooms | 5 | 12 | 97 |
| imports | 6 | 6 | 98 |
| Total | 172 | 288 | 2891 |

Ordinary states run once: Chromium, 1440x900 desktop, light. `smoke: true` runs Chromium
desktop/phone x light/dark plus Firefox/WebKit desktop/phone in light: eight cells. `realtime/**`
also runs every engine on desktop/light. Explicit matrix narrowing applies (the header menu is
phone only). Response fragments run once. Lean does not sweep widths.

Nightly/cutover uses `--matrix full`: Chromium/Firefox/WebKit x desktop 1440x900, laptop
1280x800, tablet 834x1194, phone 390x844 x light/dark, respecting narrowing. It adds widths one
pixel either side of stylesheet breakpoints for channels/timeline, account/profile and
auth/sign_in. Full self-parity defaults to two runs and an across-run comparison. The nightly
matrix is configured; WS19 acceptance evidence covers the entire lean inventory.

## Canonical commands

Run from the checkout containing `rust/`:

```sh
export PARITY_NAMESPACE=ws19 PARITY_OWNER=ws19
rust/parity/bin/reference build
rust/parity/bin/seed build default unread first_run live_rooms imports
rust/parity/bin/compare --self-parity --matrix lean --runs 2 --ports 49301,49302 --workers 4 --isolated 2
rust/parity/bin/compare --self-parity --matrix full --ports 49301,49302 --workers 4 --isolated 2
rust/parity/bin/candidate build
rust/parity/bin/candidate compare --seed default --ports 49001,49002 --workers 4 --matrix lean
```

Self-parity selects all inventory seeds automatically. Seed index i uses base +10i; fresh
mutation slot k uses base +100(k+1). Reserve base through base+250 for one invocation; do not run
another invocation on its mutation ports. Candidate compare takes one seed: run all five to
produce a complete baseline. Namespace images/containers/network with `PARITY_NAMESPACE` and
cleanup ownership with `PARITY_OWNER`. WS19 uses only 49000-49999. `Dockerfile.dev` is an optional
local shortcut copying an explicitly built current executable into the pinned candidate media
runtime; `candidate build` remains the canonical source build.

Seeds run serially by default, with parallel jobs and targets within each seed. This bounds
browser and app memory on the shared worker. `PARITY_SEED_JOBS` explicitly raises concurrent
seeds on a dedicated runner; `PARITY_VARIANT_WORKERS` controls the smaller seed job pools.

Capture containers have `--network none`. App instances use an **internal** Docker bridge with
no Internet route. `bin/forward-port` supplies host-loopback ingress; the Unix-socket capture
forwarder permits only loopback destinations. Seeded external images resolve to local fixtures;
all other external browser requests are blocked. Build-time downloads are separate.

## Inventory format and comparison

```yaml
states:
  - id: threads/panel_conversation
    area: threads
    path: /rooms/{{rooms.designers}}
    as: david
    seed: default
    steps:
      - click: '[data-action="thread-panel#toggle"]'
      - click: '.thread-panel__thread-item[data-thread-id="{{threads.launch}}"]'
      - wait_for: '#thread-panel .composer__textarea'
```

`{{table.label}}` resolves against that seed's labels. `as` installs a seeded reference-issued
verified session; challenge/enrollment contexts install app-issued pending-session cookies.
Unseeded users still use real password login with the remembered-device cookie. Auth states
exercise real forms. Wrong initial status or accidental redirect fails immediately.
`expect_final_status` checks a resulting full document; `expect_responses` asserts interaction
HTTP outcomes. Full-document Turbo fetches count as documents; card/frame fragments cannot
overwrite the document layer.

Pages compare exact pixels, normalized server HTML, live DOM, accessibility tree, response
header values/cookie values and attributes/body hash, and cable subscriptions/broadcasts.
Fragments compare status, all meaningful headers/cookies and normalized body. Digest-named
assets compare a full SHA-256 hash of the actual response bytes. Each cell has its own
proxy retaining upstream payloads before drivers discard response bodies; browser memory
cache reuse uses previously captured bytes, and animated-image routes record the original
full payload before freezing its first frame. Every distinct delivered asset payload counts. Normalized response text is saved in `.responses/` to diagnose
hashes. Repeated browser-cache requests collapse to the latest response per method/URL;
subscription and payload sets discard transport duplicates.

Readiness requires connected/registered Stimulus controllers, confirmed cable identifiers,
fonts/images, quiet network and stable DOM under deterministic fake-clock ticks. Two client
subscriptions to the same cable identifier need one confirmation. Both targets capture in
parallel, with four self-parity jobs by default. Interactive selectors/readiness fail after 8s;
cold navigation and initial readiness each have 20s. Failed script resources fail immediately.
Only renderer/network infrastructure crashes retry. Any retry disagreement fails the gate
with a non-zero exit, including a pixel-only difference that matches on a later capture.
Earlier screenshots/diffs and the per-capture result are retained; recompare cannot erase
recorded pixel flakes. Acceptance requires **zero** flaky cells. All engines discard stale
raster tiles before screenshotting; there is no pixel tolerance.

## Masks and remaining integration boundaries

There are **no state value masks, no pixel masks and no approved divergence entries**.
`allowlist.yml` documents inherited normalization cases and reasons. WS19 removed asset/PWA
body substitutions and the manifest exception. CSRF element structure remains; only token
bytes normalize. Signed IDs retain decoded record/purpose data; timestamps and frozen expiries
retain exact offsets. Crypto vectors separately exercise cookie/signature bytes. UI text,
missing elements, HTTP status and controller failures are never masked.

Live voice/stage states cover rendered participants and a synthetic live stream. Microphone/
camera permission, RTC media, gateway reconciliation, OAuth provider success/cancel/re-consent,
Drive's external Picker SDK, notification delivery, agent execution, periodic scheduling,
installed/offline service-worker lifecycle, destructive admin operations, first-run completion,
new secret/backup-code reveals and all validation branches remain integration extensions.
Static offline/manifest/worker bytes are covered. Rails loses wrong-password `flash.now` on
Turbo's required reload: that state asserts the 401 POST and captures the actual sign-in result
without modifying Rails.

## Seed and presence validation

Every capture/compare/recompare mode validates the selected seed using the reference's actual
models before capturing. `verify_parity_seed.rb` checks the default corpus plus unread, import
and live-room requirements; first_run instead checks the empty schema. Unknown seeds fail.
The host writes a receipt tied to the complete seed files, seed name, clock and validator
bytes. The networkless browser process verifies that fingerprint. Editing a seed after
validation invalidates the receipt. Standalone Node and the exported run function validate
through the same path; metadata-only list/seeds/breakpoints do not capture.

`channels/members`, `account/directory`, `account/self`, `account/other`, human/agent cards and the presence response
use `isolated: true`. Each cell gets fresh reference/candidate servers without concurrent
captures, independently of `mutates`. Omitted reset hooks fail before capture instead of
silently falling back to shared servers. The initial presence lease corpus must be empty;
normal cable subscriptions inside the isolated cell remain live. No Online/Offline text,
HTML, accessibility output or presence pixels are masked. Two fresh complete lean runs plus
an across-run comparison verify determinism.

Firefox gets a fresh browser process per cell because it retains visited-link colours across
contexts. This preserves real `:visited` styling while keeping other cells' history out of the
capture; no preference override or colour mask is used.

For deterministic request coverage, the harness fetches each declared same-origin lazy image
through its authenticated API request context at navigation and interaction boundaries,
without changing image attributes or rendered DOM.
Otherwise WebKit opportunistically fetching an offscreen timeline image races a thread-panel
transition. Their actual response headers and bytes remain compared. `channels/timeline` and
`realtime/thread_reply` explicitly scroll the timeline to the bottom after card/image or panel
reflow so WebKit scroll anchoring cannot choose two viewport positions for the same DOM.
Deep-link and unread-divider states retain their own initial positions. Non-GET bodies retain
the inherited boundary: their status/headers and resulting rendered state are compared,
but a submission is never replayed to recover a discarded response body.

Scratch sockets/browser profiles live under `PARITY_SCRATCH` (default
`/home/riels/.cache/rust-port/ws19/`); tests and reports use parity/out. The short socket path
also avoids Unix's 108-byte limit. No host scratch is written to /tmp.

Run the committed oracle fault suite with `node capture/test/injections/suite.ts after` from
rust/parity, using the WS19 namespace/image. It checks 21 header/cookie/asset divergences,
a transient first-capture pixel, a deleted-pins seed, and shared-server presence activity.
The unchanged control must pass, every fault must fail. `before` is for reproducing the old
98c96ef2 oracle with these injection fixtures. `bin/test-network-race` exercises concurrent
reference boots in three fresh namespaces. All are offline and keep the real seeds intact.

Set-Cookie fields and the attributes inside each cookie retain their transmitted order.
Later repeated cookie writes and repeated attributes can change the stored value or scope;
their order is never normalized. `before --ordering-only` reproduces the four page/fragment
false passes on 62e5b959; `after --ordering-only` requires all four to fail.

## Follow-up coverage wave

The review identified these gaps; they are not implemented by this oracle correction:
successful 2FA and the complete session lifecycle; permission boundaries; CRUD transitions;
cross-user realtime; agent execution; RTC; OAuth and Picker; periodic jobs; installed and
offline PWA behaviour; and **208 uncovered template declarations** in coverage/templates.txt.
These need behavioural coverage in addition to the current screen inventory.
