# WS8bm2 report — partial message-features delivery

**PARTIAL.** Nine of the ten previously deferred controller behaviors are complete: both whole-room poll cases and all seven quote rendering/cache/refresh cases. Root composite cache keys, message-page validators, private GitHub/Fizzy frames, pin panels and Saved/Scheduled sidebar destinations are integrated. Builder dates now use a separate calendar parser, checked against 69 actual Rails cases. Agent invocation is the one remaining named controller behavior, explicitly deferred to WS11.

The full populated provider composition, owning Markdown composer/schedule control, broader Ruby date and parameter coercions, and the remaining verification gaps listed below are partial. Physical push sending (WS17), agent integration (WS11) and huddle integration (WS13) remain clearly flagged seams. All 45 browser cases are reserved for the end-to-end system-test phase against the Rust server, as instructed; no browser harness was built. This branch is not cutover-ready.

Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2`; branch `rust/ws8bm2-message-features`. Rails pin: `d7c7de9264c63015be398001d7a1094e7695a6db`. WS9 main `4278cb1e7a4529d5e2ecee51fccf69be6ed45257` was merged previously using merge commit `4acb20fc454fee0a7de6a6329768472c9eee7e44`; it remains an ancestor. No PR or deployment.

## Pushed slices

Previously received: polls/pins (`a036f6e6`), saved/reminders (`90bb304a`), scheduled (`f39afa8b`), search (`53079ef4`), current-renderer search preloads (`f168c348`), slash/autocomplete/play (`4be208b3`), quote endpoints/Files (`40b6a541`), reminder policy/registered job/tagged-send seam (`166ae831`), and independent-checkout corrections (`b67efa3c`).

This continuation pushed each coherent slice:

| Slice | SHA |
| --- | --- |
| Nine deferred controller behaviors; preloaded quote composition and real refresh/removal streams | `d24317e8004e503fc08e3f3b332119a78df20333` |
| Root composite cache facts, private provider frames and Rails message-page validators | `5416c37b42197ae510d3000953af420e74736546` |
| Scoped pin panels and Saved/Scheduled sidebar destinations; named WS11 auth seam | `1fdf42a69462ecdb979035c1125ae87c9c5ff31d` |
| Calendar builder parsing; 69 Rails date probes; independent twelve-oracle replay tool | `f1c6315b3eb864b2d85f37a62208ca8d452c5b4f` |
| Correct two inherited root test assumptions for the new Rails behavior | `91eb3737f19f4553057e819322be4f92732e708f` |

**Verified source SHA: `91eb3737f19f4553057e819322be4f92732e708f`.** The following report-only commit has identical source; its pushed SHA is in the final reply.

## Changes by file

Paths below are relative to the assigned worktree. This table records this continuation; previously received HTTP features remain present.

| Files | Changes |
| --- | --- |
| `rust/crates/db/src/models/message_rendering.rs` | Batched quote sources, bodies, authors, rooms and attachment facts; related provider/reference/thread/pin/poll/step timestamps; nullable room-name facts and quoted-source name digest. Neutral facts remain separate from rendering. |
| `rust/crates/views/src/messages.rs`, `message_links.rs`, `templates/messages/_message.html` | Typed preloaded quote references and context-only render helper; one flagged root helper call mounts the container. Same-room cards render inline; cross-room cards expose only the authorized lazy frame. Existing replacement container order is retained. |
| `rust/crates/campfire/src/controllers/presenters.rs` | Small flagged owner seam: builds quote/provider composition and Rails expanded cache keys from preloads, in the request zone. No queries during preloaded quote rendering. |
| `rust/crates/views/src/fragment_cache/keys.rs` | Ruby-compatible nullable direct-room name digest and expanded fragment-key facts. |
| `rust/crates/campfire/src/controllers/messages.rs` | Uses composite keys for fragment reads/writes and cached-response fragments; uses Rails body-free related stamp, sorted pin-set digest and presentation version 3 for message-page ETags, without Last-Modified. Human edits replace their quote container; human deletion holds request origin through callbacks. |
| `rust/crates/campfire/src/channels/message_features.rs` | Real QuoteCards refresh/removal adapter and viewer-neutral card replacement streams, including request-free durable refresh jobs. |
| `rust/crates/campfire/src/controllers/message_features/quote_integration_tests.rs` | Nine real controller/room behaviors: anonymous/regular voter markers, inline/lazy quotes, neutral cached DM labels, zero-query preloaded quotes, registered source refresh, deletion/cache invalidation and editing to add a permalink. Real sockets verify delivery. |
| `rust/crates/campfire/src/controllers/message_features/root_cache_tests.rs` | Actual Rails key transitions and private provider frames, nil-name digest, warm source/name/poll/unpin cache changes, conditional GET and provider-secret absence. |
| `rust/crates/campfire/src/controllers/rooms.rs`, `rust/crates/views/src/{pins,rooms}.rs`, `templates/rooms/pins/_panel.html`, `templates/rooms/show/_nav.html` | Small shell seam supplies an authorized count and room STI param key; mounts pin header, dialog and lazy frame with closed/direct/voice/stage/board identities. |
| `rust/crates/views/templates/users/sidebars/{_message_tools,show}.html` | Small shell seam mounts actual Saved (`/saved`) and Scheduled (`/scheduled_messages`) links. |
| `rust/crates/campfire/src/controllers/message_features/panel_tests.rs`, `rust/crates/views/tests/core.rs` | Four exact Rails pin panel variants, two sidebar links, room access gates and STI target identity. Existing view fixtures accept the optional pin facts. |
| `rust/crates/campfire/src/concerns.rs` | Explicit `WS11_AGENT_AUTHENTICATION_SEAM` documentation. No agent authentication implementation was added. |
| `rust/crates/db/src/slash_commands/time_parser.rs`, `rust/crates/campfire/src/controllers/message_features.rs` | Separate calendar parsing for builder dates, preserving future-oriented slash parsing. Adds named months, compact date/datetime, civil overflow, offset/fraction/weekday handling and invalid-calendar versus nil distinctions. WS9 strict zone behavior is retained. |
| `rust/crates/campfire/src/controllers/message_features/date_tests.rs` | 69 Rails probes across UTC/New York/Apia and reminder/scheduled HTTP paths. |
| `rust/crates/campfire/src/controllers/messages/tests.rs`, `rust/crates/campfire/src/channels/tests/hub_test.rs` | Replace obsolete Last-Modified assertion; verify and consume the newly emitted empty QuoteCards frame before the next boost. No timing threshold or concurrency change. |
| `rust/reference-tools/messaging/{quote_integration,root_cache,panels,date_inputs}.rb`, corresponding `rust/vectors/messaging/*.json` | Four new pinned Rails oracles with committed fixture rows and exact output; reference time-zone context is applied before loading timestamped rows. |
| `rust/reference-tools/messaging/{features-reference-check,features-discriminate,verify_oracles}.py` | Verify 85 consumed reference files; eight new compiled discriminating mutations; independently replay all twelve committed oracle fixtures from Rails. |
| `rust/plans/ws8bm2-report.md` | Tracked copy of this external report. |

No schema, dependency, Cargo.lock, Rails source, parity mask or allowlist changes. The tiny root/helper/cache and shell seams above were made for the lead's explicit root/cache/panel continuation; WS8b-m and WS8b-r retain their broader root and shell ownership. Post-pin profile/status popup work remains WS8b-r2's area.

## Design and parity evidence

**Quotes and the nine controller cases.** Authorization checks the quoting room/reference before source facts. Same-room cards use batched source facts; cross-room cards retain lazy authorized loading. Private/deleted sources stay private and direct-room labels remain viewer-neutral. Four quote containers match actual Rails bytes. One versus three distinct direct-room quotes render with zero SQL queries after preload; different room binds prevent hiding lazy reads behind statement caching. Human source edits run the registered QuoteCards job and deliver replacements over a real WebSocket; deletion removes references, touches the quoting message and clears cards; editing a plain message to add a permalink replaces its own container. Both anonymous and regular polls pass through real room HTTP/cache rendering.

**Composite cache and providers.** Rails composite facts are loaded in batches: GitHub/Fizzy/Twitter/Event stamps, embeds/reference sets, PR discussion-thread stamps, pins, polls, agent steps, quoted-source max(updated_at, edited_at) and author/room-name digest. The same expanded key is used for fragment reads and writes, includes origin, and uses the request zone. Nullable DM names hash with Ruby Array.inspect's `nil`. Six transition keys in UTC/Hawaii and twelve private/unknown GitHub/Fizzy frame containers match Rails. Warm legacy source edits, renames, poll-only changes and unpins change fragments; related stamps and the sorted pin set change message-page ETags. Last-Modified is absent, matching Rails. Full populated public provider cards and their fetch/refresh/edit integrations remain incomplete; cache facts alone do not establish full populated root/search-page parity.

**Panels.** The actual room header/dialog/lazy pin frame and actual Saved/Scheduled sidebar links are mounted. Four pin panels and two sidebar link fragments match Rails exactly; inaccessible rooms load no pin rows or panels. Voice/stage/board STI target names remain correct. Schedule-send is intentionally still unmounted: the inherited Rust shell renders Lexxy while the pinned schedule/composer JavaScript requires a Markdown textarea. WS8b-r owns that composer conversion and its action menus; the already matching standalone schedule control can be mounted there. Browser behavior remains unproved.

**Dates.** Builder parsing no longer shares slash's future-oriented relative grammar. Calendar probes distinguish nil/unparseable input from an invalid numeric calendar or clock exception and cover named months, 8/14-digit dates/datetimes, fractions/offsets, February overflow, 24:00, leap seconds, weekdays and DST in three zones. Reminder and scheduled HTTP cases exercise the new path. The oracle loads Rails models inside Time.use_zone so its timestamp cache facts reflect the real request zone; no output normalization or masking was added. Full Ruby Date._parse grammar and odd parameter coercions remain partial.

Previously received behavior remains: transactional saved reminder claim/rearm and job rollback, scheduled lock-time rechecks and send rollback, search operator chips/tuple paging/access-scoped sections/DST and full current-renderer preloads, slash/autocomplete JSON and all 61 /play fragments, quote endpoint privacy, and cumulative upload/Drive Files with constant-query HTTP checks. Search still needs populated provider composition before whole-page/older-window parity can be claimed.

## Explicit seams retained

- **WS17 — physical tagged push sending.** `rust/crates/campfire/src/jobs/reminders.rs` retains the named tagged-send boundary. Policy and the real registered durable handler are complete; 27 policy cases and two real Rails payload/subscription handoffs match. The full tag and every subscription reach the boundary. Allowed pushes with subscriptions stay a single ready durable job rescheduled by sixty seconds until WS17 installs tagged pool enqueue; suppressed/empty-subscription jobs complete. No false delivery acknowledgment or physical send is claimed.
- **WS11 — agent integration.** Registered command metadata is real, but agent invocation and credential/domain integration remain deferred. `WS11_AGENT_AUTHENTICATION_SEAM` names the shared credential lookup boundary: currently every agent secret is unknown (401); the valid-token endpoint 403 behavior needs WS11's credential model. No agent implementation was added here.
- **WS13 — huddle integration.** `rust/crates/campfire/src/huddle_readiness.rs` is explicitly readiness-only. All twelve pinned readiness cases pass; gateway/token/grant/launch integration belongs to WS13. No huddle integration was added here.

## Fresh-clone verification

Independently cloned the pushed branch from the remote, rebuilt both parity seeds from committed recipes in that clone, then fast-forwarded the clone to verified source `91eb3737`. No target, seed or local fixture was copied from the implementation worktree. Generated logs/temp files live under that clone's `.scratch`; runtime fixture inputs come from committed files and its independently built seeds. Cargo used the clone's own target, Rust 1.98.1, four build jobs and four test threads. CARGO_INCREMENTAL=0 only reduced build disk usage. No concurrency or timing threshold was lowered/widened.

All commands below were executed during this continuation. Setup ran from the assigned worktree root:

```sh
git clone --single-branch --branch rust/ws8bm2-message-features https://github.com/Smart-Data-Ohio/smartfire.git .scratch/continuation-fresh/repo
```

From `.scratch/continuation-fresh/repo/rust` (TMPDIR already exists):

```sh
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 parity/bin/seed build default first_run > ../.scratch/fresh-seed.log 2>&1
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 > ../.scratch/final-metadata.json
TMPDIR="$PWD/../.scratch/tmp" CARGO_TARGET_DIR="$PWD/target" CARGO_INCREMENTAL=0 CI=1 CABLE_TEST_PORT_RANGE=52500-52549 MAIL_TEST_PORT_RANGE=52550-52599 mise exec rust@1.98.1 -- cargo test --locked -j4 --workspace --exclude html5ever -- --test-threads=4 --nocapture > ../.scratch/final-workspace-test.log 2>&1
TMPDIR="$PWD/../.scratch/tmp" CARGO_TARGET_DIR="$PWD/target" CARGO_INCREMENTAL=0 mise exec rust@1.98.1 -- cargo clippy --locked -j4 --workspace --exclude html5ever --all-targets -- -D warnings > ../.scratch/final-clippy.log 2>&1
```

Clone, seed build, locked metadata, final suite and clippy exited 0. Locked metadata has JSON output, not a native summary line. Raw seed summaries:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

The first fresh suite failed two deterministic inherited root assertions: Last-Modified was required although Rails omits it, and the hub test assumed the next frame was a boost instead of first checking the new empty quote container. The final source slice corrects both and explicitly verifies the added frame. No timing flake occurred, and timeouts/concurrency were unchanged. Preserved raw first-run summary (`first-workspace-test.log`):

```text
test result: FAILED. 599 passed; 2 failed; 3 ignored; 0 measured; 0 filtered out; finished in 91.05s
```

The final fresh suite totals **1667 passed, 0 failed, 11 existing ignored**, across the following 46 raw summaries. It includes **137 owned message-feature tests and 45 owned search-port tests (182 total)**, all passing without owned skips or ignores:

```text
test result: ok. 601 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 136.35s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.35s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 54.52s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.84s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.37s
test result: ok. 446 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 141.86s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.49s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.46s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.37s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 6.28s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.33s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.40s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 50.31s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.41s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.53s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.90s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.44s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Raw clippy summary:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 15s
```

Existing suite boundaries: the eleven ignores are two reference recorders, WS11 bot management, the push-latency measurement, four database scenario/export/Rails/rollback tests, one mail rollback export, and two kit doctests. Conditional ACME validation returned because PEBBLE_MINICA is unset. Version-specific generated media byte comparisons returned because the vectors use libvips 8.16.1/ffmpeg 7.1.5 and this host uses 8.18.6/n9.0.2. The `skipping locally: parity/.seed/default isn't built` message comes from the intentional empty-temporary-directory missing-seed guard test, not an owned seeded test. Both required seeds were present and independently validated.

From `.scratch/continuation-fresh/repo`:

```sh
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb default > .scratch/final-validate-default.log 2>&1
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb first_run > .scratch/final-validate-first-run.log 2>&1
python3 rust/reference-tools/messaging/features-reference-check.py > .scratch/final-reference-check.log 2>&1
python3 rust/reference-tools/messaging/verify_oracles.py > .scratch/final-oracle-replay.log 2>&1
```

All four exited 0. The 85-file check verifies consumed controller/model/helper/template source bytes and rejects two injected byte/file-set differences; it does not claim every image file was verified. The source check and oracle replay ran at `f1c6315b`; the following `91eb3737` changes only the two Rust assertions above. Raw validator closing lines, default then first_run:

```text
  "passed": 29,
  "failed": 0
}
  "passed": 4,
  "failed": 0
}
```

Raw reference and independent oracle replay summaries:

```text
WS8bm2 reference source check: 85 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
WS8bm2 Rails oracle: 8 poll reads/ballots; 6 poll creates; 4 pin writes; 11 partials; 10 zone/date probes
WS8bm2 oracle replay: features.json byte-identical
WS8bm2 saved Rails oracle: 11 HTTP responses; 6 item partials; 1 empty page
WS8bm2 oracle replay: saved.json byte-identical
WS8bm2 scheduled Rails oracle: 12 HTTP responses; 8 row partials; 1 empty page; 2 composer controls
WS8bm2 oracle replay: scheduled.json byte-identical
WS8bm2 search Rails oracle: 15 parsed queries; 15 chip partials; 1 empty page; 1 clear stream; 18 zone/date selections; 1 populated sections partial; 1 load-older control; 1 empty older stream; 4 HTTP responses; 771 Unicode word ranges
WS8bm2 oracle replay: search.json byte-identical
WS8bm2 preload Rails oracle: 6 complete message fragments; 9 committed fixture tables
WS8bm2 oracle replay: preloads.json byte-identical
WS8bm2 slash Rails oracle: 20 dispatch responses; 17 picker responses; 61 play presentation fragments; 3 format responses; 12 huddle readiness cases
WS8bm2 oracle replay: slash.json byte-identical
WS8bm2 links/files Rails oracle: 15 Files sections; 5 quote HTTP responses; 2 quote partials; 16 size values; 9 fixture tables
WS8bm2 oracle replay: links_files.json byte-identical
WS8bm2 reminder push Rails oracle: 27 policy cases; 2 captured real job payload/subscription handoffs; 3 fixture tables
WS8bm2 oracle replay: reminder_push.json byte-identical
WS8bm2 quote integration Rails oracle: 4 containers; 7 messages; 3 fixture tables
WS8bm2 oracle replay: quote_integration.json byte-identical
WS8bm2 root cache Rails oracle: 6 composite keys; 12 provider frame containers; 10 fixture tables
WS8bm2 oracle replay: root_cache.json byte-identical
WS8bm2 panels Rails oracle: 4 pin panels; 2 sidebar links
WS8bm2 oracle replay: panels.json byte-identical
WS8bm2 date Rails oracle: 69 Time.zone.parse cases in 3 zones
WS8bm2 oracle replay: date_inputs.json byte-identical
WS8bm2 oracle replay: 12/12 independently replayed fixtures byte-identical
```

## Compiled discrimination / failing first

New security-sensitive quote and panel behavior is checked with real compiled regressions, alongside cache and date discrimination. From the assigned worktree root:

```sh
python3 rust/reference-tools/messaging/features-discriminate.py quote- > .scratch/quote-mutations.log 2>&1
python3 rust/reference-tools/messaging/features-discriminate.py cache- > .scratch/cache-mutations.log 2>&1
python3 rust/reference-tools/messaging/features-discriminate.py panel- > .scratch/panel-mutations.log 2>&1
python3 rust/reference-tools/messaging/features-discriminate.py date- > .scratch/date-mutations.log 2>&1
```

Each command exited 0 after detecting the intended failing compiled tests and restoring the source. No string-search substitute or expected-output masking. These regressions remove inline quote privacy, introduce a viewer-bound DM label, remove reference scoping, reuse stale root fragment keys, omit pin-set validators, skip panel membership checks, lose STI identity, and route builder dates back through slash grammar. Raw summaries:

```text
quote-inline-privacy: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 591 filtered out; finished in 0.92s
quote-neutral-label: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 591 filtered out; finished in 1.08s
quote-privacy: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 591 filtered out; finished in 0.49s
WS8bm2 discrimination: 3 compiled regressions detected; sources restored
cache-root-fragment: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 596 filtered out; finished in 0.54s
cache-pin-validator: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 596 filtered out; finished in 0.45s
WS8bm2 discrimination: 2 compiled regressions detected; sources restored
panel-room-gate: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 603 filtered out; finished in 0.69s
panel-sti-targets: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 603 filtered out; finished in 0.48s
WS8bm2 discrimination: 2 compiled regressions detected; sources restored
date-builder-calendar: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 603 filtered out; finished in 0.04s
WS8bm2 discrimination: 1 compiled regressions detected; sources restored
```

Earlier deliveries' failing-first probes and mutations remain in branch history; their old commands are not claimed as rerun in this report. This continuation independently replays all twelve actual Rails oracle fixtures and discriminates the eight new regressions above.

## Grouped Rails behavior coverage

Counts below are named Rails reference behaviors mapped to passing grouped Rust tests and pinned oracles; **the Rails controller suites were not run as suites**. They do not imply complete provider, shell or browser parity.

| Rails controller file under `test/controllers/` | Named behaviors covered / total | Remaining integration |
| --- | --- | --- |
| `rooms/polls_controller_test.rb` | 16 / 16 | Browser phase; additional performance/runtime proof below |
| `messages/pins_controller_test.rb` | 6 / 6 | Action menus/browser phase |
| `rooms/pins_controller_test.rb` | 3 / 3 | Browser phase; additional performance proof below |
| `saved_items_controller_test.rb` | 12 / 12 | WS17 physical send seam; browser phase |
| `scheduled_messages_controller_test.rb` | 19 / 19 | WS8b-r Markdown composer/schedule control; browser phase |
| `searches_controller_test.rb` | 36 / 36 | Full populated provider composition; browser phase |
| `rooms/slash_commands_controller_test.rb` | 9 / 10 | WS11 agent invocation seam |
| `autocompletable/icons_controller_test.rb` | 6 / 6 | Browser phase |
| `autocompletable/slash_commands_controller_test.rb` | 7 / 7 | Browser phase; WS11 invocation seam |
| `autocompletable/users_controller_test.rb` | 5 / 5 | Broader odd-shape coercions |
| `rooms/message_links_controller_test.rb` | 12 / 12 | Browser phase |
| `rooms/files_controller_test.rb` | 8 / 8 | WS8b-r Files tab/shell; odd-shape coercions; browser phase |
| **Total named controller behavior ports** | **139 / 140** | **One WS11 agent seam** |

| Rails system file under `test/system/` | Browser cases proved / total |
| --- | --- |
| `polls_test.rb` | 0 / 4 |
| `pins_saved_test.rb` | 0 / 7 |
| `slash_commands_test.rb` | 0 / 26 |
| `search_files_test.rb` | 0 / 4 |
| `scheduled_messages_test.rb` | 0 / 4 |
| **Total** | **0 / 45; reserved for end-to-end phase, none attempted** |

## Rails test mapping and ownership

Covered controller behaviors are ported as grouped Rust tests and pinned HTTP/partial oracles; the Rails test suites themselves were not run here.

- `test/controllers/rooms/polls_controller_test.rb`: all sixteen cases covered: creates a poll; multiple anonymous close time; too few options; board rejection; replacement/broadcast; retraction; closed rejection; foreign options; non-member create/vote; bot create; viewer ballot; anonymous viewer ballot; outside-room show; bot show. The final two room-page cases (anonymous cards carry no voter IDs; regular cards carry IDs) now pass through real room HTTP and the shared cached renderer.
- `test/controllers/messages/pins_controller_test.rb`: all six behaviors covered: creation plus four frames/note, idempotency, cap, unpin plus three frames, absent unpin and non-member rejection.
- `test/controllers/rooms/pins_controller_test.rb`: all three behaviors covered: newest-first jump/unpin list, empty list and non-member rejection.
- `test/system/polls_test.rb`: all four deferred to **WS8bm2**, coordinating composer/shell with WS8b-r and root cache with WS8b-m: builder/create/change/retract, anonymous reload markers, second-session live results, closed form absence.
- `test/system/pins_saved_test.rb`: all seven deferred to **WS8bm2**: menu/panel/live badge, same-tab note jump, compact note/no menu, panel unpin, save/remind/status/remove, tomorrow-9am fall-back and custom reminder. Pin header/dialog/lazy-frame mounting is now covered through HTTP; the remaining action menus and Markdown shell belong to WS8b-r. Socket/partial tests here cover part of those behaviors but are not browser ports.

The following file mapping distinguishes completed request ports from deferred controller/system work. The remaining controller case is the explicitly deferred WS11 agent seam. Browser cases remain inventoried for the final end-to-end phase against the Rust server; no browser harness was built here:

- **Covered: 12/12 named Rails behavior ports pass.** `test/controllers/saved_items_controller_test.rb` (12 cases; 12 named Rust request tests pass, plus four additional checks): index lists saved messages with status filters; index hides items whose room access was lost; create saves with a reminder; create without a reminder leaves remind_at blank; create again updates the reminder instead of duplicating; create again after a fired reminder re-arms it; create rejects past and unparseable reminders; create is 404 for a message the user cannot see; update marks done and reopens; update rejects an invalid status; update and destroy are 404 for hidden or foreign items; destroy removes the item.
- **Covered: 19/19 individually named Rust request ports pass.** `test/controllers/scheduled_messages_controller_test.rb` (19 tests): index lists upcoming and past rows; index hides other people's rows; index shows stranded rows so they can be cancelled; creates a scheduled message; create rejects past times; create is 404 outside membership; updates text and time; update during an active claim is refused; update during an active claim redirects with a notice in HTML; update after the claim goes stale is allowed; update is 404 for sent rows and other people's rows; destroy cancels pending rows only; a send that lands between the lookup and the lock refuses the cancel; a send that lands between the lookup and the lock refuses the edit; destroy during an active claim is refused; send_now posts immediately; send_now drops rows without access; send_now drops rows the model rejects with the reason; bots are forbidden.
- **36/36 named Rust behavior ports pass; the complete current-renderer preload path is implemented; populated provider whole-page parity remains partial; quote preloads and composition are now integrated.** `test/controllers/searches_controller_test.rb` (36 tests): index initial view; finding reachable messages; unreachable messages are not found; operator words are searched literally instead of raising; a leading operator returns 200; a boolean-looking query does not exclude terms; a quote character returns 200 with sensible results; create does not run the search; clear does not run the search; clear answers Turbo with streams that empty the header and page recents in place; clear leaves recents alone when it can't answer the requested format; the header renders at most ten recents even when older rows exceed the trim; clear without Turbo returns to the page it came from; index renders the page for a Turbo Stream request without an older-results cursor; the header search field renders on every signed-in page, empty outside search; the search page without a query lists recents instead of a watermark; a query with no results shows an empty state; results page through Load older results; an older window renders as a page without JavaScript; create saves the search term; create with no searchable words redirects back with a notice and records nothing; clear search history; from: narrows results to that author; in: narrows results to that room; in: a room the user is not in returns nothing; sections exclude soft-deleted rooms; has:pin narrows results to pinned messages; has:link narrows results to messages carrying a link; on: narrows results to that day; is:thread narrows results to thread messages; filter-only queries list without text and show chips; chips link back without their operator; boards, work threads, and events render as sections scoped to access; search sections cost the same queries regardless of section size; search sections label direct rooms neutrally; operator values cannot inject SQL or FTS syntax.
- **9/10 named behavior ports pass; agent execution is deferred to WS11 integration.** `test/controllers/rooms/slash_commands_controller_test.rb` (10 tests): shrug posts through the dispatcher; unknown commands answer an error without posting; status answers ephemeral confirmation; event answers an open_url; bare event opens the blank form; poll answers open_poll; agent commands invoke through the room; thread commands dispatch with the thread; non-members get 404; bots are forbidden.
- **6/6 named behavior ports pass.** `test/controllers/autocompletable/icons_controller_test.rb` (6 tests): requires authentication like the users endpoint; returns mixed brand and emoji matches with their payloads; orders prefix matches first and limits the results; returns no matches for blank or unknown queries; returns workspace icons with their stable image URL; lists every workspace icon for the picker Custom tab.
- **7/7 named behavior ports pass.** `test/controllers/autocompletable/slash_commands_controller_test.rb` (7 tests): lists built-ins with metadata; exposes takes_arguments for immediate and argument commands; includes the room's agent commands; agent commands registered without arguments run immediately; thread conversations hide root-only commands; filters by query; non-members get 404.
- **5/5 named behavior ports pass.** `test/controllers/autocompletable/users_controller_test.rb` (5 tests): search returns matching users; search results escape HTML in names; room search returns matching users; room search omits the Markdown token for duplicate display names; room search is scoped by membership.
- **12/12 named behavior ports pass, including seven new root/cache/refresh cases.** `test/controllers/rooms/message_links_controller_test.rb` (12 tests): a member of the source room sees the quote card; a quote of a soft-deleted source room shows only the private chip; a non-member of the source room sees only the private chip; a non-member of the quoting room gets nothing; a reference from another room gets nothing; a same-room quote renders inline in the room; a cross-room quote renders a lazy frame in the room; editing the source enqueues a job that refreshes quoting cards over the stream; deleting the source clears quoting cards and busts their cache; two viewers of a cached direct-room quote see the same neutral label; quote cards cost the same queries regardless of card count; editing a message to add a permalink replaces its own card container.
- **8/8 named behavior ports pass.** `test/controllers/rooms/files_controller_test.rb` (8 tests): lists uploads newest first with jump links; lists Drive attachments as generic picker-only rows; type filters narrow uploads; filename search matches substrings and escapes wildcards; uploads page cumulatively; only the room's own files are listed; non-members get nothing; rendering costs the same queries for 4 files as for 16.
- **0/26 browser ports; all deferred.** `test/system/slash_commands_test.rb` (26 tests): typing slash opens the command picker with combobox semantics; picking poll by Enter runs it immediately; picking poll by click runs it immediately; picking event by Enter opens the form immediately; picking huddle by Enter runs it immediately; suggestion rows hint argument placeholders only; the close button dismisses the picker without sending; the close button does not overlap the first row's text; Escape closes the picker without sending; mentions and emoji pickers have no close button and still commit; the picker lists registered agent commands; agent commands that take arguments insert and wait; agent commands without arguments run immediately when picked; shrug posts through the picker; arguments close the slash picker and Enter posts; Enter submits while the picker's deactivating update is still pending; a submit queued during the live command check still runs the command; unknown slash words post as normal messages; double slash escapes a known command; a command registered after page load still runs; me renders as an action line; poll opens the poll builder; event navigates to the prefilled form; huddle reports when unconfigured; agent commands respond ephemerally until the agent replies; status sets the custom status.
- **0/4 browser ports; all deferred.** `test/system/search_files_test.rb` (4 tests): filter chips show parsed operators and remove them; a pasted permalink renders a quote card with a working jump link; a cross-room permalink loads its quote frame for members and outsiders; the Files tab lists uploads and Drive rows with working filters.
- **0/4 browser ports; all deferred.** `test/system/scheduled_messages_test.rb` (4 tests): schedules from the composer and lists in the Scheduled view; schedule send requires a draft; edits, sends now, and cancels from the Scheduled view; the sidebar links the Scheduled view.

## Remaining work, in priority order

1. **Populated provider composition and callbacks:** root cache facts, quotes and private/unknown GitHub/Fizzy lazy frames are complete. Populated public GitHub/Fizzy, Twitter/X, events, LinkedIn and generic link-embed cards, their fetch/refresh callbacks/endpoints, and complete edit-replacement targets remain incomplete. Coordinate the small root seams with WS8b-m and provider owners WS15g/WS15e/WS14e. Then prove populated search page and older-window bytes with those cards. Other owners' deletion/unpin write paths still require an origin-scope audit; the human message deletion and owned pin paths here hold request origin.
2. **Shell/panels:** pin header/dialog/lazy frame and Saved/Scheduled links are complete. Markdown composer conversion, schedule-send mounting, action menus and Files tab/full shell integration remain WS8b-r. Do not attach the schedule control to the incompatible inherited Lexxy form.
3. **Date/parameter parity:** 69 calendar probes, existing date operators/DST and normal HTTP inputs pass. Broader Ruby Date._parse grammar, malformed-input distinctions outside the probes, option stripping and odd Array/Hash-to-string parameter shapes remain partial, including user/file-filter/pager/reminder inputs. Short/variable compact dates and other unprobed Ruby formats need actual Rails differentials before being claimed.
4. **Additional non-browser proof:** poll/pin-list constant-query measurements and a periodic poll-close job's real runtime socket delivery remain unverified. Existing vote/pin live socket checks and quote preloaded zero-query tests do not establish those separate checks.
5. **Explicitly deferred seams, not implementations for this worker:** physical tagged push send WS17, agent invocation/credential integration WS11 (the one remaining named controller case), and huddle launch WS13. Preserve the named seams and let those owners integrate them.
6. **Final system-test phase:** all 45 inventoried browser cases above wait for the end-to-end Rust server. No browser harness, browser/pixel matrix or Rust Docker build was attempted here.

No open product decision is required. Verification is green at the source SHA above; the partial implementation and unverified boundaries are explicit. This external report is byte-identical to its tracked copy.
