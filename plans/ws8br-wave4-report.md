# WS8br cache-sharing review fix — complete

Continued `rust/ws8br-followup` from `383d7f85b0c1cada45244f23ccd39682df40dfef`. Merged `origin/main` at `b908ebc28f5b13039ce4dcca427bb1fb314b3d88` with merge commit `c6473fb7207a9f40b6ca58b70675589bd7216a79`. The cache fix is `b219a24c`. No stash, rebase, new PR, ignore, expectation weakening, timing-threshold change, or test-concurrency reduction was used. Tests retain four threads and compiler jobs remain two. The Python model server was not modified or operated.

## Merge resolution

Only `controllers/presenters.rs` conflicted, at two formatting-related hunks. The preload constructor keeps main's formatted fields and the existing `render_zone` clone. The collection renderer keeps the existing zone-aware detached render and, for the failing-first baseline, retained the old suffix behavior. Main's WS13 huddle integration and WS8b-r2 user/profile/account/layout behavior remain. `presenters/room_native.rs` merged automatically, retaining both viewer-zone loading and main's unread-divider index. Locked metadata and the duplicate-workspace-key check passed immediately after the merge.

## Changes by file

- `controllers/presenters/message_cache.rs`: expand the existing Rails `TimeWithZone` timestamp components with the presenter's viewer zone. The record-version timestamp still follows Rails' UTC serialization. Keys contain existing local time fields, DST flag and abbreviation; they do not gain a zone-name component.
- `controllers/presenters.rs`: remove both zone-name suffixes introduced by the preceding fix (the collection fragment key and the private GitHub card stamp). Keep zone-aware HTML rendering. Ordinary messages with no timestamp components share fragments across Hawaii/India; Hawaii/Pacific-Honolulu aliases serialize identical existing timestamp components and share GitHub fragments. Distinct component values isolate genuinely different GitHub HTML.
- `controllers/rooms/native_integration_tests.rs`: three new regressions cover the two actual HTTP sharing sequences and exact Rails collection keys across five seasonal/DST instants and eight zones. Both sharing tests warm the real fragment store, verify it does not grow on the second zone, revisit the first zone, and compare the complete selected-message key map against Rails. The seasonal test covers both Eastern DST transitions, India, Nepal, Sydney, Adelaide, Hawaii, Pacific/Honolulu and UTC. Existing warm GitHub byte comparisons remain.
- `controllers/rooms/cache_zones.json`, `reference-tools/rooms/cache_zones.rb`, `check_cache_zones.py`: committed Rails-only expected keys and observable cache-write counts. The recorder makes 44 real HTTP requests to the collection-cached message index and uses the real cache store's write methods. Both warm pairs write 40 message fragments on the first request and remain at 40 on the second. The recorder rejects a zero-write observation. Four source hashes are checked against d7c7de92. No expectation was generated from Rust.
- `controllers/rooms/card_write_zones.json`, `reference-tools/rooms/card_write_zones.rb`, `check_card_write_zones.py`: extend the Rails corpus with ten real India-viewer reloads after actor edits. The existing actor-broadcast regression now compares that other viewer's complete event container with Rails, in addition to comparing the one actor-zone payload received by both sockets. All other creation, append, GitHub, background UTC, and invalid-zone checks remain. The reproducible corpus now has 70 selected HTTP responses and eight pinned source hashes.

No message-list or composer partial internals changed. The timestamp expansion changes the value of the existing presenter key, preserving its API. There is no remaining item in this P2 slice; unrelated controller/system inventory remains with its existing owners.

## Failing-first evidence

Before changing cache production code, ran the new regressions at merge commit c6473fb7. That merge preserves the reviewed 383d7f85 cache logic; it adds main's behavior. The newly generated fixtures came only from Rails at d7c7de92 plus the approved #163 request layout.

```sh
CI=1 TMPDIR="$PWD/.scratch" CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire zone_cache_ -- --test-threads=4
```

Raw failures (`.scratch/zone-cache-followup/baseline-final.log`):

```text
test controllers::rooms::native_integration_tests::zone_cache_hawaii_and_honolulu_share_github_fragments_like_rails ... FAILED
test controllers::rooms::native_integration_tests::zone_cache_hawaii_and_india_share_ordinary_fragments_like_rails ... FAILED
test controllers::rooms::native_integration_tests::zone_cache_timestamp_components_match_rails_across_dst_and_fractional_zones ... FAILED
assertion `left == right` failed: github_alias: Rails reuses identical keys; cold 44, warm 84
assertion `left == right` failed: ordinary: Rails reuses identical keys; cold 40, warm 80
assertion `left == right` failed: Rails collection timestamp components for message 935962043
  left: String("messages/935962043-20260302140000000000//0/0/14/2/3/2026/1/61/false/UTC/1//false/false////3")
 right: String("messages/935962043-20260302140000000000//0/0/4/2/3/2026/1/61/false/HST/1//false/false////3")
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 1666 filtered out; finished in 1.80s
```

The observed totals include main's merged seed/rendering behavior, so they differ from Astra's earlier 38→76 ordinary count. The defect is the same duplicate collection fragments. Rails' two message-index sharing pairs each retain exactly 40 written message fragments. The Rust sharing tests inspect actual room-page cache growth and then compare the same selected records' exact Rails key strings. They cannot pass by dropping the suffix while leaving timestamp expansion in UTC: the seasonal key regression and existing warm GitHub byte tests reject that implementation.

## Native media runtime diagnosis

The first full workspace run in the independent clone used the host's media libraries and had one failure, `controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers`: the complete PNG bytes for the `moon.jpg` logo differed. All three new cache regressions and the preserved timezone regressions passed. Its raw app summary:

```text
test result: FAILED. 1665 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 779.77s
```

Checked the same clean clone out to main `b908ebc2` and reran that exact logo test first with host libraries, then with the pinned Rails media runtime. Main's logo source and test are identical to this branch; the main probe confirms the failure depends on media libraries. No golden or test changed:

```sh
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers -- --test-threads=4
```

Raw main comparison (native, then pinned):

```text
    Finished `test` profile [unoptimized] target(s) in 4m 20s
assertion `left == right` failed: Some("moon.jpg") null complete Rails PNG bytes
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1657 filtered out; finished in 3.21s
    Finished `test` profile [unoptimized] target(s) in 0.18s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1657 filtered out; finished in 1.70s
```

Restored the fresh clone to `b219a24c` and extracted the pinned media runtime using main's committed helper, as documented by WS8b-r2. This installs only task-private libraries/tools and does not change host libraries:

```sh
PARITY_IMAGE=ws8br-reference-d7c7de92 WS8BR2_MEDIA_DIR="$PWD/.scratch/rails-media" bash rust/reference-tools/users/media_runtime.sh
```

Raw setup receipt:

```text
WS8br2 pinned media runtime: image ws8br-reference-d7c7de92; libvips, FFmpeg tools and libraries extracted; no host libraries changed
LD_LIBRARY_PATH=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8br/.scratch/fresh19/repo/.scratch/rails-media/native-libs
PATH=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8br/.scratch/fresh19/repo/.scratch/rails-media/usr/bin:$PATH

```

The complete workspace suite was rerun with this runtime, without changing coverage, threads, ignores or timing thresholds. The failed first run and both main-probe logs are retained alongside the corrected run.

## Release-input toolchain preparation

Strict workspace/all-target clippy passed on the first run. The first requested CI-wrapper build failed before compiling our crates: the pre-existing local `campfire-toolchain` image had Rust 1.98.1 but lacked `mold`/`ld.mold`, while the committed CI wrapper selects `-fuse-ld=mold`. The dependency build scripts for libc and proc-macro2 reported:

```text
error: linking with `cc` failed: exit status: 1
  = note: collect2: fatal error: cannot find 'ld'
          compilation terminated.
error: could not compile `libc` (build script) due to 1 previous error
error: could not compile `proc-macro2` (build script) due to 1 previous error
```

Created the task-local image `ws8br-ci-toolchain:b908-mold` from that existing image, appending the **unchanged toolchain installation RUN from the committed `rust/Dockerfile`** (clippy, mold and the checksum-pinned nextest binary). This preserves the existing Rust/media layers and does not rebuild native media with additional workers or replace the shared image. The Dockerfile used is `.scratch/zone-cache-followup/ci-toolchain/Dockerfile`. Executed:

```sh
docker build --label parity.owner=ws8br --label parity.namespace=ws8br-ci-cache-sharing --tag ws8br-ci-toolchain:b908-mold "$PWD/.scratch/zone-cache-followup/ci-toolchain"
```

Raw pinned-tool receipt:

```text
#5 2.944 /usr/src/nextest.tar.gz: OK
```

Reran the exact requested release-input command with `RUST_CI_IMAGE=ws8br-ci-toolchain:b908-mold`; its successful raw completion appears below. The initially failed linker log is retained as `release-inputs-missing-mold.log`. No repository source, build flags, assertions or expectations were changed to resolve either environment mismatch.

## Fresh-clone verification

Independently cloned the committed follow-up branch with:

```sh
git clone --no-local --single-branch --branch rust/ws8br-followup . .scratch/fresh19/repo
```

All branch-verification commands below ran from that clone, at code commit `b219a24c`, with newly built pinned `default` and `first_run` seeds and a new target directory. The initial receipts use `.scratch/zone-cache-followup/verify-fresh.py`; the corrected full-suite and clippy use `resume-fresh.py`, and the corrected CI build uses `release-fresh.py`, in the same scratch folder:

```sh
export CI=1 TMPDIR="$PWD/.scratch" CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
export CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 RUSTC_BOOTSTRAP=1
export PARITY_NAMESPACE=ws8br-fresh19-seed PARITY_OWNER=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="$PWD/rust/reference-tools/rooms/pinned_media_runner.py"
# For the corrected native workspace suite and clippy:
export PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs"
```

`RUSTC_BOOTSTRAP` enables libtest's duration reporting. The committed runner executes storage-vector binaries with the pinned reference media libraries; ordinary binaries run natively with the oracle's private media runtime described above. Storage version and byte assertions remain enabled. The compiler and test limits were not increased. The requested CI wrapper runs in the task-local pinned toolchain image described above. A task-local Docker adapter passes two Cargo jobs and a task-local copy of the configured rustc wrapper, with its container-invisible ancestry filter replaced by unconditional participation in the same machine-wide lock pool. It reads the original configured slot-count file and uses the original shared lock files. Neither the original throttle nor its settings was edited. This prevents the container build from bypassing the machine throttle. The successful CI build sets `RUST_CI_IMAGE=ws8br-ci-toolchain:b908-mold`. CI scratch uses `RUNNER_TEMP` in this clone, `RUST_CI_CONTAINER_PREFIX=ws8br-ci`, and `CARGO_TARGET_DIR=/src/rust/target`; no second extra target is created. Cargo color is disabled for plain raw summaries. Exact commands, each rerun in the independent clone:

```sh
rust/parity/bin/seed build default first_run
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1
python3 rust/reference-tools/rooms/check_workspace.py
python3 rust/reference-tools/rooms/check_cache_zones.py
python3 rust/reference-tools/rooms/check_card_write_zones.py
python3 rust/reference-tools/rooms/check_event_zones.py
python3 rust/reference-tools/rooms/check_full_pages.py
python3 rust/reference-tools/rooms/check_empty_shell_http.py
python3 rust/reference-tools/rooms/check_thread_review.py
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire -- --test-threads=4 controllers::rooms::native_integration_tests controllers::rooms::full_page_tests controllers::channel_threads::page_tests controllers::channel_threads::content_tests controllers::rooms::refreshes_rails_cases channels::tests::events_test preloaded_quote_cards_render_without_queries full_message_preloads_keep_queries_constant
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml --workspace --no-fail-fast -- --test-threads=4 -Z unstable-options --report-time
mise exec rust@1.98.1 -- cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins
```

Cargo metadata exited zero; the duplicate-workspace-key check passed. The release-input guard builds from only Cargo.toml, Cargo.lock, and crates/ plus the Dockerfile's separate explicit Rails asset inputs, without vectors or reference tools. The report-only follow-up commit changes no tested code. Raw seed, manifest, and Rails receipts:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
Cargo TOML duplicate-key check: all manifests parse; no duplicate workspace dependency keys
Rails cache-zone oracle: 44 HTTP responses; two warm sharing pairs; 40 timestamp keys across eight zones and five DST/seasonal instants; 4 pinned sources; exact keys and cache reuse
Rails card write-zone oracle: 70 captured HTTP responses; 10 creation/append containers, 10 shared actor-zone replacements, 10 other-viewer reloads, 40 GitHub containers and one background UTC frame; 8 pinned sources; byte-identical
Rails viewer-zone HTTP oracle: 18 responses, 24 complete event containers and 6 complete PR headers; Hawaii, both Eastern DST transitions and UTC fallbacks; byte-identical
Rails full-page oracle: 4 complete pages reproduced; pinned rooms/messages and approved #163 layout verified; bytes unchanged
Rails empty-shell HTTP oracle: 4 full responses reproduced; 8 pinned/approved source files verified; bytes unchanged
Rails PR-thread HTTP oracle: 4 responses reproduced; 4 source files match d7c7de92; bytes unchanged
```

Metadata raw orchestration receipt:

```text
END metadata.log: exit 0
```

Fresh focused test receipts:

```text
    Finished `test` profile [unoptimized] target(s) in 3m 44s
test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 1632 filtered out; finished in 44.49s
```

Every workspace test/doctest harness raw summary:

```text
test result: ok. 1666 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 1014.36s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.64s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.40s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 951 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 118.78s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.28s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.03s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.03s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.84s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.31s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.36s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.17s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.79s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.46s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.03s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.96s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.57s
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

Across 60 harnesses: 3328 passed, zero failures, 12 existing ignores. The app ran 1666 passes and three existing ignores (`channels::tests::golden::record_reference`, `jobs::tests::push_latency`, and the merged `controllers::internal_huddle_tests::huddle_gateway_own_node_suite_against_rust_endpoints`, which requires the explicit Node/gateway harness). No seeded app test silently skipped. No new ignore or timing flake occurred.

Clippy raw completion:

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 30s
```

Release-input build raw completion:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 27s
```

Largest app cases by libtest duration (coverage retained):

```text
test controllers::rooms::events::tests::pr174_attendance_parameter_shapes_match_pinned_rails ... ok <128.057s>
test controllers::github::webhooks::tests::webhook_http_status_body_selection_and_privacy_match_rails ... ok <63.862s>
test controllers::fizzy_message_cards::tests::ws15e_fizzy_message_creation_http_matrix ... ok <61.944s>
test controllers::github::write_tests::github_write_http_results_payloads_own_token_prompts_retry_text_and_streams_match_rails ... ok <58.525s>
test controllers::channel_thread_messages::tests::nested_reads_require_alive_membership_and_both_thread_and_message_scope ... ok <55.028s>
test controllers::messages::paging_tests::root_formats_and_destroy_side_effects_match_rails ... ok <53.676s>
test controllers::messages::paging_tests::pages_match_rails_tuple_edges_formats_and_etag_bytes ... ok <51.896s>
test controllers::messages::paging_tests::page_anchors_require_alive_membership_and_a_root_message ... ok <50.859s>
test controllers::github::connection_tests::github_connections_http_identity_flash_revocation_and_audits_match_rails ... ok <50.540s>
test controllers::messages::room_list_tests::room_list_places_unread_outside_shared_fragments_and_matches_rails_around_pages ... ok <49.762s>
```

New zone-regression durations in the complete app run:

```text
test controllers::rooms::native_integration_tests::zone_audit_event_broadcast_matches_rails_actor_zone_for_every_recipient ... ok <2.089s>
test controllers::rooms::native_integration_tests::zone_audit_github_message_cards_match_rails_with_warm_zones ... ok <2.269s>
test controllers::rooms::native_integration_tests::zone_cache_hawaii_and_honolulu_share_github_fragments_like_rails ... ok <1.311s>
test controllers::rooms::native_integration_tests::zone_cache_hawaii_and_india_share_ordinary_fragments_like_rails ... ok <1.212s>
test controllers::rooms::native_integration_tests::zone_audit_message_creation_matches_rails_and_reload ... ok <4.702s>
test controllers::rooms::native_integration_tests::zone_cache_timestamp_components_match_rails_across_dst_and_fractional_zones ... ok <1.319s>
test controllers::rooms::native_integration_tests::zone_audit_utc_and_invalid_zone_fallbacks_cover_all_three_paths ... ok <7.873s>
```

The independent clone target measured 16G before removal. The target and release-input scratch directories were deleted after all verification processes exited. Raw receipts remain under `.scratch/fresh19/repo/.scratch/`; baseline receipts remain under `.scratch/zone-cache-followup/`. No scratch target is retained. The external report and tracked mirror contain identical bytes.
