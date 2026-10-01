# WS8bm main merge, attachment boundaries and integration — partial

Date: 2026-09-30. Worker: GPT-6.1 Sol. Branch: `rust/ws8bm-messages-http`.
Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm`.
Merged main: `27fd7892` via merge commit `b90ae426` (129 commits; #171 `76e54ad5`, WS17 #170 and deflake #169/#173). No stash, rebase, test concurrency reduction or timing threshold change.
Latest implementation/fresh-check input: **`83d4d7f1a8d8817274592a1effaf1e091f9e2e33`**. Subsequent commits contain case attribution, integration documentation/patch and this report; they do not change the worker's production code or normal tests. Current integration patch commit: `e2790e26`.

**PARTIAL.** The pending durable-edit regression is resolved by #171's merged atomic APIs. The JPEG mismatch needed additional after-commit work and is fixed. Avatar/bot/logo, filenames, cache-hit provider fetches and positive forwards are verified. Full combined live chrome/PR provider parity and end-to-end behaviour acceptance remain open. This report supersedes the prior claim that durable analysis/JPEG were still pending.

## Changed files and coherent pushed slices

Paths are relative to `rust/`.

- `b90ae426`: merge the main attachment APIs, WS17/provider changes and deflake helpers. `controllers/messages.rs`, `channel_thread_messages.rs`, `channel_threads/writes.rs` now use `Assignment::stage` and the shared `attachment_blob`/`enqueue_analysis` inside the writer. Preserve both queue invariants in `jobs/tests.rs`, both authorization/clock test-support paths and all test declarations. Locked metadata and all 13 manifests parse without duplicate workspace dependency keys.
- `1fa07b14`, `d313e498`: `messaging.rs`/`ForwarderCopier`, `active_storage.rs`, `crates/storage/src/storage.rs`, `controllers/messages.rs` and `messages/review_tests.rs` preserve the request's actual commit boundary. New variants start identified, durable variant analysis enqueues in their transaction, and nested new-variant upload failure is returned after commit with rows retained and the failed generated file removed. Existing variants reuse successfully; root processing keeps its separate committed-primary boundary. Explicit `Message#process_attachment` analysis runs even for already-analyzed originals and touches every attached owner, including a shared avatar. Durable AnalyzeJob retries retain their no-op semantics. Reference generators/vectors: `jpeg-boundary`, `thread-upload-coverage`; `initial-image-gap.py` now runs the normal passing regression.
- `8f872e7c`: `crates/kit/src/body.rs` normalizes multipart filenames the way Rack's split/last does, discarding trailing empty path pieces. `presenters/attachments/avatar_logo_tests.rs`, `avatar-logo-uploads.rb` and its vector cover 18 real authenticated CSRF requests through the merged path: user avatar, bot icon, account logo × signed/multipart × trailing path/unsafe filename/real PNG. Compare whole response bytes, raw/sanitized names, MIME, downloaded bytes, metadata and inline/durable job counts.
- `530216d4`: `check-goldens.py`/`thread-pages.json` obtain full layout bytes from the explicitly approved Rails #163 revision `2e20b24c` and cross-check **every non-layout field** unchanged against `d7c7de92`. No response mask or hand-edited fingerprint. The image's application layout, people.css and profile_card_controller.js were compared with the Git revision. All other message/thread oracles remain at the pin.
- `43ee6f3c`: `controllers/presenters.rs` retains the cached Twitter resolver while restoring Rails's actual markdown predicate, attachment fallback and forward-note composition. Collect stale Twitter/link fetch intent before cache hits; a successful HTML cache insertion followed by a rejected durable enqueue must retry on the next request. `presenters/link_embeds/tests.rs` adds a real two-sibling enqueue-rejection regression; main's equivalent Twitter regression passes as well.
- `75211148`: `crates/db/src/models/message.rs` and the Markdown presenter use the existing `campfire_storage::Filename` display conversion for attachment-only plain text. Raw DB filenames stay intact. The domain crate adds that pure data dependency through the existing workspace key; lockfile update adds one existing dependency. `signed-attachments.rb`/vector/test add four complete root/thread comparisons with unsafe path/metacharacters and nonbreaking-space Unicode names, bringing this oracle to 31 actual requests.
- `1f357c6f`, `6ec56965`: `plans/ws8bm-controller-cases.md` attributes 65 more existing-test assertion scopes in this continuation. Overall 71/156 named declarations have scoped evidence; 85 still await attribution or missing assertions. These are **not** 71 newly ported one-to-one tests.
- `42b10905`, `83d4d7f1`: `fresh-check.py` runs all workspace tests/doctests and all-target clippy with fresh generated seeds/target, eight test threads and two build jobs in pinned `campfire-toolchain` media. Container compilers claim the same host flock slots unconditionally (Docker's ancestry cannot detect Codex); this preserves the machine-wide throttle. Containers use worker names. The extra target is deleted on completion.
- `239adb47`, `e2790e26`: `plans/ws8bm-integration.md` and `owner-current-integration.patch` retain current shell/M2 integration adapters as a reviewable patch for the lead. The worker branch does not implement M2 features or modify the owner's live shell.

## Failure-first and commit-boundary evidence

All before lines below were executed in this continuation before the corresponding fix, with eight test threads/two build jobs. They are compiled/runtime failures, not compilation-only evidence.

| Regression | Before | Fixed evidence |
| --- | --- | --- |
| Durable human attachment edit, root/thread × signed/multipart | First run on the #171 merge already passed; no new durability workaround | Four actual HTTP enqueue rejections roll back all request rows/files; atomic durable API retained in each controller |
| Initial JPEG after commit | Rust 201, Rails 500; both retain one thread/message | Rails 500 with retained rows and missing new-variant file; six exact root/initial/reply responses and new/reuse lifecycle comparisons |
| Multipart avatar filename ending in slash | Successful name edit but no new avatar; regression fails | Rack basename semantics restored; all 18 avatar/bot/logo requests match |
| Cached generic-card enqueue retry | Second successful GET enqueued 0 jobs instead of 2 | Both claims and jobs roll back on failure; warm render durably enqueues both after rejection is removed |
| Signed attachment-only thread filename | JSON byte mismatch at 158: raw filename instead of sanitized display name | All 31 signed request bodies/headers/rows match, including four new filename cases |
| Merged room component mounts | Pending template 1 extra byte; list 6 extra bytes | Current owner patch mounts the owned bytes verbatim; strict checker passes 8/9 components, with only missing GitHub content remaining |

Raw before summaries:

```text
WS8bm merged JPEG: Rust 201 Created; committed messages/threads (1, 1); Rails 500 / (1, 1)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 900 filtered out; finished in 1.30s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 900 filtered out; finished in 0.77s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 904 filtered out; finished in 0.74s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 904 filtered out; finished in 3.28s
```

The previous report's blanket statement that thread-upload failures roll back was wrong and remains corrected: **pre-commit** identification/processing/queue failures roll back thread/message/membership/metadata/representation rows and staged files; **after-commit** Rails JPEG variant upload failure preserves committed rows and returns 500. No subsequent successful reuse changes that boundary. Three real HTTP representation-queue rejection cases also verify root-primary retention versus initial/reply whole-request rollback. Forward processing still uses `ForwarderCopier`'s transaction and staged file guards; its positive whole-response/row/recipient regressions pass in the fresh suite.

A first native fresh workspace run exposed three own integration failures (forward plain text, cached Twitter retry, outdated approved layout) plus the strict storage version gate: host libvips 8.18.6/FFmpeg 9.0.2 versus pinned 8.16.1/7.1.5. Fix the three own failures and run in the actual pinned environment; do not skip or weaken the gate. The final fresh run below is green, including all ten storage vector tests. No timing failure was dismissed or threshold widened.

## Stable list/composer and current owner integration

The entry points remain `Presenter::messages(&records)`, `messages::Index { ctx, messages }`, `Presenter::room_message_list(&selected_roots, divider.message_id, divider.count)`, `Composer { ctx, facts, scheduled_control }`, `FooterComposer` and `PendingTemplate { ctx, user }`. No field/signature was changed.

The merged shell must supply:

- The selected root records and unread divider message ID/count. The room owner supplies around/anchor selection, membership cursor and read effects. Mount the returned list **verbatim**, including invitation whitespace; keep viewer-specific markers outside shared fragments. Set `cache_base_url` to the verified origin and use the app fragment-cache scope.
- The live request `ViewContext`, viewer, account/layout chrome, asset resolver, verified origin, stream signer and real CSRF provider. Use `composer_facts(room, viewer, thread, drive)` for room name/kind, optional thread ID/name, built-in command names then ordered room agent commands.
- Google's actual Picker availability; `composer_drive_flow(viewer, share_picker_available)` returns Share/Metadata/None. Do not infer availability from an unconditional false once the owner provider is merged.
- M2's real `scheduled_messages::ComposerButton { ctx, room_id, thread_id }` as trusted schedule child, rendered under the same request scope. Use `FooterComposer` for a room footer and `Composer` inline/in a pane. Mount `PendingTemplate` without an added newline.
- For `Conversation`: scoped selected items, optional same-thread anchor, room updated_at, viewer view, ordered thread steps, composer facts and the real schedule child. Browsing does not join.

The current isolated integration used published shell **`6dc741c9bd42922914d619f3d87889c63e5b839d`**, which includes M2/WS11, in a private scratch clone. Resolve the six documented merge overlaps preserving both sides. Apply the tracked current patch; it wires the schedule child/assertions, keeps request fetch intent with M2's batched references, sanitizes M2's preloaded filename fallback, preserves the old main quiet-stream payload decoder despite WS11's renamed lifecycle module, and removes duplicate caller whitespace. Reverse/forward patch application was checked against the tested merge. The clone reuses only the worker's ordinary compiler cache; this is **not fresh owner-build or whole live-page signoff**. It generates its own default/first_run seeds.

17 thread tests and five native room tests pass. The separate strict checker (which includes the case the owner's test currently exempts) fails explicitly on Designers' missing GitHub PR card: Rust 287734 bytes, Rails 288823. Its diff contains only the absent populated PR article/discussion link (1,089 bytes). Fizzy, LinkedIn and generic bodies match. No mask or allowlist makes the ninth comparison green. Full app chrome/PR/system acceptance is still incomplete. The old automated `owner-integration-check.py`/`owner-schedule-integration.patch` target historical `27990da2`; refresh their merge automation for the current branch before reuse.

`render_thread_pull_request_header` remains the named WS15g seam; ordinary GET thread HTML is 200, full fixed-token templates match, but populated PR headers require the lead's WS15g provider merge. The normal worker branch still has an explicitly empty `render_thread_schedule_control` because M2's module is not on main. The current integration patch supplies its real child after the owner merge. WS12 activity/work/board remain flagged; authorized work/board show/content HTML return 501.

## Commands rerun and raw output

Executed from the worktree root unless a working directory is specified. Repeated setup/cache validation is distinguished from acceptance. Current production code was frozen before the final fresh checkout; subsequent files are documentation/patch only.

### Fresh final workspace and clippy

```sh
python3 rust/reference-tools/messaging/fresh-check.py > .scratch/fresh-filenames-final.log 2>&1
```

The helper runs locked metadata, fresh seed generation, then these actual Cargo commands in the pinned toolchain image, with `CI=1 CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=8`, shared compiler locks and its empty clone target:

```sh
cargo test --locked -j2 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast
cargo clippy --locked -j2 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
```

All 48 raw summary lines are pasted, including zero-doctest and ignored lines; total **2193 passed, 0 failed, 12 existing ignores**. All seeded app cases execute under CI. The test inputs are committed vectors plus generated seeds, not prior scratch/target state. The helper deletes the extra target; a final directory check found no `.scratch/**/target`.

```text
WS8bm fresh checkout: 83d4d7f1a8d8817274592a1effaf1e091f9e2e33; no pre-existing scratch or Cargo target; /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-fresh-s70i3gue
WS8bm fresh concurrency: eight test threads; two build jobs; no timing threshold changes
WS8bm pinned processing: campfire-toolchain; shared machine rustc flock slots
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3m 07s
test result: ok. 902 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 459.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.96s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 612 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 71.21s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.83s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.03s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.22s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.21s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.37s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.69s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.25s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.86s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.82s
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.41s
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 21s
WS8bm fresh target removed
WS8bm fresh-check: committed inputs only; generated default/first_run seeds; workspace tests/doctests/clippy passed
```

Existing ignored hooks (none added):

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test controllers::presenters::accounts::tests::manages_bots ... ignored, WS11: resetting Bender's bot key leaves the original seeded key visible in the account bot list
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

### Metadata and manifest check

```sh
cd rust
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 >/dev/null
```

Executed again after the filename dependency/lock update; exit zero. Python tomllib parsed `Cargo.toml` and every `crates/*/Cargo.toml`.

```text
WS8bm manifests: 13 parsed; zero duplicate workspace dependency keys
```

### Targeted runtime regressions

The actual invocation pattern was `CI=1 CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=8 CABLE_TEST_PORT_RANGE=52000-52049 MAIL_TEST_PORT_RANGE=52000-52049 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire --bin campfire SELECTOR -- --nocapture`. Each selector below was run in this continuation; the final fresh full suite reruns all of them.

```text
human_attachment_edits_enqueue_atomically_on_roots_and_threads:
WS8bm durable edit: 4 real HTTP enqueue failures; root/thread and signed/multipart; all request rows/files rolled back
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 900 filtered out; finished in 3.74s
controllers::messages::review_tests:::
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 894 filtered out; finished in 13.30s
avatar_bot_logo_uploads_match_pinned_rails:
WS8bm avatar/bot/logo: 18 Rails responses byte-identical; attachment bytes/filenames/MIME/metadata/job counts match
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 901 filtered out; finished in 3.04s
controllers:::
test result: ok. 323 passed; 0 failed; 1 ignored; 0 measured; 581 filtered out; finished in 72.50s
controllers::presenters:::
test result: ok. 59 passed; 0 failed; 1 ignored; 0 measured; 845 filtered out; finished in 8.48s
signed_root_and_thread_attachments_match_rails_response_and_blob_rows:
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 904 filtered out; finished in 3.55s
```

The 42-scenario/66-request broader source oracle contains 24 scalar scenarios (48 requests) and 18 initial capability/media requests, now checked by Rust tests. Separately the signed upload oracle has 31 requests, the avatar/logo oracle 18, and JPEG boundary oracle six. The 26-state owned message corpus runs cold/warm (52 whole fragments) and publishes 78 append/replace/remove socket frames. The modern reaction oracle checks 46 actual HTTP responses/rows and 44 complete reaction frames. Positive forwards check 16 subscription decisions, six private-stream refusals, 29 delivered frames/20 distinct owned frames; one captured WS12 activity callback is explicitly deferred. These counts are scoped fixture comparisons, not a claim of the full application state cross product.

### Rails goldens and controller reference

```sh
python3 rust/reference-tools/messaging/check-goldens.py > .scratch/all-goldens-filenames-final.log 2>&1
python3 rust/reference-tools/messaging/check-controller-files.py > .scratch/controller-reference-current.log 2>&1
python3 rust/reference-tools/messaging/deferred-system-inventory.py > .scratch/system-inventory-current.log 2>&1
```

These commands were rerun. Goldens use fresh DB copies and files from the generated seed; the goldens runner is serialized because it owns one scratch database. An earlier accidental concurrent invocation raced that scratch DB and was discarded; the final complete serialized invocation passes. Layout generation's initial image-argument typo was fixed before regeneration; no failed oracle output was committed.

```text
WS8bm thread layout: #163 Rails layout; every non-layout field identical to d7c7de92
WS8bm signed-attachments oracle: 31 actual root/thread requests; attach/replace/delete, expiry/purpose/missing-blob rejection, expired retries and failed-edit preservation
WS8bm avatar-logo oracle: 18 authenticated CSRF requests; avatar/bot/logo; signed/multipart; sanitized filenames and inline/durable analyzers
WS8bm JPEG boundary oracle: 6 actual Rails requests; initial/reply/root; new/reused variants; rows/files/lifecycle after commit
WS8bm golden check: 24 Rails oracles re-run; 25 golden files byte-identical
```

Per-file Rails **reference execution**, not Rust one-to-one port counts:

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

### Isolated current owner integration

Working directory: `.scratch/ws8bm-owner-merge-s7ztvppb`. The prepared merge carries the documented conflict resolutions/current patch and worker filename/model changes. Commands actually rerun there:

```sh
PARITY_NAMESPACE=ws8bm-owner PARITY_OWNER=ws8bm PARITY_CPUS=2 PARITY_IMAGE=triage-reference-d7c7de92 bash rust/parity/bin/seed build default first_run
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1 >/dev/null
```

Then, with `CI=1 CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=8`, ports 52000–52049, `CAMPFIRE_REFERENCE` set to that clone and `CARGO_TARGET_DIR` set to this worker's ordinary `rust/target`, run both Cargo selectors using `--locked -j2 -p campfire --bin campfire -- --nocapture`:

```text
controllers::channel_threads:::
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 1324 filtered out; finished in 19.65s
controllers::rooms::native_integration_tests:
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 1336 filtered out; finished in 1.30s
```

Compilation first exposed the old lifecycle namespace and a removed listener variable still present in a log line; the patch keeps the legacy payload decoder and removes the obsolete log variable. A missing Broadcast trait implementation in the first decoder attempt was corrected before tests. Those compilation failures are not counted as runtime failure-first evidence.

```sh
python3 rust/reference-tools/rooms/native_components_check.py --capture-log .scratch/native-integration-final.log
```

Exit **1**, expected unresolved provider evidence:

```text
FAIL room 654632876 message_list: Rust 287734 bytes, Rails 288823 bytes
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

## Exact remaining attribution and scope

`plans/ws8bm-controller-cases.md` lists every declaration by name and owner. Scoped evidence does not replace full merged behaviour signoff. Remaining named declarations total **85**:

| Pinned controller file | Rails runs | Scoped declarations | Still pending |
| --- | ---: | ---: | ---: |
| messages_controller_test.rb | 56 | 31 | 25 |
| messages_drive_attachments_test.rb | 19 | 0 | 19 |
| messages/cached_fragment_csrf_test.rb | 4 | 0 | 4 |
| messages/legacy_presentation_cache_test.rb | 2 | 0 | 2 |
| messages/boosts_controller_test.rb | 17 | 17 | 0 |
| channel_threads_controller_test.rb | 24 | 9 | 15 |
| channel_thread_messages_controller_test.rb | 12 | 8 | 4 |
| channel_thread_messages_drive_attachments_test.rb | 13 | 0 | 13 |
| message_forwards_controller_test.rb | 7 | 4 | 3 |
| message_forward_sources_controller_test.rb | 2 | 2 | 0 |

Next work in user-impact order:

1. Lead merge of the current room/M2 branches with the reviewed integration adapters; merge/wire WS15g's populated message PR cards and `render_thread_pull_request_header`, then make the ninth strict component and complete live layout/chrome parity green. Refresh the historical automated owner merge checker. The ordinary thread page and real schedule component/pane checks already pass in the prepared merge; the normal branch's schedule seam remains empty until that merge.
2. Finish broader upload/state/recipient cross products beyond the listed exact fixture matrices (including populated PR/thread headers and combined feature states), retaining the now-correct atomic/after-commit boundaries. WS15g/WS15e own provider internals; WS8bm owns their message integration. M2 owns polls/pins/saves/schedules/search/slash/autocomplete/links/files; do not implement those here.
3. Attribute/implement missing assertions for the 85 named declarations above, with the per-declaration owners in the inventory. Work/board controller cases remain WS12/WS11 seams; bot/agent controller semantics remain WS11-owned.
4. End-to-end **behaviour** execution for the 19 inventoried system files; 156 literal declarations, zero executed here. Shared shell/Google/keyboard/a11y interactions require the merged app. No screenshots or pixel-diff work is queued.
5. WS12 activity frame and authorized work/board HTML 501 seams remain explicitly deferred to WS12; do not treat them as implemented.

System inventory raw lines:

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

No production deployment, PR creation or outbound messages were performed. No open permission question. All launched test/build processes must be finished at handoff; extra scratch targets are removed. Scratch logs are evidence outputs only, never normal-test inputs.
