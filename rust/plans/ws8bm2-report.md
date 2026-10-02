# WS8bm2 PR #191 review fixes

This requested review slice is complete and PR-ready. Fresh-clone workspace verification passed: **4083 passed, 0 failed, 14 existing ignores**; strict all-target clippy, locked metadata, the release-input build and all 28 Rails oracle replays passed. The final commit after verified production head 1344ab35a changes this report only; its pushed SHA is in the final reply. The merge retains both the reviewed message renderers/features and WS14g Google sign-in/Calendar boot. Main merged at `0700e59d43cb0a0122036a6c3a8ea06ad3d9fc1a`; merge commit `033a9696db0d642ba16e494686ac9854e7b9e283`. Production fixes are `068f7f8dfd58f1b0337f827b0fe9e6f8abaf7df4` and `1344ab35a6b944ed4e8713f1d28c1615455cf6ad`.

## Failing first against the reviewed head

A separate clone at exact `9dd8263928d64abe7c485b660faa8977fc3bd167` independently rebuilt default and first_run Rails seeds. Only the new review test module, its inclusion and the Rails matrix vector were copied in. Its tracked production diff is exactly the three test-module inclusion lines in message_features.rs. No production fix was present. The four tests exited 101. Raw evidence: `.scratch/pr191/before.log`.

```text
WS8bm2 Rust warm HTTP search: 4 results; 266 SELECT/WITH executions
WS8bm2 Rust warm HTTP search: 16 results; 926 SELECT/WITH executions
WS8bm2 role/room matrix: 520 responses; 460 byte comparisons; 168 differences
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 2025 filtered out; finished in 36.11s
```

1. The production warm-cache HTTP query regression failed with left 266/right 926. The former constant-query test omitted cache_base_url; it now uses the production cache path and actual executed SQL, with mixed boost/poll messages. This strengthened test additionally failed in the intermediate controller run with 32 versus 44 reads: a singleton account read on each cold fragment miss; that account is now read lazily once per page.
2. The removed-member JSON poll assertion failed with `{"status":404,"error":"Not Found"}`. The complete matrix had 148 such body mismatches and 20 missing-message-link lookup-order mismatches (Rust 406 versus Rails 404).
3. Request-free scheduled dispatch delivered a real cable frame containing `data-actions-url="http://example.org/rooms/486777696/messages/935962058/actions"` despite APP_URL=https://scheduled.test:8443. The regression invokes the scheduled writer/dispatch path used by Periodic::ScheduledMessages and inspects the actual subscribed socket, without an HTTP OriginGuard or a sleep.

## Fixes and parity boundaries

Search collection aggregates, validators and individual rendered-record versions are batched by authorized root id. No whole-page maximum enters an individual message key. Reply and quote sources, nullable quote room names, users/mentions/voters/avatars, attachments/blobs/embeds, pins, polls/options/votes, agent steps, Drive rows, provider reference/card versions, PR thread state and only consulted workspace icon versions remain in the key. The viewer-zone expansion from #179 remains intact. Singleton versus batched keys are compared on mixed complete messages across UTC/Honolulu/New York and both DST transitions, and on the populated private-provider/quote fixture across edits, renames, poll changes and unpinning. Unused duplicate provider snapshots/renderers were removed; actual merged provider factories and refresh policies remain in use.

The shared Ctx finish path supports Rails controller-level rescue_from RecordNotFound => head :not_found. Shared message-feature scoping plus saves/scheduling preparation declare it. It does not turn routing errors or controllers without that Rails rescue into empty bodies: those still use the reviewed PublicExceptions renderer. MessageLinks performs lookup before format negotiation as Rails does. The role/room test retains Astra's comparison predicate: compare status for every response and exact body for JSON or empty bodies. The committed matrix omits unused HTML bodies carrying random tokens; it does not change which responses are compared.

Detached renderer defaults now share the validated configured APP_URL origin, with example.org only for an unconfigured app. Request-scoped origins keep precedence. The audit of jobs/channels/integrations found no other literal default-host rendering path after this change: GitHub card refresh/notifier, room directory, huddle effects and the message replacement sink all use the shared detached renderer/default helper. Existing fixture and test URLs are retained.

## Independent Rails counts and HTTP matrix

The committed review_query_counts.rb and review_http_matrix.rb are Astra's probes (the latter removes only unused bodies from output). The new review_denied_formats.rb checks identical removed-member requests as JSON, HTML and Turbo stream. Each ran against isolated copies of a rebuilt pinned d7c7de92 Rails seed.

```text
REVIEW Rails warm search 4 messages: 31 SELECT/WITH executions
REVIEW Rails warm search 16 messages: 32 SELECT/WITH executions
REVIEW Rails role/room matrix: 40 combinations; 520 responses recorded
REVIEW Rails denied response formats: 24/24 empty 404 bodies
```

The initial isolated Rails probe measured 31/31 reads for 4/16 results. The fresh-clone replay above measured 31/32; its extra statement is exactly `SELECT COUNT(*), MAX("updated_at") FROM "workspace_icons" LIMIT 1`. Rails Icons refresh_custom_icons uses a one-second CLOCK_MONOTONIC TTL, independently of the frozen Rails clock. This measurement is retained; no retry, timeout change or count mask was applied. The target is page-constant production cache dependency reads, not equality to a particular Rails cache TTL sample.

## Exact verification commands and raw receipts

All commands below were run this turn. Cargo used the owned `.scratch/pr191/ci-env.sh` adapter: unchanged machine-wide rustc slot file and wrapper, CARGO_BUILD_JOBS=2, the pinned speedup toolchain image, four-CPU containers, this worker's existing port ranges, and `/native-target` mapped to the ordinary owned `rust/target` build cache. Explicit -j4/four test threads were retained. Release, clippy and workspace Cargo commands ran sequentially. No retry, timeout, threshold or concurrency change was made.

Failing-first baseline (assigned worktree root; the clone was detached at exact 9dd82639 with only test inputs copied in):

```bash
source .scratch/pr191/ci-env.sh
CARGO_TARGET_DIR=/native-target bash .scratch/pr191/baseline/rust/ci/cargo.sh test --locked -p campfire -j4 controllers::message_features::review_tests -- --test-threads=4 --nocapture > .scratch/pr191/before.log 2>&1
```

Exit 101, with the four failures shown above. The intermediate controller run exposed the additional cold singleton-account reads; after 1344ab35a, the same strengthened assertion passes in the full fresh-clone workspace run.

```bash
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh test --locked -p campfire -j4 controllers:: -- --test-threads=4 > .scratch/pr191/controllers.log 2>&1
```

Intermediate raw result, retained rather than hidden:

```text
test result: FAILED. 1089 passed; 1 failed; 1 ignored; 0 measured; 1050 filtered out; finished in 510.05s
```

Fresh clone was created from the committed branch and advanced by a fast-forward to the cold-account fix before its suite ran. Only the prior owned Cargo registry cache was copied in; the fresh clone rebuilt both seeds itself. Neither the normal worktree's seeds nor its untracked code or test data supplied the suite.

```bash
git clone --no-hardlinks . .scratch/pr191/fresh
mkdir -p .scratch/pr191/fresh/.scratch/ws8bm2 .scratch/pr191/fresh/rust/.cargo-home
cp -a .scratch/pr191/baseline/rust/.cargo-home/registry .scratch/pr191/fresh/rust/.cargo-home/
git -C .scratch/pr191/fresh pull --ff-only origin rust/ws8bm2-message-features
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 bash .scratch/pr191/fresh/rust/parity/bin/seed build default first_run > .scratch/pr191/fresh/.scratch/ws8bm2/seeds.log 2>&1
```

Seed builder raw lines (exit 0):

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

The following commands ran from `.scratch/pr191/fresh/` (all exit 0):

```bash
source ../ci-env.sh
CARGO_TARGET_DIR=/native-target bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins > .scratch/ws8bm2/release-inputs.log 2>&1
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh metadata --locked --format-version 1 > .scratch/ws8bm2/metadata.json 2> .scratch/ws8bm2/metadata.log
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh clippy --locked --workspace --all-targets -j4 -- -D warnings > .scratch/ws8bm2/clippy.log 2>&1
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh test --locked --workspace --no-fail-fast -j4 -- --test-threads=4 --nocapture > .scratch/ws8bm2/workspace.log 2>&1
```

Cargo.lock unchanged; metadata stderr empty. Release-input and strict clippy raw final lines, respectively (terminal color escapes removed):

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 55.75s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 54.52s
```

Full workspace raw summary lines, including vendored html5ever and doctests (61 summaries total; 4083 passes, zero failures, 14 ignores):

```text
test result: ok. 2136 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 659.43s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.98s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1220 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 196.00s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.25s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.94s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.83s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.17s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 21.03s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.62s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.25s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.24s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.79s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.82s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Fresh-clone review results from that same suite:

```text
WS8bm2 denied response formats: 24/24 empty 404 bodies
WS8bm2 timer scheduled send: configured HTTPS origin and port delivered over cable
WS8bm2 Rust warm HTTP search: 4 results; 46 SELECT/WITH executions
WS8bm2 Rust warm HTTP search: 16 results; 46 SELECT/WITH executions
WS8bm2 role/room matrix: 520 responses; 480 byte comparisons; 0 differences
test controllers::searches::ports::full_message_preloads_keep_queries_constant ... ok
```

Tests explicitly skipped by their existing ignore declarations:

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test controllers::internal_huddle_tests::huddle_gateway_own_node_suite_against_rust_endpoints ... ignored, requires Node and the gateway pinned ws package; run explicitly with --ignored
test integrations::agent_jobs::drive_attachment_cases::ws14g_agent_polling_http_carries_an_empty_drive_array ... ignored, pending WS11-API PR: /agents/events shape at 60d97bd is not on main
test integrations::agent_jobs::drive_attachment_cases::ws14g_agent_polling_http_carries_drive_file_ids_and_urls_only ... ignored, pending WS11-API PR: /agents/events shape at 60d97bd is not on main
test jobs::tests::push_latency ... ignored, a measurement, not a test
test record_reference ... ignored, needs a running reference app; see the module docs
test tests::differential_test::scenario_matches_ruby ... ignored, needs CAMPFIRE_RUBY_SCENARIO_DB, the reference app's database after scenario.rb
test tests::fixtures_test::export_database_for_rails ... ignored, writes a database to CAMPFIRE_EXPORT_DB for the Rails rollback check
test tests::fixtures_test::fixtures_match_ruby_row_for_row ... ignored, needs CAMPFIRE_RUBY_FIXTURES_DB, a database the reference app filled with `db:fixtures:load` at CAMPFIRE_FIXTURES_NOW
test tests::two_factor_rollback_test::read_rails_rollback_changes ... ignored, requires Rails to mutate the rollback fixture; run reference-tools/auth/rollback.sh
test acme_tls_alpn_certificate_cached_and_reused ... ignored, requires a local Pebble ACME CA, PEBBLE_MINICA root certificate and TLS ports 5001/5002
test export_for_rails ... ignored, exports a database for the Rails rollback check; requires CAMPFIRE_MAIL_EXPORT_DIR
test crates/kit/src/error.rs - error::halt (line 91) ... ignored
test crates/kit/src/lib.rs - (line 7) ... ignored
```

The two WS14g polling cases explicitly await WS11-API. The missing-seed panic/"skipping locally" lines in nocapture output are the passing missing_seed_fails_in_ci/missing_seed_may_skip_locally negative-control tests, not skipped feature tests. No seed-dependent feature test silently skipped.

Oracle replay and source discrimination, fresh-clone `rust/` (exit 0):

```bash
python3 reference-tools/messaging/verify_oracles.py > ../.scratch/ws8bm2/oracles.log 2>&1
python3 reference-tools/messaging/features-reference-check.py > ../.scratch/ws8bm2/reference-check.log 2>&1
```

```text
WS8bm2 oracle replay: 28/28 independently replayed fixtures byte-identical
WS8bm2 reference source check: 106 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
```

Astra's Rails probes were also replayed from the fresh clone (fresh root; all exit 0, and cmp confirms the committed matrix byte for byte):

```bash
mkdir -p .scratch/ws8bm2/review-db
cp -a rust/parity/.seed/default/. .scratch/ws8bm2/review-db/
export PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92
bash rust/parity/bin/reference runner --storage .scratch/ws8bm2/review-db --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/messaging/review_query_counts.rb /rails/storage/db/review-counts.json > .scratch/ws8bm2/rails-counts.log 2>&1
bash rust/parity/bin/reference runner --storage .scratch/ws8bm2/review-db --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/messaging/review_http_matrix.rb /rails/storage/db/review-matrix.json > .scratch/ws8bm2/rails-matrix.log 2>&1
cmp .scratch/ws8bm2/review-db/db/review-matrix.json rust/vectors/messaging/review_http_matrix.json
bash rust/parity/bin/reference runner --storage .scratch/ws8bm2/review-db --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/messaging/review_denied_formats.rb > .scratch/ws8bm2/rails-denied-formats.log 2>&1
```

Their raw lines are above in Independent Rails counts and HTTP matrix. Rust has 46/46 reads versus fresh Rails 31/32 for 4/16 results (initial Rails 31/31); the one TTL stamp read is explicitly retained. All 520 statuses match, with 480 exact body comparisons and zero differences.



## Changed files and retained inventory

- `db/models/message_rendering.rs`: batch collection facts and voter users; filter root dependencies without database reads. Removed its unused duplicate provider/event/X snapshot modules and the unregistered presenter provider_cards.rs.
- `controllers/presenters/{message_cache,message_dependencies,message_cache_preloads}.rs`, `controllers/messages/freshness.rs`, `controllers/searches/preloads.rs`: production per-root aggregate/version/validator preloads. `presenters.rs` memoizes the detached account on cold misses.
- `kit/ctx.rs`, shared `controllers/message_features.rs`, saves/scheduling preparation and `rooms/message_links.rs`: declared controller rescue behavior and Rails lookup order.
- `controllers/presenters/page.rs`, `channels/{message_features,sink}.rs`: shared configured detached origin.
- `message_features/review_tests.rs`: four new failing-first review tests; `searches/ports.rs` and `message_features/root_cache_tests.rs`: strengthened production cache and mixed/DST key checks. The three review Ruby probes and matrix vector reproduce Astra's actual Rails requests.

Cross-owner changes are the shared WS8b-m presenter/cache and WS7 Kit/rendering sink integration, plus the requested merge of WS14g's boot/configuration. No controller-specific JSON-body patches or new owner API stand-ins were introduced. All 140 accepted controller behavior ports remain represented by their existing Rust tests and the 28 replayed Rails oracles; no Rails controller case is newly deferred in this slice. This is behavior coverage, not Rails test files executing directly against Rust.

The already accepted browser inventory below is retained as prior evidence from `9dd82639` and Astra's review, not as a claim of a new browser run. Browser behavior scripts, timeouts and harness are unchanged in this slice.

| Rails system file (`test/system/`) | Accepted Rails | Accepted Rust |
| --- | --- | --- |
| polls_test.rb | 4/4 | 4/4 |
| pins_saved_test.rb | 7/7 | 7/7 |
| slash_commands_test.rb | 26/26 | 26/26 |
| search_files_test.rb | 4/4 | 4/4 |
| scheduled_messages_test.rb | 4/4 | 4/4 |
| Total | 45/45 | 45/45 |

## Remaining scope

Nothing remains in these three P2 fixes and requested merge/verification slice. Stopping for the lead's review of PR #191. The wider workstream remains partial: additional populated-provider/older-window callback permutations and exceptional date/coercion matrices remain outside this request. WS12 #187 and WS11-API consumer integration retain their named owner dependencies. Existing real WS17 transport, WS11 human agent command and WS13 readiness integrations are retained. This is not an owner-blocked-only completion claim. Astra's accepted browser review was 45/45 on each app; this request does not rerun the browser suites.

## Cleanup and publication

The fresh clone's generated rust/target contained 32,806 file bytes and was deleted after the suite. The initial broad directory-name cleanup also matched registry cc source folders named target; all 14 were restored immediately, and their 56 files byte-verified against unchanged local crate archives. No production or test source changed. The ordinary native build cache and registry caches remain. All owned Cargo/reference containers exited; no owned app server or forwarder remains. No other worker's files, the configured rustc throttle, or the Python model server were touched. No stash, rebase, force push, PR creation or deployment was performed.

Raw receipts: `.scratch/pr191/before.log`, `.scratch/pr191/controllers.log`, `.scratch/pr191/fresh/.scratch/ws8bm2/` and `.scratch/pr191/cleanup.log`. The tracked report and authorized wave4/ws8bm2-report.md are identical. Push and stop for the lead's review.
