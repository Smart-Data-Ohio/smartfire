# WS8bm GitHub integration, Drive coverage and live chrome — partial

Date: 2026-10-01. Worker: GPT-6.1 Sol. Branch: `rust/ws8bm-messages-http`.
Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm`.
Pinned Rails: `d7c7de92`, with only the approved #163 layout drift at `2e20b24c` used by the thread full-layout oracle.
Merged `origin/main` at `65ad0d39` with merge commit **`e32d20ab`**. This incorporates #167 plus the earlier #171/#170/#173 changes. No stash, rebase, timing threshold change or concurrency reduction.
Latest implementation and fresh-clone input: **`4e1e40e82af395af119202d2cc19cdee4c674896`**. The following report-only commit changes no production code or normal tests.

**PARTIAL.** The missing 1,089-byte GitHub card now uses WS15g's real rendering and matches the complete owned card container on cold/warm lists. The live thread show also uses WS15g's authorized private-safe PR header and lazy write frame. Thirty root/thread Drive request cases, two additional member-role refusals, 93 guarded socket frames and four live icon/recent-search components are checked against Rails. Named controller attribution advances from 71/156 to 103/156: 53 declarations remain pending. Full merged owner-shell/schedule acceptance remains open; this report does not convert the earlier eight-of-nine result into a new nine-of-nine pass.

## Coherent pushed slices and changed files

Controller/presenter paths below are beneath `rust/crates/campfire/src/`; domain and view crate paths are given separately.

- `e32d20ab`: merge main's GitHub/WS11 integrations, preserving both sides in `channels/sink.rs`, `controllers/messages.rs`, `presenters/message_item.rs`, `presenters/page.rs`, `crates/views/src/lib.rs` and `crates/db/src/models/message.rs`. Retain main's typed agent replay/budget `PostingOutcome` and GitHub callbacks, the worker's whole-request thread processing, separate root after-commit boundary, full aggregate cache key and pre-cache provider fetch intent. Locked metadata passes; all 13 manifests parse without duplicate keys.
- `4955cfb0`: `controllers/channel_threads.rs`, `presenters/github.rs` and the shared `messages::present` path wire the clearly named `render_thread_pull_request_header` to WS15g's real `thread_header`. The PR/thread/room mapping is scoped, room access is checked first, and stale refresh intent is enqueued after releasing the read. New `messages/github_integration_tests.rs` and `channel_threads/github_tests.rs` compare complete owned card containers and actual public/private/unknown thread GET bodies, including private-safe suppression and non-member refusal. New `room-components.rb`, `github-thread-page.rb` and vectors come from Rails. The merged main presenter already supplies `github::message_cards`; no copied card markup or room-shell edit was needed.
- `98323f92`: new `messages/drive_tests.rs`, `reference-tools/messaging/drive-controllers.rb` and vector compare 30 actual root/thread HTTP responses, saved order/source/IDs and message/attachment deltas. Additional non-admin refusals retain rows. Complete display/edit components, actual live generic chips, removable chips, blank sentinel and Google-consent independence are checked. `channels/tests/hub_test/message_parity.rs` compares all 93 Rails frames through the real WS7 publisher, guard and sockets, with silence after omitted fields/refusals. `check-owned-mutations.py` proves provider omission and author-policy bypass are rejected. `plans/ws8bm-controller-cases.md` attributes all 19 root and 13 thread Drive declarations to these explicit assertion scopes.
- `8f38a5af`: `controllers/presenters/rich_text.rs` and `presenters/page.rs` populate the shared live layout from the existing ordered brand/alias registry, name-ordered workspace icons and current-user search domain, limited to the latest ten. `channel_threads/chrome_tests.rs`, `live-chrome.rb` and vector compare the entire icon meta value and recent-search child through real thread GETs for two users, before/after custom icons. Escaped text/URLs, ordering and viewer isolation are checked. The Clear form retains its real request-local CSRF token. No M2 feature endpoint was implemented.
- `4e1e40e8`: `messages/rendered.rs` takes stale GitHub refresh intent from the presenter on root/thread edits, then calls WS15g after releasing the reader. The existing `github/card_tests.rs` caller test keeps its twelve concurrent GET/refresh requests and original bodyless legacy edit; a new pinned Rails oracle adds unchanged rich-text root/thread edits and verifies whole responses, saved bodies, ignored attributes, edited state, durable fetch counts and claims. Both edit paths retain successful responses on queue rejection and roll back the refresh claim. No expectation or original assertion was weakened.

The prior attachment fixes remain intact after this main merge. The full fresh suite reruns the durable root/thread edit rollback, missing-image closed-thread rollback, signed initial attachment, scalar client-ID retry, JPEG after-commit and avatar/bot/logo regressions. The merged code preserves #171's atomic attachment/analysis APIs and Rails's distinct after-commit failure behavior. The earlier blanket claim that all thread-upload failures roll back remains withdrawn: pre-commit failures roll back; Rails JPEG variant upload failure after commit returns 500 with committed rows retained.

## Failing-first and negative-control evidence

The actual thread-page regression failed before wiring WS15g's header. The icon/history regression failed on missing live icon metadata before populating the shared layout. These are runtime failures; initial oracle setup/compile errors are not counted. Main already implements the card provider, so its omission is a deliberate negative control, not a claim of a newly discovered main defect. The Drive author mutation is also a negative control of already-correct authorization.

```text
PR private=Some(false): byte 482; actual 551 bytes, Rails 2280 bytes
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1014 filtered out; finished in 1.53s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1014 filtered out; finished in 1.64s

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1017 filtered out; finished in 1.32s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1017 filtered out; finished in 1.33s

room 654632876, message 935962053, warm=false: byte 95; actual 115 bytes, Rails 1204 bytes
github-card-omitted: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1015 filtered out; finished in 1.35s
drive-author-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1016 filtered out; finished in 7.19s
WS8bm owned mutations: 2 rejected; 0 survived; production files restored
```

The omitted card has exactly the original 1,089-byte deficit. After restoring production, the complete-container regression passes:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1014 filtered out; finished in 1.32s
```

Targeted selectors below were executed in this continuation with eight test threads/two build jobs and generated seed inputs. This exact edit-refresh command was rerun before and after the fix; the final fresh full suite reruns every selector:

```sh
CI=1 CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=8 CABLE_TEST_PORT_RANGE=52000-52049 MAIL_TEST_PORT_RANGE=52000-52049 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire review_refreshes_use_real_message_broadcast_and_refresh_callers -- --nocapture
python3 rust/reference-tools/messaging/check-owned-mutations.py
```

Selectors: `controllers::messages::review_tests`, `github_thread_show_matches_complete_rails_public_private_and_unknown_bodies`, `complete_github_containers`, `drive_`, `live_thread_chrome_matches_rails_icons_and_viewer_scoped_latest_ten_searches`, and `review_refreshes_use_real_message_broadcast_and_refresh_callers`. The initial fresh suite at `8f38a5af` found the missing edit-refresh enqueue in the merged WS15g caller test. The expanded regression failed before the fix with `bodyless_root/claim`: Rust false, Rails true. Rails confirms that a bodyless legacy edit is valid (302); its original fixture and status expectation remain. The fix collects refresh intent from the rendered edit presenter and enqueues outside the read. Raw failing-first and fixed lines:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1017 filtered out; finished in 1.99s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1017 filtered out; finished in 1.80s
```

Raw review and final Drive summaries:

```text
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 1003 filtered out; finished in 29.21s
WS8bm Drive writes: 30 complete Rails response/row comparisons; rejected creates and edits leave message and attachment counts unchanged
WS8bm Drive broadcasts: 93 complete Rails frames through WS7 publisher/guard/socket; 30 root/thread requests; no extra frames
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 1009 filtered out; finished in 21.67s
```

The pre-fix fresh app summary was:

```text
test result: FAILED. 1015 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out; finished in 1101.42s
```

Only `controllers::github::card_tests::review_refreshes_use_real_message_broadcast_and_refresh_callers` failed (0 refresh jobs versus Rails 1). No timing failure was waived. The final run below starts from another fresh clone containing the pushed refresh fix.

## Fresh-clone workspace verification

```sh
python3 rust/reference-tools/messaging/fresh-check.py >.scratch/fresh-current.log 2>&1
```

The committed helper creates a new clone of the branch, asserts no pre-existing `.scratch` or `rust/target`, generates default/first-run seeds, and runs locked metadata, the entire workspace test/doctest suite (excluding vendored html5ever) and all-target workspace clippy with `-D warnings`. It uses the pinned `campfire-toolchain` media environment, eight test threads/two build jobs and the same machine-wide rustc flock slots. It does not depend on the worker's seed, scratch files or existing target. The extra fresh target is removed after all available checks. No pixel check is part of acceptance.

Computed across the 48 raw test summaries: **2,355 passed, zero failed, 11 existing ignored hooks**. Seeded app: **1,016 passed, zero failed, two existing ignored hooks**. All-target clippy exits zero with `-D warnings`.

Raw summary lines from this fresh invocation:

```text
WS8bm fresh checkout: 4e1e40e82af395af119202d2cc19cdee4c674896; no pre-existing scratch or Cargo target; /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-fresh-giwjp_y8
WS8bm fresh concurrency: eight test threads; two build jobs; no timing threshold changes
WS8bm pinned processing: campfire-toolchain; shared machine rustc flock slots
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4m 31s
test result: ok. 1016 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 1145.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.66s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.26s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 660 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 92.54s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.54s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.96s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 7.59s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.57s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.37s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.88s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.78s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.29s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.89s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.52s
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.72s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.84s
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 31s
WS8bm fresh target removed
WS8bm fresh-check: committed inputs only; generated default/first_run seeds; workspace tests/doctests/clippy passed
```

Existing ignored hooks are listed below from the fresh run; none was added in this continuation:

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
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

## Rails goldens, grouped controller reference and manifests

These commands were rerun in this continuation. The golden runner uses fresh database copies and seed storage; it is serialized because it owns one reference scratch database. Committed normal tests read tracked vectors and independently generated seeds, never these scratch outputs.

```sh
python3 rust/reference-tools/messaging/check-goldens.py >.scratch/goldens-final.log 2>&1
python3 rust/reference-tools/messaging/check-controller-files.py >.scratch/controller-files-current.log 2>&1
python3 rust/reference-tools/messaging/deferred-system-inventory.py >.scratch/system-inventory-current.log 2>&1
CARGO_BUILD_JOBS=2 mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1 >/dev/null
```

Metadata exits zero. Python `tomllib` parsed `rust/Cargo.toml` and every `rust/crates/*/Cargo.toml`; its raw line is:

```text
TOML duplicate-key check: 13 manifests parsed; no duplicate keys
```

Raw golden summary lines:

```text
WS8bm preview oracle: 8 real Rails HTTP responses; 0 messages written
WS8bm invalid-create oracle: 6 real Rails HTTP responses; 0 messages written
WS8bm scalar-cast oracle: 8 actual Rails model assignments
WS8bm fragment oracle: 5 real Rails messages; 2 viewers through one fragment cache; 0 session-bound values
WS8bm root oracle: 10 real Rails actions responses; 6 updates and saved rows; 2 rejected updates; 9 edit forms and 1 actions menu; 4 standalone messages
WS8bm paging oracle: 16 real Rails page requests; 12 format requests; template digest 8c84e9c3391ab09f136a472ad8ab8b69
WS8bm broadcast oracle: 6 real Rails writes; 25 rendered/channel publisher frames; request port 3443
WS8bm thread-membership oracle: 11 real Rails requests; membership rows and JSON bytes captured
WS8bm collection oracle: 10 real Rails states; keys and cache-hit bytes; 0 session-bound values
WS8bm room-list oracle: 9 real Rails room requests; selected roots/unread facts and show list-slot bytes; 0 session-bound values
WS8bm message-states oracle: 26 real Rails states rendered cold/warm; 78 actual append/replace/remove frames; 0 session-bound values
WS8bm thread-message read oracle: 18 actual Rails requests; scoped pages, empty formats, raw JSON/actions/HTML and locked reads
WS8bm thread-message write oracle: 15 actual Rails writes; 50 publisher frames; retries, rows, Drive sets, locks and tombstones
WS8bm thread-pages oracle: 28 actual Rails requests; state lists, standalone HTML/JSON, latest replies and deleted starter
WS8bm thread layout: #163 Rails layout; every non-layout field identical to d7c7de92
WS8bm thread-lifecycle oracle: 27 actual Rails actions; creation retries, metadata/tags, lifecycle permissions, rollback rows and delete frames
WS8bm thread-content oracle: 9 actual requests; anchor scope and fixed-secret conversation/composer bytes
WS8bm forwards oracle: 18 real picker/refusal/source-privacy requests; exact JSON bytes
WS8bm forward-success oracle: 5 positive actual requests; 7 forwards; complete bodies/rows and 21 rendered frames
WS8bm modern-boosts oracle: 46 actual toggle/alias/legacy/duplicate/delete/coercion requests; 44 rendered reaction replacements
WS8bm signed-attachments oracle: 31 actual root/thread requests; attach/replace/delete, expiry/purpose/missing-blob rejection, expired retries and failed-edit preservation
WS8bm boost-pages oracle: 7 actual index/new/actions requests; complete fixed-token forms, distinct reactors and hostile tooltip names
WS8bm thread-review oracle: 4 actual Rails requests; boolean retry, failed closed-thread media rollback, signed initial attachment
WS8bm thread-upload-coverage oracle: 42 scenarios; 66 actual Rails requests; 3 client-id paths, 4 media types, top/nested initial capabilities and rollback
WS8bm client-retries oracle: 24 scenarios; 48 actual Rails requests; root/reply/initial scalar IDs and raw blank-value semantics
WS8bm avatar-logo oracle: 18 authenticated CSRF requests; avatar/bot/logo; signed/multipart; sanitized filenames and inline/durable analyzers
WS8bm JPEG boundary oracle: 6 actual Rails requests; initial/reply/root; new/reused variants; rows/files/lifecycle after commit
WS8bm room components: 3 complete list/composer/template goldens; reference d7c7de92
WS8bm GitHub thread page: 3 complete public/private/unknown show bodies; reference d7c7de92
WS8bm Drive controllers: 30 actual Rails writes; complete responses, rows and frames; root/thread
WS8bm live chrome: 4 icon/recent-search components; ordered custom icons, scoped latest ten, escaped HTML/URLs; reference d7c7de92
WS8bm GitHub edit refresh: 3 actual Rails requests; bodyless legacy and unchanged root/thread refresh claims
WS8bm golden check: 29 Rails oracles re-run; 30 golden files byte-identical
```

Per-file Rails reference execution, **not** Rust one-to-one port counts:

```text
test/controllers/messages_controller_test.rb
56 runs, 265 assertions, 0 failures, 0 errors, 0 skips
test/controllers/messages_drive_attachments_test.rb
19 runs, 83 assertions, 0 failures, 0 errors, 0 skips
test/controllers/messages/cached_fragment_csrf_test.rb
4 runs, 58 assertions, 0 failures, 0 errors, 0 skips
test/controllers/messages/legacy_presentation_cache_test.rb
2 runs, 13 assertions, 0 failures, 0 errors, 0 skips
test/controllers/messages/boosts_controller_test.rb
17 runs, 155 assertions, 0 failures, 0 errors, 0 skips
test/controllers/channel_threads_controller_test.rb
24 runs, 206 assertions, 0 failures, 0 errors, 0 skips
test/controllers/channel_thread_messages_controller_test.rb
12 runs, 65 assertions, 0 failures, 0 errors, 0 skips
test/controllers/channel_thread_messages_drive_attachments_test.rb
13 runs, 57 assertions, 0 failures, 0 errors, 0 skips
test/controllers/message_forwards_controller_test.rb
7 runs, 35 assertions, 0 failures, 0 errors, 0 skips
test/controllers/message_forward_sources_controller_test.rb
2 runs, 12 assertions, 0 failures, 0 errors, 0 skips
WS8bm Rails controller reference: 10 files passed; reference counts only
```

## Stable room-shell seams and owner integration

Detailed caller contract: `plans/ws8bm-integration.md`. Stable entry points are unchanged:
`Presenter::messages(&records)`, `messages::Index { ctx, messages }`,
`Presenter::room_message_list(&selected_roots, divider.message_id, divider.count)`,
`Composer { ctx, facts, scheduled_control }`, `FooterComposer`, and
`PendingTemplate { ctx, user }`.

The merged shell must pass:

- Selected root records plus unread divider message ID/count; room-owned around/anchor selection and membership/read effects. Mount returned list bytes verbatim, including invitation whitespace; do not prepend that line. Render under the app fragment-cache scope with verified `cache_base_url`.
- Live `ViewContext`, current viewer/account, assets, verified request origin, stream signer and real request CSRF provider. `composer_facts(room, viewer, thread, drive)` supplies room name/kind, optional thread ID/name, built-in commands followed by ordered room agent commands.
- Actual Google Picker availability. `composer_drive_flow(viewer, share_picker_available)` supplies Share/Metadata/None; consent alone does not imply enhanced Picker availability.
- M2's real `scheduled_messages::ComposerButton { ctx, room_id, thread_id }` as trusted schedule child in the same request scope. Use `FooterComposer` for the room footer and `Composer` for the inline pane. The named `render_thread_schedule_control` remains explicitly empty here until that provider is merged.
- `PendingTemplate` with the viewer's `UserView`, mounted without an extra newline. `Conversation` additionally requires scoped selected items/anchor, room `updated_at`, viewer view, ordered thread steps, composer facts and schedule child.
- The new shared `Layout::load` supplies actual icon names and scoped search history. Other owner chrome/provider facts still need complete merged-page acceptance.

The PR call site is now live: `render_thread_pull_request_header` calls `Presenter::github_thread_header`, which delegates to WS15g's real private-safe adapter and retains the lazy write frame. The list receives WS15g's actual card through the existing renderer. No owned caller signature was changed; WS8b-r can consume both without template copies.

The previous isolated owner check on shell `6dc741c9bd42922914d619f3d87889c63e5b839d` was 8/9, with only the GitHub card missing. That is historical evidence, not a new full-component pass in this continuation. The complete GitHub slot is now independently verified from actual Rails room responses on cold/warm lists, including the precise omission negative control. A full normal-branch list comparison also exposed the unmerged M2 message-link placeholder (157 bytes); it was not fixed by copying another owner's feature. Current owner merge acceptance remains required. An attempted current-main merge in the isolated historical owner checkout produced 24 overlaps and was aborted, preserving its checkpoint; no partial conflict resolution is represented as acceptance. Refresh the historical owner checker/patch for the current revisions before running it. This worker does not modify the owner's room-shell production files.

## Exact remaining attribution and scope

`plans/ws8bm-controller-cases.md` lists every declaration by name and owner. Scoped evidence is not full merged behavior signoff. **53 named declarations remain pending**:

| Pinned controller file | Rails runs | Scoped declarations | Pending |
| --- | ---: | ---: | ---: |
| messages_controller_test.rb | 56 | 31 | 25 |
| messages_drive_attachments_test.rb | 19 | 19 | 0 |
| messages/cached_fragment_csrf_test.rb | 4 | 0 | 4 |
| messages/legacy_presentation_cache_test.rb | 2 | 0 | 2 |
| messages/boosts_controller_test.rb | 17 | 17 | 0 |
| channel_threads_controller_test.rb | 24 | 9 | 15 |
| channel_thread_messages_controller_test.rb | 12 | 8 | 4 |
| channel_thread_messages_drive_attachments_test.rb | 13 | 13 | 0 |
| message_forwards_controller_test.rb | 7 | 4 | 3 |
| message_forward_sources_controller_test.rb | 2 | 2 | 0 |

Remaining work, in user-impact order:

1. Merge current room/M2 owners with both sides' behavior; wire the real schedule provider, refresh the historical integration adapters, and rerun all nine strict native room components and complete live chrome/provider pages. The owned 1,089-byte GitHub gap is closed, but a whole combined-page or nine-of-nine owner pass is not claimed.
2. Complete the named missing root/thread provider-edit/recipient/cache/edited-state assertions, plus wider combined message-state/upload/recipient coverage after owner merge. The existing 26-state cold/warm corpus, upload/forward/boost matrices and new Drive matrix are specific verified fixture scopes, not the full application cross product. WS15g/WS15e own provider internals; this worker owns their message integration. M2 owns polls/pins/saves/schedules/search/slash/autocomplete/links/files and is not implemented here.
3. Finish attribution or missing assertions for the exact 53 declarations above. Six root bot/agent HTTP cases depend on WS11 behavior already merged on main; do not treat domain availability as completed controller attribution. Ten work-tracking thread cases remain WS12/WS11-owned HTTP seams. Constant-query-cost and remaining cache/token tests need direct case-level evidence.
4. End-to-end behavior acceptance for the 19 inventoried system files (156 literal declarations, zero executed here), once the merged app is ready. No screenshot/pixel phase remains.
5. Preserve WS12's flagged activity callback and authorized work/board HTML show/content 501 seams; they remain deliberately unimplemented by this worker.

Raw deferred system inventory:

```text
test/system/boosting_messages_test.rb: 4 literal test declarations; 0 executed; deferred
test/system/code_highlighting_test.rb: 6 literal test declarations; 0 executed; deferred
test/system/sending_messages_test.rb: 3 literal test declarations; 0 executed; deferred
test/system/threads_test.rb: 15 literal test declarations; 0 executed; deferred
test/system/workspace_markdown_test.rb: 8 literal test declarations; 0 executed; deferred
test/system/composer_test.rb: 11 literal test declarations; 0 executed; deferred
test/system/composer_attach_menu_test.rb: 9 literal test declarations; 0 executed; deferred
test/system/message_interactions_test.rb: 10 literal test declarations; 0 executed; deferred
test/system/message_actions_mobile_test.rb: 2 literal test declarations; 0 executed; deferred
test/system/message_toolbar_test.rb: 13 literal test declarations; 0 executed; deferred
test/system/message_list_a11y_test.rb: 29 literal test declarations; 0 executed; deferred
test/system/drive_attachments_test.rb: 3 literal test declarations; 0 executed; deferred
test/system/unread_divider_test.rb: 5 literal test declarations; 0 executed; deferred
test/system/search_forward_edit_test.rb: 3 literal test declarations; 0 executed; deferred
test/system/keyboard_shortcuts_test.rb: 14 literal test declarations; 0 executed; deferred
test/system/content_security_policy_test.rb: 5 literal test declarations; 0 executed; deferred
test/system/motion_test.rb: 9 literal test declarations; 0 executed; deferred
test/system/mobile_layout_test.rb: 5 literal test declarations; 0 executed; deferred
test/system/timezone_detection_test.rb: 2 literal test declarations; 0 executed; deferred
WS8bm system inventory: 19 pinned files; declarations only; no browser or pixel pass claim
```

No deploy, PR creation or outbound message was performed. Both fresh-run targets were removed by the helper; a final recursive scratch inspection found zero Cargo targets. All launched test/build commands completed before handoff; the ordinary worktree target is the allowed cache. Scratch logs are outputs only, never normal-test inputs.
