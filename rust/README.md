# Smartfire in Rust

This tree implements Smartfire, the Rails app at the repository root, in Rust. It derives from
37signals' MIT-licensed [ONCE Campfire Rust port](https://github.com/basecamp/once-campfire-rust),
imported with its full history by `git subtree add --prefix=rust` at upstream `24c85975`.
We do not track subsequent upstream changes. See [`MIT-LICENSE`](MIT-LICENSE).

## Current status

Smartfire's boards, threads, huddles and stages, Slack import, Google and GitHub integrations,
agent APIs, and two-factor authentication have controllers, jobs and views in this tree.
The former stock-Campfire-only status is obsolete; the current handler table is
[`crates/campfire/src/controllers.rs`](crates/campfire/src/controllers.rs#L195), with integration
routes also registered in [`app.rs`](crates/campfire/src/app.rs#L303). This is implementation
status, not a claim that every original Rails assertion or the production cutover has passed.
The remaining behavioral gaps and exceptions are listed under [Known differences](#known-differences).

The Rails app remains the behavior oracle. The port owns its copies of the Rails app's static
inputs: the assets, JavaScript, vendored JavaScript, `public/`, importmap and LiveKit scripts in
[`web/`](web) (laid out like the Rails app, which reads them through symlinks) and the test
fixtures in [`fixtures/`](fixtures). `CAMPFIRE_REFERENCE` names another Rails app's absolute root
to read them from instead; the tools that run Rails use it, defaulting to the repository root. Canonical Rails oracle images and generated seeds
use [`parity/reference.sha`](parity/reference.sha); approved source copies are described in
[`crates/assets/OVERRIDES.md`](crates/assets/OVERRIDES.md). The port uses the Rails SQLite schema,
storage layout and compatible signed/AES-GCM cookies. Boot loads an empty database's compiled
schema or checks an existing database's exact migration set; it does not run Rails migrations or
add the former upstream extra message index ([schema preparation](crates/db/src/schema.rs#L103)).
Apply needed Rails migrations before switching runtimes.

### Current behavior

These contracts supersede the imported upstream operational claims:

- **CSRF and sessions:** Rails-compatible masked global and per-form authenticity tokens are
  checked and rendered in meta tags and forms, including forms opened before a runtime switch
  ([kit CSRF](crates/kit/src/csrf.rs#L1), [view helpers](crates/views/src/helpers/request_forgery.rs#L113),
  [Rails vectors](crates/kit/tests/rails_vectors.rs)). The encrypted session holds CSRF, flash,
  return destinations and sign-in/step-up state, rather than only flash and a return URL
  ([session](crates/kit/src/session.rs#L1), [session keys](crates/campfire/src/concerns/session_keys.rs)).
- **Background jobs:** Redis/Resque are replaced by the durable `campfire_jobs` queue in SQLite's
  `background_jobs`. Production jobs persist with the originating transaction, wake after commit,
  survive restarts, and use claims and retry/discard policies; the ad hoc best-effort helper is
  test-only ([job integration](crates/campfire/src/jobs.rs#L1), [queue store](crates/jobs/src/store.rs)).
- **Web Push:** 410 and OpenSSL-equivalent key/TLS failures invalidate a subscription; 404 is
  retained. Notification titles and bodies are preserved, and oversized encryption raises rather
  than truncating. The VAPID subject is fixed to `mailto:support@smartdata.net`; `VAPID_SUBJECT` and
  `TLS_DOMAIN` do not configure it ([delivery/build](crates/campfire/src/integrations/web_push.rs#L48),
  [invalidation](crates/campfire/src/integrations/web_push.rs#L115),
  [size regression](crates/campfire/src/integrations/web_push/encryption.rs#L192),
  [configuration regression](crates/campfire/src/config.rs#L282)).
- **Routes, assets and rich text:** `/rooms/directs/:id` preserves Rails' inherited nil-room 500;
  the working conversation page is `/rooms/:id`
  ([handler](crates/campfire/src/controllers/rooms/directs.rs#L14)). Copy-link markup uses Rails'
  `content` value, without the removed upstream clipboard JavaScript override
  ([asset history](crates/assets/OVERRIDES.md#L17)). The sanitizer keeps `name` on anchors and removes
  it elsewhere, and the current allowlists strip `style`, including highlight colors
  ([sanitizer](crates/richtext/src/sanitizer.rs#L244), [style regression](crates/richtext/src/sanitizer.rs#L521)). A still-valid User SGID whose
  row was deleted reproduces Rails' missing-user partial error; other missing attachments can
  render ☒ ([renderer](crates/richtext/src/attachables.rs#L364)).
- **PWA manifest:** values retain Rails ERB HTML escaping, including `&amp;` in the small-logo URL;
  the former upstream JSON-escaping divergence is gone
  ([manifest](crates/views/src/pwa.rs#L11), [template](crates/views/templates/pwa/manifest.json)).

### Verification and CI

[`../.github/workflows/rust.yml`](../.github/workflows/rust.yml) restores the committed
**`default`, `first_run`, `agents_ui` and `ledger_originals`** seeds (built once by Rails, now
frozen) and checks them against this build's migrations; it never runs Rails. Missing seeds
fail seed-dependent tests when `CI` is set; local tests can skip with a message. Report which
seeds and test groups actually ran. See [`parity/seeds/README.md`](parity/seeds/README.md).
Some workspace/doctest steps remain advisory (`continue-on-error`), so a successful CI run alone
is not evidence that the complete correctness suite or a production-copy rehearsal passed.

The current screen oracle has **172 states and 288 lean cells**, documented with canonical
commands in [`parity/SCREENS.md`](parity/SCREENS.md). These are inventory sizes, not pass totals.
[`parity/allowlist.yml`](parity/allowlist.yml) has no approved Smartfire divergences: it documents
normalization of entropy and runtime framing while keeping application/security headers,
cookie attributes, asset bytes and PWA bodies visible. The former upstream screenshot totals and
manifest allowlist are not current Smartfire acceptance evidence.

From `rust/`, with the Dockerfile's Rust toolchain:

```sh
export CARGO_BUILD_JOBS=4
python3 parity/bin/frozen-seeds restore
cargo nextest run --locked --workspace --exclude html5ever -j 4
cargo test --locked --workspace --exclude html5ever --doc -- --test-threads=4
cargo clippy --locked --workspace --exclude html5ever --all-targets -- -D warnings
```

After adding a migration, `python3 parity/bin/frozen-seeds migrate target/debug/campfire`
applies it to the committed seeds. The screen matrix uses its own additional seeds and commands
in `parity/SCREENS.md`. Golden vectors and fixtures were recorded from Rails and are frozen. See [`AGENTS.md`](AGENTS.md) for layout and working rules.

## Building and deploying

This repository's deployable Rust image is private in **GCP Artifact Registry**, tagged
**`rust-git-<full Git SHA>`** and built for **`linux/amd64`** by
[`publish-rust-image.yml`](../.github/workflows/publish-rust-image.yml). Pull requests build
without publishing; main pushes publish or resolve an existing immutable tag. The registry image
path comes from the tracked workflow's `GCP_IMAGE` repository variable.

Deploy the selected commit with
[`deploy-gcp.yml`](../.github/workflows/deploy-gcp.yml), selecting **`runtime=rust`**.
Rails remains the default runtime. The workflow resolves `rust-git-SHA` to a digest and uses the
common backup, freeze, rehearsal and cutover path in
[`deploy/gcp/campfire-release.sh`](../deploy/gcp/campfire-release.sh). Production Rust deployment
also checks for a successful Rust CI run for that commit. An image build or workflow definition
does not establish that a candidate has completed a production-copy rehearsal.
The upstream GHCR `latest`/version/`main` images and arm64 recipe are historical upstream artifacts.
See [`deploy/README.md`](../deploy/README.md) for the release procedure.

The image runs one `campfire` executable with libvips and ffmpeg, replacing Ruby/Puma,
Redis/Resque and Thruster. It retains `/rails/storage` and the ONCE backup hooks. `TLS_DOMAIN`
configures front-server TLS/ACME; `DISABLE_SSL` supports a proxy deployment. `TARGET_PORT`
defaults to 3000 and `TARGET_BIND` to `127.0.0.1`; callers outside the container should use the
front listener unless explicitly configured otherwise
([front configuration](crates/kit/src/front/config.rs#L75)). Web Push uses `VAPID_PUBLIC_KEY` and
`VAPID_PRIVATE_KEY`, with the fixed subject described above. Other settings are in
[`crates/campfire/src/config.rs`](crates/campfire/src/config.rs).

For a local source image, from `rust/`:

```sh
docker build -t smartfire-rust .
cargo run -p campfire -- server  # needs SECRET_KEY_BASE or SECRET_KEY_BASE_DUMMY=1
```

## Known differences

These are current code boundaries and observable differences from Rails, not a blanket cutover
approval. Each entry states its reason and points to implementation or regression evidence;
older approvals remain scoped to their recorded inputs.

| Difference | Reason and evidence |
|---|---|
| Relative-duration overflow at unit-dependent ranges | Shared arithmetic is fixed-width I512 rather than Ruby's unbounded integers: power-of-ten inputs first overflow at 10^147 minutes, 10^145 hours, or 10^143 days/weeks (the preceding power still fits, including 10^144 hours); the 12 recorded hour/day inputs produce 24 mismatched outcomes. [Timestamp limit:23](crates/db/src/time.rs#L23), [parser:235](crates/db/src/slash_commands/time_parser.rs#L235), [boundary vectors](vectors/messaging/extreme_range.json), [diagnostic test:84](crates/campfire/src/controllers/message_features/extreme_range_tests.rs#L84), [strict probe](reference-tools/messaging/probe_extreme_range.py). |
| Security response headers on HTTP/1.1 Active Storage proxies | Rust retains six security defaults that Rails' Live responses omit, including successful streams and handled 404/416 responses. [Defaults:26](crates/campfire/src/security.rs#L26), [all-header regression:118](crates/campfire/src/controllers/agent_review_r5_tests.rs#L118), [scoped header list](plans/ws11api-approved-differences.md#http11-proxy-security-headers-pr-203-follow-up). |
| Huge-integer password-confirmation replay → 302 | Rust discards optional replay parameters when the encrypted session would overflow, avoiding Rails' 500 for 1,001/1,002-digit JSON integers; this evidence concerns replay parameters, not arbitrary path IDs. [Sudo guard:57](crates/campfire/src/concerns/sudo.rs#L57), [HTTP regression:233](crates/campfire/src/controllers/accounts/bots/casting_followups_tests.rs#L233). |
| Committed-file retention | Successful COMMIT transfers file ownership before fallible callbacks, so an earlier callback error cannot leave committed rows without their staged files as Rails can. [Callback/file regression:47](crates/campfire/src/controllers/agent_review_r4_tests.rs#L47), [retention contract](plans/ws11api-approved-differences.md#committed-file-ownership-and-missing-file-serving-pr-192-fourth-and-fifth-reviews). |
| Malformed SLA arrays → 400 | Rust rejects retained hashes/nested arrays before mutation instead of Rails' 500, preserving rules and audits; accepted empty/scalar arrays keep Rails behavior. [Validation:15](crates/campfire/src/controllers/rooms/board_automations/sla.rs#L15), [complete-response regression:142](crates/campfire/src/controllers/rooms/board_automation_tests.rs#L142). |
| Stricter race rechecks | Rust checks current membership/grants/suspension inside writes and seals private replies against current ownership, avoiding stale Rails authorization or private fields. [Race contract and original evidence](plans/ws11api-approved-differences.md#current-authorization-and-owner-transfer-races-205-review), [work authorization:190](crates/db/src/models/agent_work.rs#L190), [write-boundary regressions](crates/campfire/src/controllers/agent_work_writes_tests.rs). |
| Atomic durable enqueue vs Rails adapter soft-refusal | A failed SQLite job insert rolls back the originating write; Rails can commit a room tombstone despite adapter refusal and recover it later. [Job persistence:1](crates/campfire/src/jobs.rs#L1), [queue-failure/recovery regression:8](crates/campfire/src/controllers/rooms/queue_recovery_tests.rs#L8), [message rollback:570](crates/campfire/src/controllers/messages/http_tests.rs#L570). |
| No AES-CBC legacy-cookie fallback | The encryptor and cookie reader implement AES-256-GCM only; GCM vectors do not establish continuity for any deployed CBC cookies. [Encryptor:11](crates/rails_compat/src/message_encryptor.rs#L11), [cookie reader:154](crates/rails_compat/src/cookies.rs#L154). |
| Active Storage blobs embedded in rich-text bodies render ☒ | The attachable resolver has no blob renderer, so these SGIDs fall through to the missing-attachment marker; top-level message files/previews are separate. [Attachable types:109](crates/richtext/src/attachables.rs#L109), [resolution:208](crates/richtext/src/attachables.rs#L208), [render fallback:368](crates/richtext/src/attachables.rs#L368). |
| Session-cookie write frequency | Rust sends an encrypted session cookie on changes rather than Rails' refresh on each loaded/carried session, reducing cookie traffic while retaining its contents. [Session contract:6](crates/kit/src/session.rs#L6), [HTTP regression:636](crates/kit/tests/http.rs#L636). |
| Authentication-cookie refresh frequency | Rust re-signs `session_token` when session activity is due for its hourly refresh; Rails re-signs it on every authenticated application request. [Refresh:708](crates/campfire/src/concerns.rs#L708), [HTTP regression:176](crates/campfire/src/app/tests.rs#L176), [Rails authentication](../app/controllers/concerns/authentication.rb). |
| `last_room` cookie write frequency | Rust writes the permanent cookie only when the room changes, avoiding Rails' write and expiry renewal on every room page. [Cookie write:921](crates/campfire/src/concerns.rs#L921), [HTTP regression:477](crates/campfire/src/controllers/rooms/tests.rs#L477), [Rails room visit](../app/controllers/concerns/tracked_room_visit.rb#L8). |
| ETag construction for cached pages | Rust hashes page-part digests to reuse cached compression instead of hashing the entire body; validators still change with content. [ETag construction:137](crates/kit/src/deflater/splice.rs#L137), [content-change test:517](crates/kit/src/deflater/splice.rs#L517). |
| Compressed Action Cable frames | Rust negotiates `permessage-deflate` without context takeover to share compressed broadcasts; Rails' Cable does not negotiate it, while decoded messages retain the protocol. [Socket handshake:125](crates/cable/src/socket.rs#L125), [handshake regression:549](crates/cable/src/socket.rs#L549). |
| Rich-text hardening limits | Attribute angle brackets are escaped before autolinking, `name` is anchor-only, and content attachments stop after eight levels to bound unsafe/expensive input. [Sanitizer:244](crates/richtext/src/sanitizer.rs#L244), [hardening regressions:55](crates/richtext/tests/hardening.rs#L55). |
| Resource and listener bounds | Rust caps buffered JSON/form parameter bodies at 16 MiB before parsing and Cable subscriptions/identifiers at 64/4 KiB; its app listener defaults to loopback. These bound resource use and the forwarded-header trust boundary. [Body limits:19](crates/kit/src/body.rs#L19), [Cable limits:92](crates/cable/src/connection.rs#L92), [front defaults:75](crates/kit/src/front/config.rs#L75). |
| Media subprocess deadlines | Rust kills and reaps ffprobe after 30 seconds and ffmpeg preview extraction after 60 seconds; Rails' analyzer/previewer subprocesses have no such deadlines. [ffprobe:15](crates/storage/src/analyze.rs#L15), [ffmpeg:20](crates/storage/src/process.rs#L20), [kill/reap regression:168](crates/storage/src/process.rs#L168). |
| Composer unfurl budget | Rust adds a 10-second overall budget including slot waits, DNS, redirects and image validation, with at most 16 unfurls in flight; Rails' composer has no overall budget. Both use 5-second open/read/write operation timeouts. [Unfurl:27](crates/campfire/src/integrations/opengraph.rs#L27), [deadline regression:190](crates/campfire/src/integrations/opengraph/tests.rs#L190), [Rails fetch:9](../app/models/opengraph/fetch.rb#L9). |
| Push HTTP-exchange deadline | Rust uses 10-second open/read and 60-second write timeouts, plus a 30-second HTTP-exchange deadline after DNS resolution; Rails uses 60-second operation timeouts without that overall exchange deadline. These bound time spent awaiting a push service, not DNS. [Timeouts:28](crates/campfire/src/integrations/web_push.rs#L28), [DNS before delivery:75](crates/campfire/src/integrations/web_push.rs#L75), [exchange deadline:182](crates/campfire/src/integrations/web_push.rs#L182). |

## Imported upstream history

The following performance measurements and build account describe the upstream stock ONCE
Campfire port before Smartfire-specific work. They are retained as history, not current Smartfire
benchmarks, parity receipts or operating instructions. In particular, the historical token-free
CSRF and additive-index optimizations below were superseded by the current contracts above.
Upstream originally used a `reference/` submodule at stock Campfire `90b33002`.

### Historical upstream performance

These numbers come from benchmarking the [`v0.1.1`](https://github.com/basecamp/once-campfire-rust/releases/tag/v0.1.1)
image against stock ONCE Campfire: production images of both, the same seed data, the same 4 pinned
hardware threads, host networking, and 3 interleaved runs per app. The medians are below; the full
tables with spreads are in
[`bench/results/v0.1.1-20260928/report.md`](bench/results/v0.1.1-20260928/report.md). The host ran
other light work on other cores during the run; the spread between runs stays within a few percent
for the Rust app.

#### Throughput (16 concurrent clients)

| Route | Rails | Rust | Rust advantage |
|---|---|---|---|
| Room page | 215 req/s | 20,479 req/s | **95×** |
| Messages page (`?before=`) | 406 req/s | 23,365 req/s | **58×** |
| Sidebar | 528 req/s | 12,642 req/s | **24×** |
| Search | 385 req/s | 23,399 req/s | **61×** |
| Post a message | 274 req/s | 5,452 req/s | **20×** |
| `/up` | 4,069 req/s | 136,228 req/s | **33×** |

#### Latency

| Measurement | Rails | Rust | Rust advantage |
|---|---|---|---|
| Room page p50, one client | 10.3 ms | 0.19 ms | **54×** |
| Room page p99, 64 clients | 515 ms | 5.1 ms | **101×** |
| Post a message p99, one client | 13.4 ms | 1.67 ms | **8×** |
| Post a message p99, 64 clients | 349 ms | 16.7 ms | **21×** |
| Upload a 505 KB JPEG until its thumbnail is served | 132 ms | 29.4 ms | **4.5×** |

#### Real time (Action Cable, up to 10,000 clients in one room)

| Measurement | Rails | Rust | Rust advantage |
|---|---|---|---|
| Deliveries per second, 100 clients | 7,892 | 312,272 | **40×** |
| Deliveries per second, 1,000 clients | 10,771 | 503,302 | **47×** |
| Deliveries per second, 5,000 clients | 11,930 | 595,886 | **50×** |
| Deliveries per second, 10,000 clients | 9,585 | 638,688 | **67×** |
| Post to all 1,000 clients received, p50 | 107 ms | 6.5 ms | **16×** |
| Post to all 10,000 clients received, p50 | 1,171 ms | 40 ms | **29×** |
| Post to all 10,000 clients received, p99 | 1,519 ms | 61 ms | **25×** |
| Connect and subscribe 10,000 clients | 29.1 s | 2.4 s | **12×** |

Every client subscribed in every run, for both apps.

#### Startup and memory

| Measurement | Rails | Rust | Rust advantage |
|---|---|---|---|
| Cold start (`docker run` until `/up` answers) | 2,567 ms | 149 ms | **17×** |
| Idle memory (container) | 309 MB | 15 MB | **21×** |
| App process, 1,000 idle cable clients (Pss) | 645 MB | 169 MB | **3.8×** |
| App process, 10,000 idle cable clients (Pss) | 1,507 MB | 313 MB | **4.8×** |
| App process, 10,000 cable clients under load (Pss) | 2,191 MB | 310 MB | **7.1×** |
| Whole container, 10,000 cable clients under load (Pss) | 3,519 MB | 310 MB | **11×** |
| Image size, unpacked | 933 MB | 169 MB | **5.5×** |
| Image size, compressed download | 359 MB | 67 MB | **5.4×** |

Rails' whole container adds Redis and Thruster to its app processes; the Rust app is one process.

#### Since the previous benchmark

The run before this one benchmarked `main` at `898653e` the same way
([`bench/results/scale-20260927`](bench/results/scale-20260927/report.md)). Since then came
[cached page parts](#gzip-and-etags-from-cached-page-parts), [the new WebSocket
layer](#100000-clients-and-a-raspberry-pi-5), and opting out of transparent huge pages
([`bench/results/thp-20260928`](bench/results/thp-20260928/report.md)):

| Rust app | `898653e` | `v0.1.1` | Change |
|---|---|---|---|
| Room page, 16 clients | 6,002 req/s | 20,479 req/s | **3.4×** |
| Search, 16 clients | 8,970 req/s | 23,399 req/s | **2.6×** |
| Deliveries per second, 10,000 clients | 379,608 | 638,688 | **1.7×** |
| Post to all 10,000 clients received, p50 | 42 ms | 40 ms | 1.05× |
| Idle memory (container) | 47 MB | 15 MB | **3.1× less** |
| App process, 10,000 idle cable clients (Pss) | 582 MB | 313 MB | **1.9× less** |
| App process, 10,000 cable clients under load (Pss) | 876 MB | 310 MB | **2.8× less** |

#### 100,000 clients, and a Raspberry Pi 5

One Campfire holds 100,000 connected clients in 1.5 GB: every one connects in about 13 s, and
memory stays flat while messages fan out to all of them. On a Raspberry Pi 5's CPU budget
(emulated: four pinned cores capped at 1.2 cores' worth), the app delivered over a million
messages a second to those clients, the load generator's limit, using 0.71 of its 1.2 cores. What
limits a Pi is its gigabit Ethernet: about 51,000 compressed deliveries a second. That's 100,000
chatters in rooms of 100, each posting every five minutes, with a third to spare; a single room of
100,000 can't be busy on one gigabit link. Details and caveats in
[`bench/results/pi-100k-20260928/report.md`](bench/results/pi-100k-20260928/report.md).

| At 100,000 clients | Before | After |
|---|---|---|
| Memory, idle | 3.2 GB | 1.5 GB |
| Memory, while fanning out | 5.9 GB | 1.6 GB |
| A message on the wire | ~10 KB | ~2.3 KB (compressed) |
| Posting while a message fans out to everyone, p50 | 637 ms | 43 ms |

What changed: Action Cable sockets use their own small WebSocket implementation, which writes each
broadcast's shared bytes to every socket without copying them per connection, and compresses a
broadcast once for all of its subscribers (`permessage-deflate`, which browsers offer). Connections
run on threads of their own, so page loads and posts don't queue behind a fan-out. The app also
raises its own open-file limit, which in Docker would otherwise stop it at 65,536 clients.

#### Where the speed came from

A straight translation was already 3–10× faster than Rails. Profiling (in
[`plans/perf-attribution.md`](plans/perf-attribution.md)) then showed where the time went, and each
change since has been measured before and after, keeping the test suite and the parity gate green.
In the order they landed:

| Change | Effect |
|---|---|
| Look up a message's cached fragment before building its view, as Rails' `cache` does | Room page 989 → 1,664 req/s; messages page 1,116 → 2,234 req/s |
| gzip on the zlib-rs backend instead of miniz_oxide | Room page 662 → 955 req/s |
| Run SQLite WAL checkpoints on their own thread, off the writer | POST p99 at one client: 12.5 → 1.7 ms |
| Cache prepared statements for every query | POSTs about 10% faster |
| Share cached fragments instead of copying them; build stylesheet tags once per process | 6–20% less CPU per page |
| Cable: 4 KiB read buffers instead of zero-filling 128 KiB per read; encode each broadcast once and share it across subscribers; batch socket writes | 4.7× less CPU per delivery; half the latency and memory under fan-out |
| Fat LTO, one codegen unit, jemalloc | A further 5–14% per route |
| Splice precompressed messages into gzipped pages ([below](#gzip-and-etags-from-cached-page-parts)) | Room page 2,527 → 5,461 req/s; messages page 3,709 → 16,523 req/s; search 2,123 → 5,526 req/s |
| Forgery protection by `Sec-Fetch-Site` instead of CSRF tokens (upstream token-free configuration) | Room page +9%, messages page +6%, search +10%; pages render the same until their content changes, so revalidation gets a 304 |
| Index messages by `(room_id, created_at)`; check "more than a page" without counting the room | In a room with 236k messages: room page 95 → 6,051 req/s (64×), messages page 87 → 17,972 req/s (208×). Before, a room page sorted the room's whole history, so rooms slowed as they grew; now a long room serves as fast as a new one |
| Cache every part of a page, not just its messages, and take the ETag from the parts ([below](#gzip-and-etags-from-cached-page-parts)); send cookies only when they change | Room page 2.9×, search 2.8×, messages page 1.3× |
| Cable: own WebSocket framing with shared, once-compressed frames; connections on their own runtime ([above](#100000-clients-and-a-raspberry-pi-5)) | 100,000 clients in 1.6 GB instead of 5.9 GB while fanning out; a post during a 100,000-client fan-out 637 → 43 ms; frames 10 KB → 2.3 KB on the wire |
| Keep the compressed form of every page, not only pages with cached messages ([`bench/results/whole-page-parts-20260929`](bench/results/whole-page-parts-20260929/summary.md)) | Sidebar 10,683 → 19,400 req/s (1.8×); it spent 41% of its CPU compressing the same page again |
| No transparent huge pages for the process or jemalloc ([`bench/results/thp-20260928`](bench/results/thp-20260928/report.md)) | Idle memory 37 → 11 MB on two cores and 160 → 15 MB on 32, where the kernel's THP setting is `always`; throughput unchanged |

Against Rails, the room page went from 4.4× in the preliminary benchmark to 95× in the latest one.

#### gzip and ETags from cached page parts

Every response is gzipped at level 6, as Rails' `Rack::Deflater` does, and after the passes above
that was 60–76% of the CPU on large pages. Most of a room page is cached messages, whose bytes are
the same on every request, so the app stopped compressing them per request, in two steps:

1. **Spliced gzip.** Each cached message is compressed once and kept, and pages splice the stored
   pieces into the gzip stream. Compressing each message on its own would make a room page 4.4×
   larger, because consecutive messages share most of their markup, so each piece is compressed
   against the message before it as a preset dictionary, and reused only when that same message
   (with the same text between them) comes before it again: the steady state for a room page. The
   layout around the messages was still compressed live, because every page carried a fresh CSRF
   token.
2. **Cached page parts.** Without CSRF tokens (in the upstream token-free configuration), a page
   renders byte for byte the same until what it shows changes, so the layout can be stored too. A
   page is now split into parts that cover it end to end: its cached messages and the text between
   them. Each part is compressed once, against the part before it, and kept under the part's
   identity (the cached fragment, or the SHA-256 of the text) and its predecessor's; a message keeps
   pieces for the few predecessors it's seen with (its room, a page of older messages, search
   results). The ETag comes from the parts' digests instead of a SHA-256 over the whole body.

For a 466 KB room page, gzip and the ETag took ~1,200 µs per request at first, ~460 µs after
splicing, and 42 µs now; the first request after a page changes pays ~2 ms, once, to compress its
new parts. The decoded body is unchanged, and the compressed page is within 1% of compressing it
whole.

| Route (16 clients) | Before | Spliced gzip | Cached page parts |
|---|---|---|---|
| Room page | 2,527 req/s | 5,461 req/s | 16,881 req/s |
| Messages page (`?before=`) | 3,709 req/s | 16,523 req/s | 22,580 req/s |
| Search | 2,123 req/s | 5,526 req/s | 16,097 req/s |

Each step was measured natively against the commit before it, in its own session, so the columns
come from different runs (the page-parts run on a busy host, which understates it). Details in
[`bench/results/splice-20260927`](bench/results/splice-20260927/report.md),
[`bench/results/header-csrf-20260927`](bench/results/header-csrf-20260927/report.md) and
[`bench/results/page-parts-20260927`](bench/results/page-parts-20260927/report.md).

### How the upstream port was built

The port was built in about a day by coordinated Claude Code agents, each owning one crate or
harness component. They followed the plan in `plans/rust-conversion.md`, which Codex also reviewed.
[`plans/overnight-report.md`](plans/overnight-report.md) logs the unattended overnight run: the
parity gate going green, the Thruster replacement, the benchmarks, and each optimization with its
before and after numbers. The optimizations and divergences since then were made the same way, one
pull request each, with their measurements in `bench/results/`.

## License

MIT. See [`MIT-LICENSE`](MIT-LICENSE).
