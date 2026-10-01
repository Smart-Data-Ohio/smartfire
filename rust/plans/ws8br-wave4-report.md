# WS8br detached card-zone follow-up — complete

Continued `rust/ws8br-followup` from Astra-approved `d8db7688a032b9ac2a1981f6907f8cca0a4a5bf5`. The code slice is `7a8d4d35787bb0d2e4be0f35f82ba8f94bf0031d`. No merge, stash, PR creation, new ignore, expectation weakening, timing threshold change, or test concurrency reduction was used. Compiler jobs remain two and tests retain the branch's four-thread baseline.

## Changes by file

- `controllers/presenters/page.rs`: a writer-thread zone guard retains the event actor's Rails `Time.zone` through ordered after-commit renderers. Drop restores the previous value, including validation-error paths. Detached rendering reads that scope; background rendering without an actor defaults to UTC. It carries no user, session, CSRF token, or CSP nonce.
- `controllers/rooms/events.rs`: event create, update and cancel use the existing database `write_scoped` API. Domain models, transaction semantics, callbacks and the shared broadcast fanout remain unchanged.
- `controllers/presenters/events.rs`: event replacement cards use the scoped actor zone instead of a hardcoded UTC zone. Rails renders one shared payload; the same complete Turbo frame is delivered to Hawaii/other actor and India recipient sockets.
- `controllers/messages.rs`: only two localized renderer hunks changed: the creation response and creation broadcast load the persisted actor/viewer zone and pass it into detached rendering. No selection, persistence, retry, authorization, or message-partial logic changed. WS8b-m owns this file and can reconcile these small hunks during its main merge.
- `controllers/presenters/github.rs`: the existing `message_cards` API remains; a zone-explicit entry point renders dated GitHub cards with the caller's context.
- `controllers/presenters.rs`: presenters inherit a scoped actor zone where present, otherwise UTC. Real viewer requests still load their persisted zone explicitly. Message collection fragments and the private GitHub card cache stamp include the non-UTC render zone so warm caches cannot reuse another viewer's datetime bytes. The domain cache-key API is unchanged.
- `controllers/searches/preloads.rs`: pre-rendered GitHub cards receive the presenter's zone; preloaded rendering retains its no-query contract.
- `controllers/rooms/refreshes.rs`: the refresh presenter loads the current viewer's zone before pre-rendering provider cards.
- `controllers/rooms/native_integration_tests.rs`: four regression cases exercise real HTTP controllers and two authenticated Action Cable recipients, with no injected provider HTML. Existing zone-fixture setup was extracted unchanged. Creation tests compare complete event-card containers in the response, append, reload, and idempotent retry; event edit tests compare the complete shared Turbo frame; GitHub tests compare four real HTTP routes and revisit the first viewer after caches warm.
- `controllers/rooms/card_write_zones.json`, `reference-tools/rooms/card_write_zones.rb`, and `check_card_write_zones.py`: a committed Rails-only corpus and reproducible recorder/checker. Sixty selected real HTTP responses cover ten creation/append containers, ten actor-zone edits, and forty GitHub containers, plus a direct background update's full UTC replacement frame. Twenty additional HTTP GETs obtain real CSRF tokens. The eight event/message/GitHub/SetTimeZone source hashes are verified against d7c7de92. Only the previously approved #163 page-layout drift is used for request setup.

## Failing first on d8db7688

Read Astra's probes under `/home/riels/.cache/rust-port/ws8brr/followup/evidence/`, including `raw-test-lines.txt`, without modifying them. Independently recorded expected bytes from Rails, added the regression fixture/tests, and ran this with all production code still unchanged at d8db7688:

```sh
CI=1 TMPDIR="$PWD/.scratch" CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire zone_audit_ -- --test-threads=4
```

Raw baseline lines (`.scratch/zone-write-followup/baseline-final.log`):

```text
test controllers::rooms::native_integration_tests::zone_audit_event_broadcast_matches_rails_actor_zone_for_every_recipient ... FAILED
test controllers::rooms::native_integration_tests::zone_audit_github_message_cards_match_rails_with_warm_zones ... FAILED
test controllers::rooms::native_integration_tests::zone_audit_message_creation_matches_rails_and_reload ... FAILED
test controllers::rooms::native_integration_tests::zone_audit_utc_and_invalid_zone_fallbacks_cover_all_three_paths ... ok
test result: FAILED. 1 passed; 3 failed; 0 ignored; 0 measured; 1426 filtered out; finished in 6.34s
```

| Regression | First difference and lengths | Rust on d8db7688 | Rails Hawaii |
| --- | --- | --- | --- |
| Event-edit shared frame | byte 505; Rust 1090, Rails 1100 bytes | start `2026-03-08T06:30:00Z`, end `2026-03-08T07:30:00Z` | start `2026-03-07T20:30:00-10:00`, end `2026-03-07T21:30:00-10:00` |
| Message-create event containers | byte 394; Rust 1829, Rails 1849 bytes | same spring UTC values; fall `2026-11-01T05:30:00Z` / `2026-11-01T06:30:00Z` | spring as above; fall `2026-10-31T19:30:00-10:00` / `2026-10-31T20:30:00-10:00` |
| GitHub message-card container | byte 969; Rust 1325, Rails 1330 bytes | `2026-07-01T12:00:00Z` | `2026-07-01T02:00:00-10:00` |

The raw actual/expected files remain in `rust/target/ws8bm-diffs/difference-UjBAoT`, `difference-rfvN5L`, and `difference-QcGueE`. No expected HTML was copied from Rust or adjusted to silence a mismatch. Fixture message IDs follow the Rails insertion sequence so attendance-frame IDs are compared exactly.

## UTC audit coverage and scope

All three former fallbacks now have separate non-UTC regressions; reverting any corresponding integration to UTC fails an exact Rails-byte comparison. The common fallback regression also exercises **all three paths** for UTC, null, blank, and invalid persisted zones. Non-UTC coverage includes Hawaii, Eastern Time, India, Nepal, Sydney and Adelaide. Creation/reload cards cross both Eastern DST transitions; GitHub updates use a July instant to exercise southern winter offsets. Repeated request-zone changes and warm fragment caches are covered.

The broadcast regression uses actual PATCH requests, actual after-commit callbacks and two subscribed sockets whose viewer zones differ. Both receive the same actor-zone frame. After each success and each 422 validation error, a direct model write must match the Rails background UTC frame, proving actor context does not survive into unrelated work. Creation appends are also compared for both recipients, with idempotent retries asserting no duplicate broadcast. Card comparisons assert the absence of session-bound HTML.

GitHub containers are compared through room show, room refresh, message index, and standalone message show. The original non-UTC room-list/thread-header checks and full-page goldens remain enabled. No message-list or composer template internals changed.

This closes the three fallbacks assigned in this follow-up, including the event/GitHub provider deferrals called out in the previous report. Other feature broadcasts and controllers remain with their existing owners; this report does not claim new coverage for message editing, polls, quotes, provider-refresh jobs, or browser/system inventory items. Browser pixel-diff work remains excluded. There is no remaining item in this assigned three-fix slice.

## Fresh-clone verification

Independently cloned the committed follow-up branch with:

```sh
git clone --no-local --single-branch --branch rust/ws8br-followup . .scratch/fresh18/repo
```

All subsequent commands ran from that clone, at code commit `7a8d4d35`, with newly built pinned `default` and `first_run` seeds and a new target directory. Environment used by `.scratch/zone-write-followup/verify-fresh.py`:

```sh
export CI=1 TMPDIR="$PWD/.scratch" CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
export CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 RUSTC_BOOTSTRAP=1
export PARITY_NAMESPACE=ws8br-fresh18-seed PARITY_OWNER=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="$PWD/rust/reference-tools/rooms/pinned_media_runner.py"
```

`RUSTC_BOOTSTRAP` enables libtest's duration reporting. The committed runner executes storage-vector binaries with the pinned reference media libraries; ordinary binaries run natively. Storage version and byte assertions remain enabled. The compiler and test limits were not increased. Exact commands, each rerun in the independent clone:

```sh
rust/parity/bin/seed build default first_run
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1
python3 rust/reference-tools/rooms/check_workspace.py
python3 rust/reference-tools/rooms/check_card_write_zones.py
python3 rust/reference-tools/rooms/check_event_zones.py
python3 rust/reference-tools/rooms/check_full_pages.py
python3 rust/reference-tools/rooms/check_empty_shell_http.py
python3 rust/reference-tools/rooms/check_thread_review.py
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire -- --test-threads=4 controllers::rooms::native_integration_tests controllers::rooms::full_page_tests controllers::channel_threads::page_tests controllers::channel_threads::content_tests controllers::rooms::refreshes_rails_cases channels::tests::events_test preloaded_quote_cards_render_without_queries full_message_preloads_keep_queries_constant
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml --workspace --no-fail-fast -- --test-threads=4 -Z unstable-options --report-time
mise exec rust@1.98.1 -- cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
bash rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo build --locked --workspace --bins
```

Cargo metadata exited zero; the duplicate-workspace-key check passed. The release-input guard builds from only Cargo.toml, Cargo.lock, and crates/ plus the Dockerfile's separate explicit Rails asset inputs, without vectors or reference tools. The report-only follow-up commit changes no tested code. Raw seed, manifest, and Rails receipts:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
Cargo TOML duplicate-key check: all manifests parse; no duplicate workspace dependency keys
Rails card write-zone oracle: 60 captured HTTP responses; 10 creation/append containers, 10 shared actor-zone replacements, 40 GitHub containers and one background UTC frame; 8 pinned sources; byte-identical
Rails viewer-zone HTTP oracle: 18 responses, 24 complete event containers and 6 complete PR headers; Hawaii, both Eastern DST transitions and UTC fallbacks; byte-identical
Rails full-page oracle: 4 complete pages reproduced; pinned rooms/messages and approved #163 layout verified; bytes unchanged
Rails empty-shell HTTP oracle: 4 full responses reproduced; 8 pinned/approved source files verified; bytes unchanged
Rails PR-thread HTTP oracle: 4 responses reproduced; 4 source files match d7c7de92; bytes unchanged
```

Fresh focused test receipts:

```text
    Finished `test` profile [unoptimized] target(s) in 4m 12s
test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 1396 filtered out; finished in 40.92s
```

Every workspace test/doctest harness raw summary:

```text
test result: ok. 1428 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 609.53s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.17s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 739 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 70.95s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.27s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.53s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.23s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.82s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.60s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.43s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.83s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.97s
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.21s
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

Across 60 harnesses: 2874 passed, zero failures, 11 existing ignores. The app ran 1428 passes and two existing ignores (`channels::tests::golden::record_reference` and `jobs::tests::push_latency`). No seeded app test silently skipped. No new ignore or timing flake occurred.

Clippy raw completion:

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 00s
```

Release-input build raw completion:

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 15s
```

Largest app cases by libtest duration (coverage retained):

```text
test controllers::rooms::events::tests::pr174_attendance_parameter_shapes_match_pinned_rails ... ok <81.954s>
test controllers::channel_thread_messages::tests::nested_reads_require_alive_membership_and_both_thread_and_message_scope ... ok <51.165s>
test controllers::fizzy_message_cards::tests::ws15e_fizzy_message_creation_http_matrix ... ok <47.505s>
test controllers::github::connection_tests::github_connections_http_identity_flash_revocation_and_audits_match_rails ... ok <41.927s>
test controllers::github::webhooks::tests::webhook_http_status_body_selection_and_privacy_match_rails ... ok <40.356s>
test controllers::github::write_tests::github_write_http_results_payloads_own_token_prompts_retry_text_and_streams_match_rails ... ok <36.952s>
test controllers::messages::room_list_tests::room_list_places_unread_outside_shared_fragments_and_matches_rails_around_pages ... ok <29.399s>
test controllers::github::subscription_tests::github_subscription_http_status_flash_token_events_and_membership_match_rails ... ok <25.786s>
test integrations::fizzy::agent_requests::tests::ws15e_fizzy_agent_requests_match_pinned_service_results ... ok <24.865s>
test app::round_four_security_tests::profile_zone_case_and_alias_validation_matches_rails_without_partial_writes ... ok <24.617s>
```

New zone-regression durations in the complete app run:

```text
test controllers::rooms::native_integration_tests::zone_audit_event_broadcast_matches_rails_actor_zone_for_every_recipient ... ok <1.300s>
test controllers::rooms::native_integration_tests::zone_audit_github_message_cards_match_rails_with_warm_zones ... ok <2.011s>
test controllers::rooms::native_integration_tests::zone_audit_message_creation_matches_rails_and_reload ... ok <3.668s>
test controllers::rooms::native_integration_tests::zone_audit_utc_and_invalid_zone_fallbacks_cover_all_three_paths ... ok <5.221s>
```

The independent clone target and release-input scratch directories were deleted after all verification processes exited. Raw receipts remain under `.scratch/fresh18/repo/.scratch/`; baseline receipts remain under `.scratch/zone-write-followup/`. No scratch target is retained. The external report and tracked mirror contain identical bytes.
