# WS8br Wave 4 continuation report — PARTIAL

This continuation merges `origin/main` at `eaba80d5` (#168), fixes eight of the nine native byte differences and maps five more original controller declarations. The latest tested Rust source is `def60c7b`; the final report commit changes documentation/inventory only. The seeded fresh-clone workspace has 2395 passes, 0 failures and 10 existing ignores across 58 harness summaries. No seed-dependent case silently skips. Full native room-page byte acceptance is still partial: one message-list region and ten original controller mappings remain. Browser/system acceptance stays inventoried for the end-to-end phase.

## Pushed slices and files

| Commit | Change |
| --- | --- |
| `49e120fe` | Merge current main with a merge commit. Locked metadata succeeds; every Cargo TOML parses without duplicate dependency keys. |
| `20fc79cc` | Adapt root composer/footer and pending-template boundary bytes; add eight native region assertions using main's shared live-asset helper. Initial list boundary placement is corrected in the later slice. |
| `9ffc23e5` | Add the three RoomsController link-preview cases, complete HTTP switcher query measurement, and complete HTTP named/unnamed group-DM sidebar query measurement. Trace actual SQLite executions on the writer and every pooled reader. Inventory reads the pinned Git declarations rather than silently incorporating post-pin Rails tests. |
| `def60c7b` | Keep the shared message-list seam unchanged and add room-only whitespace in `room_native::message_list`. Apply the shared asset helper to the complete standalone thread-page golden. Record exact residual owner card bytes and independent boundary discriminators. |

Production mounting changes are in `controllers/presenters/room_native.rs` and `controllers/rooms.rs`. `presenters/room_list.rs` ends identical to its pre-continuation implementation: owner list callers and its unread/anchor corpus retain their original bytes. Tests extend `rooms/rooms_rails_cases.rs`, `sidebars_rails_cases.rs`, `switchers_rails_cases.rs`, `native_integration_tests.rs` and `channel_threads/page_tests.rs`. The new `rooms/query_probe.rs` observes real full-request SQL and detaches callbacks before freeing state, reusing WS13's test tracing pattern. Query tests stop their own job runner so unrelated background reads cannot contaminate either leg; harness concurrency remains four. No room/message/pin/agent/presence policy was reimplemented.

`test-support/asset_goldens.rs` comes directly from main. It validates actual pipeline digests in identified local URL fields and keeps surrounding bytes exact. The thread golden's old `people-b8926caa.css` field corresponds to the live `people-8adb2aea.css` field. Its newly created parent/replies are text-only, with no serialized fixture asset URL requiring a frozen field. No custom masks or parity allowlists were added; the separate strict native comparator still rejects the remaining list.

## Native bytes and the exact remaining difference

Fixed: composer and pending template for Designers (`654632876`), pair DM (`186869642`) and group DM (`699448329`), plus both DM message lists. Root footer capture contributes two spaces after removing the owner partial's one initial newline. Pending template contributes the missing final newline. Only the rooms/show list mount contributes `"\n    \n"`; the owner's standalone partial is unchanged.

Raw fresh-clone comparison:

```text
FAIL room 654632876 message_list: Rust 286287 bytes, Rails 288823 bytes
PASS room 654632876 composer: 10373 exact bytes
PASS room 654632876 pending_template: 1449 exact bytes
PASS room 186869642 message_list: 19547 exact bytes
PASS room 186869642 composer: 10369 exact bytes
PASS room 186869642 pending_template: 1449 exact bytes
PASS room 699448329 message_list: 6559 exact bytes
PASS room 699448329 composer: 8596 exact bytes
PASS room 699448329 pending_template: 1464 exact bytes
Native room component acceptance: 8 exact matches; 1 differences; no masks
```

The populated Designers list is Rust 286287 bytes versus Rails 288823 bytes. Its sole remaining 2536-byte shortfall consists of these four empty owner containers:

| Owner slot | Rust bytes | Rails bytes |
| --- | ---: | ---: |
| `github_pr_cards` | 101 | 1190 |
| `fizzy_cards` | 93 | 272 |
| `link_embed_cards` | 103 | 543 |
| `linkedin_cards` | 104 | 932 |

`rust/plans/ws8br-native-residual.json` contains **both complete Rails and Rust strings** for each unmatched region. Rust emits the existing empty `<div id="..." class="..."></div>` container; Rails adds respectively the public PR card (including discussion link), lazy Fizzy card frame, generic link article, and LinkedIn article. The diagnostic verifies that these four regions account for every remaining byte, without changing or accepting a production render. Full actual/expected list bytes and the unified diff are retained under the fresh clone's `.scratch/native-components-diff/654632876-message_list.*`.

This difference remains with owner integration: WS15g supplies GitHub and WS15e supplies Fizzy/link/LinkedIn. Their code is on unmerged owner branches, not main. Their full branch merges also change shared transport, jobs, networking, models and page adapters; this continuation does not substitute local implementations for those owners or edit WS8bm's message/composer internals. The lead must reconcile those owner branches and call their real component factories. Native capture checks all root selections, asserts the eight completed regions, and explicitly leaves the Designers child-provider region to the nonzero strict checker. It does not claim all nine regions or a full page pass.

## Exact shell-to-owner inputs

The contract is documented in `rust/plans/ws8br-owner-integration.md`. The shell passes:

- Actual reader/app, verified request origin, request host and shared fragment store; original root `Message` records (last 40, or 40 before + root anchor + 40 after), with foreign/missing/thread anchors falling back to the last page.
- `room_native::message_list(&presenter, &messages, divider.message_id, divider.count)`, which calls the unchanged `Presenter::room_message_list` and adds only the rooms/show caller prefix. Divider facts come from the viewing membership's last-read pointer/unread timestamp; they stay outside shared message fragments.
- `composer_facts(&room, &viewer, None, drive_flow)`: room ID/kind/display name, root thread `None`, ordered static and agent slash commands, Drive `None`/`Metadata`. WS14 Picker availability is still `false`; configured `Share` remains open.
- The real request `ViewContext`, user ID/name/title/fresh signed avatar, account, request CSRF/CSP values and chrome. WS8bm2's `ComposerButton {ctx, room_id, thread_id: None}` becomes the composer's trusted scheduled control; WS8bm's `PendingTemplate {ctx, user}` is mounted once.
- Room/header/involvement facts, selected cached fragments, updated timestamp, original/unpaged invitation, join code, signed message stream and WS17 OOO member facts. Voice/stage/board `RoomKind` remains the previous Closed fallback pending the corresponding screen owners; header/refresh target STI keys are preserved.

Full root thread panel, complete pins panel, poll builder, configured huddle header/sidebar and configured Picker remain owner integrations. Pin refresh continues calling WS8bm2's real list/count seam; no pin logic is ported here.

## Original controller mappings and exact remainder

Pinned Rails: 14 owned controller files, 161 original declarations, all 161 executed successfully in Rails. Rust: **151 individually named original mappings passed, 10 deferred**. The two additional members regressions and one additional pins regression are separate from this original-case count. All previously delivered room CRUD, authorization, membership, unread/involvement/favorite/category, DM error/forms/notes, refresh/pins, members JSON and inbound-address cases remain in the fresh-clone run.

| Rails file | Rust mapped original passes | Remaining |
| --- | ---: | ---: |
| rooms/directs | 29 | 0 |
| rooms_controller | 27 | 2 |
| rooms/opens | 15 | 0 |
| users/sidebars | 6 | 8 |
| rooms/members | 13 | 0 |
| rooms/closeds | 12 | 0 |
| rooms/involvements | 8 | 0 |
| rooms/inbound_email_addresses | 8 | 0 |
| rooms/reads | 7 | 0 |
| rooms/favorites | 6 | 0 |
| room_categories | 6 | 0 |
| rooms/categories | 5 | 0 |
| switchers | 5 | 0 |
| rooms/refreshes | 4 | 0 |

Remaining, by file:

`test/controllers/rooms_controller_test.rb`:
1. `show renders collapsed work-thread guidance in the new-thread panel` — WS8bm root-panel factory absent.
2. `destroy succeeds when the queue is down and the sweep recovers the room` — conflicts with lead decision 2's atomic enqueue/rollback contract. Existing native HTTP queue fault injection is separate and is not mislabeled as this Rails case.

`test/controllers/users/sidebars_controller_test.rb`:
1. `channel row shows the live huddle stack with names and count`.
2. `board row shows the live huddle stack with names and count`.
3. `direct row shows the live huddle stack when the peer is in the call`.
4. `quiet rows keep an empty stack target with no visible presence`.
5. `direct row re-renders when a participant joins`.
6. `group direct rooms render member names and a huddle stack`.
7. `no channel or DM stacks without huddle configuration`.
8. `sidebar query count does not grow with quiet channels, DMs, boards, and stages` — includes the source assertion of exactly one `huddle_grants` SELECT.

These eight need WS13's grant/presence and configured stack/call-row adapter integration. The named/unnamed group-DM query case now measures the complete live HTTP request and detects per-row queries; it does not close the separate huddle row assertions. The source inventory deliberately stays at `d7c7de92` so main's additional status-popup/card declaration is not silently attributed to this worker. Post-pin status-popup/card parity stays with WS8br2.

The broader inventory still includes all 58 original files and 512 source-declared cases, other owners and browser/system declarations. Browser/system acceptance stays inventoried: DM picker/create/settings, navigation/header/sidebar/switcher, per-viewer unread behavior, full list/composer/panels, responsive/accessibility/motion, configured huddle/venue states, and inbound-email reveal/enable/regenerate/disable/non-admin denial. Earlier shipped browser probes are tracked, not rerun here.

## Discrimination and resolved verification failures

Sequentially from the canonical worktree:

```sh
python3 rust/reference-tools/rooms/native_boundary_discrimination.py
python3 rust/reference-tools/rooms/remaining_cases_discrimination.py
```

Each boundary is independently compiled without its adapter; each must fail. The preview regression trusts the attachment image URL and unsanitized body, rejecting all three security-relevant cases. The query regression inserts real SELECTs per membership while keeping response data intact; both full-request measurements reject it. Sources are restored in `finally`; no broken implementation is committed. Raw results:

```text
list: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1036 filtered out; finished in 1.50s
composer: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1036 filtered out; finished in 1.13s
template: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1036 filtered out; finished in 0.78s
Native boundary discrimination: three independently compiled unadapted owner seams rejected; source restored
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 1034 filtered out; finished in 1.29s
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 1035 filtered out; finished in 0.89s
Remaining case discrimination: unsafe previews and per-row HTTP queries rejected; source restored
```

The five new case run and corrected owner integration run respectively:

```sh
CI=1 TMPDIR="$PWD/.scratch" CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire --bin campfire -- show_costs_a_constant_number_of_queries_as_rooms_people_and_threads_grow sidebar_query_count_does_not_grow_with_group_dms_named_or_not show_renders_a_link_preview show_renders_an_unfurled_link_preview --test-threads=4
CI=1 TMPDIR="$PWD/.scratch" CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire --bin campfire -- native_component_capture_matches_rails_root_selection complete_standalone_thread_templates_match_rails_layout_bytes room_list_places_unread_outside_shared_fragments_and_matches_rails_around_pages --test-threads=4
```

```text
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 1032 filtered out; finished in 1.42s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1034 filtered out; finished in 3.93s
```

The first full fresh-clone run exposed the shared list seam's six extra bytes and stale live `people.css` fingerprint in the complete thread golden:

```text
test result: FAILED. 1033 passed; 2 failed; 2 ignored; 0 measured; 0 filtered out; finished in 174.00s
```

Both were corrected and the entire fresh-clone workspace rerun: all regular harnesses pass, including the app. I overlapped clippy with the final doctests on the same target; ten doctest targets then reported E0463 crate-linkage errors. After clippy completed, all workspace doctests were rerun sequentially. Combined regular plus sequential doctest summaries below include every harness once; no failed compiler invocation is counted as a pass. These failures are not labeled inherited or timing flakes. Earlier build attempts failed before tests (rustc SIGKILL, then temporarily missing self-contained linker); unchanged-concurrency retries completed. No timing test flaked, no deadline was widened and no harness concurrency was lowered.

## Fresh-clone verification

Fresh clone: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8br/.scratch/fresh-continue9/repo`. It was created with `git clone --no-local --single-branch --branch rust/ws8br-rooms-http . .scratch/fresh-continue9/repo`, then fast-forwarded to the pushed source slices before verification. Its compiler target began empty. Seeds were independently built from tracked tooling and the pinned Rails image; no canonical seed, target, captured response or scratch fixture was copied in. Only generated `.scratch/` output is untracked. The final source checked is `def60c7b`; later report/inventory changes do not change Rust source.

From that clone, rerun commands:

```sh
PARITY_NAMESPACE=ws8br-fresh9-seed PARITY_OWNER=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 rust/parity/bin/seed build default first_run
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path rust/Cargo.toml >/dev/null
python3 rust/reference-tools/rooms/check_workspace.py
CI=1 TMPDIR="$PWD/.scratch" CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 RUSTC_BOOTSTRAP=1 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml --workspace --no-fail-fast -- --test-threads=4 -Z unstable-options --report-time
CI=1 TMPDIR="$PWD/.scratch" CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 RUSTC_BOOTSTRAP=1 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml --workspace --doc -- --test-threads=4 -Z unstable-options --report-time
CI=1 TMPDIR="$PWD/.scratch" CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 mise exec rust@1.98.1 -- cargo clippy --locked -j4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
CI=1 TMPDIR="$PWD/.scratch" CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 RUSTC_BOOTSTRAP=1 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire --bin campfire native_component_capture_matches_rails_root_selection -- --test-threads=4 --nocapture
python3 rust/reference-tools/rooms/native_components_check.py --capture-log .scratch/native-capture.log
python3 rust/reference-tools/rooms/native_residual.py --capture-log .scratch/native-capture.log
```

The strict comparator exits 1 for the reported owner-card difference. All regular workspace harnesses pass; the sequential doctest rerun and other final checks exit 0. The initial combined workspace command exits 101 at doctest linkage, as recorded below. Metadata intentionally has no stdout. `RUSTC_BOOTSTRAP` is used to obtain libtest timing output on stable 1.98.1; debug symbols are disabled for disk usage, not debug assertions/deadlines. Seed/key/clippy raw lines:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
Cargo TOML duplicate-key check: all manifests parse; no duplicate workspace dependency keys
    Finished `dev` profile [unoptimized] target(s) in 4m 09s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1036 filtered out; finished in 1.04s
```

Raw regular-harness plus successful sequential doctest summaries:

```text
test result: ok. 1035 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 236.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.96s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.25s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 697 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 74.17s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.66s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.10s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.03s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.36s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.34s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.60s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 25.13s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.04s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.39s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.39s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.14s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.45s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.89s
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

Logs are in `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8br/.scratch/fresh-continue9/repo/.scratch`: `workspace-tests-final.log`, `workspace-doc-tests.log`, `workspace-clippy.log`, `seed-build.log`, `native-capture.log`, `native-comparison.log`. The first failed full run is retained as `workspace-tests.log`; exact owner failure artifacts are preserved in that clone's `rust/target/ws8bm-diffs/`.

## Runtime observations

The app harness took 236.15 s. Four threads and full coverage are unchanged. Its slowest observed tests:

- `app::admin_two_factor_tests::self_service_limits_use_ip_and_user_windows_and_remembered_actions_share_a_bucket`: 25.090 s.
- `app::round_four_security_tests::profile_zone_case_and_alias_validation_matches_rails_without_partial_writes`: 21.454 s.
- `channels::tests::golden::replays_reference_frames`: 20.940 s.
- `app::profile_security_tests::profile_guard_fields_errors_and_security_writes_match_pinned_rails`: 14.580 s.
- `app::round_four_security_tests::settings_audit_has_only_changed_values_and_updated_account_label`: 13.917 s.

The slowest tests across the whole workspace:

- `concurrent_cutoff_drops_later_publications`: 43.252 s.
- `tests::slash_commands_test::slash_review_parser_differential_matches_rails`: 38.935 s.
- `tests::slash_commands_test::slash_callbacks_match_rails`: 37.265 s.
- `tests::slash_commands_test::slash_dispatch_and_rows_match_rails`: 37.158 s.
- `corpus_matches_rails`: 25.126 s.

The slowest owned room cases:

- `controllers::rooms::closeds_rails_cases::updating_the_icon_replaces_sidebar_rows_and_headers_for_members_only`: 2.440 s.
- `controllers::rooms::closeds_rails_cases::create_case`: 1.758 s.
- `controllers::rooms::tests::room_pages_carry_only_their_own_viewers_session_bound_values`: 1.176 s.
- `controllers::rooms::opens_rails_cases::create_case`: 1.155 s.
- `controllers::rooms::involvements_rails_cases::update_involvement_sends_turbo_update_when_becoming_visible_and_when_going_invisible`: 1.107 s.

These measurements reflect shared-host load, not a controlled speed comparison. No owned room test individually accounts for the previous 588-second app run; the named slow cases are retained, not shortened.

## Pinned Rails execution and case inventory receipts

From the canonical worktree, rerun:

```sh
python3 rust/reference-tools/rooms/check_controller_files.py
python3 rust/reference-tools/rooms/deferred_inventory.py --test-log .scratch/fresh-continue9/repo/.scratch/workspace-tests-final.log --rails-log .scratch/controller-reference9.log
```

Raw Rails source-file execution summaries (separate from Rust mappings):

```text
test/controllers/rooms_controller_test.rb
29 runs, 153 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/opens_controller_test.rb
15 runs, 55 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/closeds_controller_test.rb
12 runs, 68 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/directs_controller_test.rb
29 runs, 167 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/involvements_controller_test.rb
8 runs, 63 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/refreshes_controller_test.rb
4 runs, 24 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/reads_controller_test.rb
7 runs, 25 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/members_controller_test.rb
13 runs, 57 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/categories_controller_test.rb
5 runs, 13 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/favorites_controller_test.rb
6 runs, 21 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/inbound_email_addresses_controller_test.rb
8 runs, 26 assertions, 0 failures, 0 errors, 0 skips
test/controllers/room_categories_controller_test.rb
6 runs, 26 assertions, 0 failures, 0 errors, 0 skips
test/controllers/switchers_controller_test.rb
5 runs, 33 assertions, 0 failures, 0 errors, 0 skips
test/controllers/users/sidebars_controller_test.rb
14 runs, 76 assertions, 0 failures, 0 errors, 0 skips
WS8br Rails controller reference: 14 files passed; reference counts only
```

Raw mapping receipts:

```text
Rails case port receipts: test/controllers/rooms/inbound_email_addresses_controller_test.rb: 8 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/directs_controller_test.rb: 29 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms_controller_test.rb: 27 Rust cases passed, 2 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/opens_controller_test.rb: 15 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/closeds_controller_test.rb: 12 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/members_controller_test.rb: 13 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/refreshes_controller_test.rb: 4 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/users/sidebars_controller_test.rb: 6 Rust cases passed, 8 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/involvements_controller_test.rb: 8 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/reads_controller_test.rb: 7 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/favorites_controller_test.rb: 6 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/room_categories_controller_test.rb: 6 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/categories_controller_test.rb: 5 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/switchers_controller_test.rb: 5 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails controller reference receipts: 14 files, 161 passes, 0 failures, 0 errors, 0 skips; reference only
Rails deferred inventory: 58 files, 512 source-declared cases; 161 Rails tests run, 161 Rails reference passes; Rust mappings separate
```

Tracked mirror: `rust/plans/ws8br-wave4-report.md`. Required external report: `/home/riels/Projects/SD-Labs/Campfire/.claude/delegation/rust-port/wave4/ws8br-report.md`.
