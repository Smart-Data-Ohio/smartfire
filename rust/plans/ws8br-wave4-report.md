# WS8br viewer-zone follow-up — complete

Created `rust/ws8br-followup` from `origin/main` at `434d1c14a0d47a6e6b5172404bcda2607530b167`. The code slice is `c53e4809`. No PR was opened. No stash, expectation weakening, added ignore, timing threshold change, or concurrency reduction was used. Tests retain this branch's four-thread baseline; compilation retains the existing two-job limit.

## Changes

- `controllers/presenters/page.rs`: added explicit zone input for detached nested request partials. Existing detached/background entry points retain their UTC default and token-free context.
- `controllers/presenters.rs`: carries the persisted viewer's Rails-compatible zone and preserves it through preload cloning.
- `controllers/presenters/room_native.rs` and `room_list.rs`: load the viewer zone before rendering the owner message-list seam. The existing `room_message_list(records, divider_id, unread_count)` entry point and owner partials are unchanged.
- `controllers/messages.rs` and `channel_threads.rs`: the audit found the same UTC fallback in the standalone GitHub thread header. The existing request presenter loads the viewer zone, and the real GitHub provider renders with that zone. No provider HTML is injected.
- `controllers/rooms/native_integration_tests.rs`: four real HTTP regressions compare complete card containers against Rails for Hawaii, Eastern Time, UTC, null, blank, and invalid zones. Eastern events cross both DST transitions. Repeated viewer changes exercise warm fragment caches. The tests persist Rails-produced model records and assert that compared card HTML contains no session-bound tokens.
- `controllers/rooms/event_zones.json`, `reference-tools/rooms/event_zones.rb`, and `check_event_zones.py`: committed Rails-only records, complete HTML containers, source hashes, and a reproducible HTTP recorder/checker. The corpus has 18 actual Rails responses, 24 complete event-card containers, and 6 complete GitHub thread headers. It uses the pinned d7c7de92 event/GitHub/SetTimeZone sources and the separately approved #163 page layout. The checker verifies all five source hashes against the pin. No golden was derived from Rust.

The fix passes only the zone into the detached render context. `Current.user`, CSRF tokens, CSP nonces, request cookies, and session state remain absent. The normal request layout and nested list therefore serialize the same instants with the same offsets as Rails. Event-card display text retains the event's own scheduling zone.

## Failing-first evidence

With production source unchanged at `434d1c14`, built the pinned `default` and `first_run` seeds, then ran:

```sh
CI=1 TMPDIR="$PWD/.scratch" CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire native_room_event_cards_match_rails -- --test-threads=4
```

Raw result:

```text
test controllers::rooms::native_integration_tests::native_room_event_cards_match_rails_both_eastern_dst_transitions ... FAILED
test controllers::rooms::native_integration_tests::native_room_event_cards_match_rails_hawaii_and_warm_viewer_changes ... FAILED
test controllers::rooms::native_integration_tests::native_room_event_cards_match_rails_utc_and_invalid_zone_fallbacks ... ok
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 1422 filtered out; finished in 1.33s
```

The Hawaii container first differed at byte 392; Eastern at byte 395. Rust returned 950-byte containers, Rails 960-byte containers. Exact spring event datetime values:

| Attribute | Rust before fix | Rails Hawaii | Rails Eastern |
| --- | --- | --- | --- |
| start | `2026-03-08T06:30:00Z` | `2026-03-07T20:30:00-10:00` | `2026-03-08T01:30:00-05:00` |
| end | `2026-03-08T07:30:00Z` | `2026-03-07T21:30:00-10:00` | `2026-03-08T03:30:00-04:00` |

The fall fixture also compares both occurrences of Eastern 01:30, with `-04:00` then `-05:00`. Before changing the thread-header integration (which was still identical to `434d1c14`), ran the additional HTTP regression:

```sh
CI=1 TMPDIR="$PWD/.scratch" CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire native_thread_header_matches_rails_viewer_zones -- --test-threads=4
```

```text
test controllers::rooms::native_integration_tests::native_thread_header_matches_rails_viewer_zones ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1425 filtered out; finished in 0.98s
```

The header first differed at byte 1194 (Rust 1859 bytes, Rails 1864): Rust emitted `2026-07-01T12:00:00Z`; Rails Hawaii emitted `2026-07-01T02:00:00-10:00`, and Eastern emitted `2026-07-01T08:00:00-04:00`.

Raw logs remain in `.scratch/zone-followup/baseline.log` and `header-before.log`; full first-difference byte files remain in the canonical `rust/target/ws8bm-diffs/difference-htsdH0`, `difference-KfkN9z`, and `difference-QDy24C` directories. The final comparisons preserve complete source containers, without DOM reserialization or datetime masking.

## Detached/broadcast audit

| Scope path | Result |
| --- | --- |
| Room-native load and detached message-list render | Fixed; real room HTTP comparisons cover Hawaii, both Eastern DST transitions, and UTC fallbacks. |
| Standalone thread PR header | Same fallback found and fixed; real GitHub provider and HTTP path compared against Rails. |
| Room header/sidebar HTTP page render, composer, pending template, thread/poll/pin panels | Use the request layout's `Zone::for_user` context. No additional detached UTC fallback. |
| Room refresh and pin list integration | `page::bare` loads the viewer zone. Refresh event-card bytes pass the non-UTC oracle; pins receive that same request context. |
| Room files integration | Uses `page::framed_page`; upload/Drive datetime tags read `ctx.time_zone`. |
| `rooms::render_shared_room`, `render_shared_header`, `render_membership_sidebar` | Their shared room rows and header identity partials format no dates. Existing detached UTC context cannot change their bytes. |
| `rooms::directs::broadcast_create_room`, `rooms::involvements` direct-row render | Identity/menu/member facts; no date formatting. |
| `channels::rooms_directory::render` DirectSidebar and RoomHeader descriptors | Identity/member facts; no date formatting. |
| Room category/favorite/read/member/inbound-address/switcher/sidebar controllers | No further detached date formatting; ordinary page rendering already carries the viewer zone. JSON timestamps keep their Rails serialization contract. |

The wider search also located explicit UTC contexts in the provider-owned `presenters::github::message_cards` (WS15g/WS8b-m's pre-rendered shared message cards) and `presenters::events::cards` (WS14e's shared EventCards broadcasts). Those are outside this room/thread integration slice and were not modified. Their shared-cache/broadcast actor context needs a separate owner comparison before changing serialization; this report does not certify their non-UTC broadcast parity. Ordinary message broadcasts, poll/quote feature broadcasts, and provider-specific card caches remain with their existing owners. No owner partial internals were ported or changed.

No requested fix remains in WS8b-r's detached room/thread paths. Previous broader controller/system deferrals remain in their existing inventories; this follow-up does not claim to close them.

## Fresh-clone verification

Independently cloned the committed follow-up branch with:

```sh
git clone --no-local --single-branch --branch rust/ws8br-followup . .scratch/fresh17/repo
```

All subsequent commands ran from that clone, at code commit `c53e4809`, with newly built pinned `default` and `first_run` seeds and a new target directory. Environment used by `.scratch/zone-followup/verify-fresh.py`:

```sh
export CI=1 TMPDIR="$PWD/.scratch" CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
export CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 RUSTC_BOOTSTRAP=1
export PARITY_NAMESPACE=ws8br-fresh17-seed PARITY_OWNER=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="$PWD/rust/reference-tools/rooms/pinned_media_runner.py"
```

`RUSTC_BOOTSTRAP` enables libtest's duration reporting. The committed runner executes storage-vector binaries with the pinned reference media libraries; ordinary binaries run natively. Storage version and byte assertions remain enabled. The compiler and test limits were not increased. Exact commands, each rerun in the independent clone:

```sh
rust/parity/bin/seed build default first_run
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1
python3 rust/reference-tools/rooms/check_workspace.py
python3 rust/reference-tools/rooms/check_event_zones.py
python3 rust/reference-tools/rooms/check_full_pages.py
python3 rust/reference-tools/rooms/check_empty_shell_http.py
python3 rust/reference-tools/rooms/check_thread_review.py
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire -- --test-threads=4 controllers::rooms::native_integration_tests controllers::rooms::full_page_tests controllers::channel_threads::page_tests controllers::channel_threads::content_tests controllers::rooms::refreshes_rails_cases channels::tests::events_test
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
Rails viewer-zone HTTP oracle: 18 responses, 24 complete event containers and 6 complete PR headers; Hawaii, both Eastern DST transitions and UTC fallbacks; byte-identical
Rails full-page oracle: 4 complete pages reproduced; pinned rooms/messages and approved #163 layout verified; bytes unchanged
Rails empty-shell HTTP oracle: 4 full responses reproduced; 8 pinned/approved source files verified; bytes unchanged
Rails PR-thread HTTP oracle: 4 responses reproduced; 4 source files match d7c7de92; bytes unchanged
```

Fresh focused test receipts:

```text
    Finished `test` profile [unoptimized] target(s) in 3m 08s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 1398 filtered out; finished in 35.19s
```

Every workspace test/doctest harness raw summary:

```text
test result: ok. 1424 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 640.55s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.60s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.13s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 739 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 73.00s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.19s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.27s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.38s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.56s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.80s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.04s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.38s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.26s
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.37s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.58s
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

Across 60 harnesses: 2870 passed, zero failures, 11 existing ignores. The app ran 1424 passes and two existing ignores (`channels::tests::golden::record_reference` and `jobs::tests::push_latency`). No seeded app test silently skipped. No new ignore or timing flake occurred.

Clippy raw completion:

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 07s
```

Release-input build raw completion:

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 39s
```

Largest app cases by libtest duration (coverage retained):

```text
test controllers::rooms::events::tests::pr174_attendance_parameter_shapes_match_pinned_rails ... ok <96.404s>
test controllers::channel_thread_messages::tests::nested_reads_require_alive_membership_and_both_thread_and_message_scope ... ok <60.960s>
test controllers::fizzy_message_cards::tests::ws15e_fizzy_message_creation_http_matrix ... ok <42.288s>
test controllers::github::connection_tests::github_connections_http_identity_flash_revocation_and_audits_match_rails ... ok <39.488s>
test controllers::github::webhooks::tests::webhook_http_status_body_selection_and_privacy_match_rails ... ok <37.603s>
test controllers::github::write_tests::github_write_http_results_payloads_own_token_prompts_retry_text_and_streams_match_rails ... ok <33.705s>
test integrations::fizzy::agent_requests::tests::ws15e_fizzy_agent_requests_match_pinned_service_results ... ok <33.137s>
test controllers::messages::room_list_tests::room_list_places_unread_outside_shared_fragments_and_matches_rails_around_pages ... ok <30.842s>
test controllers::github::agent_tests::github_agent_http_authorization_actions_replay_budgets_and_throttle_match_rails ... ok <27.927s>
test controllers::github::subscription_tests::github_subscription_http_status_flash_token_events_and_membership_match_rails ... ok <26.337s>
```

New zone-regression durations in the complete app run:

```text
test controllers::rooms::native_integration_tests::native_room_event_cards_match_rails_both_eastern_dst_transitions ... ok <1.278s>
test controllers::rooms::native_integration_tests::native_room_event_cards_match_rails_hawaii_and_warm_viewer_changes ... ok <1.184s>
test controllers::rooms::native_integration_tests::native_room_event_cards_match_rails_utc_and_invalid_zone_fallbacks ... ok <1.253s>
test controllers::rooms::native_integration_tests::native_thread_header_matches_rails_viewer_zones ... ok <0.780s>
```

The independent clone target and release-input scratch directories were deleted after all verification processes exited. Raw receipts remain under `.scratch/fresh17/repo/.scratch/`; baseline receipts remain under `.scratch/zone-followup/`. No scratch target is retained. The external report and tracked mirror contain identical bytes.
