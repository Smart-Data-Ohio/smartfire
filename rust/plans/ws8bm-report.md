# WS8bm messaging HTTP — partial continuation

Date: 2026-09-30. Worker: GPT-6.1 Sol. Branch: `rust/ws8bm-messages-http`.
Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm`.
Rails reference: `d7c7de92`. Main merged: `21a7332f2d3c324f0862cdf448baf17a84395aa0`.
Merge commit: `1caa4c6348e55071ac2d48cc077c2df5e6a9c37c`.
Pushed implementation: `5d2f248f79da40b98f22482ad3f775149abc4613`. This report is committed afterward.

**PARTIAL.** The merge and a coherent root edit/update/actions slice are pushed. The earlier authorization/preview and create/Markdown-fragment slices remain included (`0fa03758`, `462a7595`). Step 1 is still partial; steps 2–7 have not been completed. The final full seeded app binary has zero failures. Full-workspace/all-targets clippy is clean. No cutover or PR was attempted.

## Changes by file

- `rust/crates/campfire/src/controllers.rs`: registers `messages#actions` in the Rails-derived router.
- `rust/crates/campfire/src/controllers/messages.rs`: human updates resolve root replies, assign source/body/client id/reply flags and Drive sets, call WS8a's `Message::edit`, and return the modern payload. Missing/non-null source handling clears stale Markdown mode on legacy updates. Rejected updates return Rails' 422 shape. Legacy-to-Markdown edits preserve non-mention attachments from rendered Action Text inside the same writer transaction. The edit action uses the editable Markdown source. Actions use the alive-member/root scope and default authentication/CSRF chain and set `Cache-Control: no-store`. Update/actions and preview encode JSON as the actual Rails HTTP responses do, preserving raw HTML entities. Existing attachment staging/analysis and the old presentation-only replace broadcast remain; they are not claimed as complete upload/broadcast ports.
- `rust/crates/campfire/src/controllers/messages/payload.rs`: request-specific message/actions data, source and copy text, creator/room facts, reply/deleted/forwarded/Drive/streaming fields, canonical links, distinct reaction counts/current-viewer flags, pins and saved-item state, and normal thread summaries. No shared fragment cache stores these values. Field order and escaping are verified against raw Rails response bodies. Nonempty work-owner/agent/card states are implemented from the reference but unproven in this slice.
- `rust/crates/campfire/src/controllers/presenters.rs`: fallible rendered Action Text and `Message#editable_markdown_source` adapters. Rails calls `LegacyMarkdown` with `Content#to_s`; the standalone WS5 vectors use `Content#to_html`. Rendering first reproduces the actual app's concatenation and attachment-attribute filtering. The older bot JSON helper's render-error fallback is retained.
- `rust/crates/campfire/src/controllers/presenters/rich_text.rs`: connects WS1's existing `embed_image` signer and segment escaping to the WS5 resolver. The legacy unfurl's complete rendered body, including its signed image URL, matches Rails bytes. No proxy HTTP/fetch implementation was added.
- `rust/crates/db/src/models/message.rs`: small WS8a seams in `MessageChanges`: explicit source clearing, a rendered non-mention snapshot, nullable client/reply id assignments and the reply notification flag. Existing validation, `edited_at`, references, index and durable queue callbacks remain in the writer transaction. Defaults preserve existing model callers' assignment semantics. No schema or domain redesign.
- `rust/crates/views/src/messages.rs` and `templates/messages/edit.html`: the Markdown textarea and Drive chips/sentinel/save control replace the stock edit form. The legacy `EditView.editable_body_html` field name is retained, with its source meaning documented. `ActionsMenu` exposes the existing shared per-page menu for byte comparison; the menu template itself needed no change. The message-list entry point is documented without changing its fields.
- `rust/crates/campfire/src/controllers/presenters/test_support.rs`: caller-owned frozen-clock boot for exact row/JSON timestamp differentials; ordinary tests retain the ticking seed clock and CI seed enforcement.
- `rust/crates/campfire/src/controllers/messages/root_tests.rs`: eight mandatory-seed tests. Ten real actions responses are compared as two authenticated members; six PATCH responses and saved rows, two rejected updates, nine detached edit forms, three rendered Action Text bodies and one detached menu are checked. They also cover root/thread scope, removed/non-members/deleted room, update CSRF, valid bot keys denied even for a bot-owned message, and text/Drive rollback on queue insertion failure.
- `rust/crates/campfire/src/controllers/messages/http_tests.rs`: preview now compares raw HTTP JSON bytes in addition to its parsed shape; the existing 15 tests still execute.
- `rust/crates/campfire/src/controllers/messages/tests.rs`: replaces two obsolete upstream assumptions (Lexxy editor and JSON update 500) with the pinned Rails Markdown form and successful payload assertions. The full-suite failure exposed these assumptions; it was not labeled inherited or ignored.
- `rust/reference-tools/messaging/{root.rb,preview.rb}` and `rust/vectors/messaging/{root.json,preview.json}`: real Rails/default-seed request, row, raw JSON and render oracles. No Rust output generated expected values. The root generator uses Rails runner's `--skip-executor` so integration requests own their execution context.
- `rust/reference-tools/messaging/{root-discriminate.py,discriminate.py}`: 14 new and 17 cumulative compiled regressions. The older create mutation anchor now includes its trailing comma so it cannot match `clear_markdown_source`. Each requires the intended named runtime test to fail and restores source in `finally`; compiler/setup failures do not count.
- `rust/reference-tools/messaging/reference-check.py`: checks 19 exact pinned source files and rejects injected source-byte/file-set drift.

## Stable entry point for WS8b-r

Continue rendering `campfire_views::messages::Index { ctx, messages: &[MessageItem] }` from the room shell. Build the items with `Presenter::messages(&records)`. `messages/index.html` calls `campfire_views::messages::cached_message_item`; uncached single-message rendering remains `campfire_views::messages::message(ctx, &MessageView)`. No room-shell/sidebar file or entry-point field changed.

Around selection belongs at the room-shell boundary: WS8a exposes `Message::page_around(conn, Timeline::Room(room_id), &anchor)`. Our Rails root `MessagesController#index` only selects `before`, `after`, or the last page; it does not select an `around` parameter. Room-shell around/last-read/unread behavior still needs coordination and differential coverage with WS8b-r.

## Verification and raw summaries

All commands below ran in this worktree this continuation. The final source tests ran after mutation sources were restored. Scratch directories/storage were copied from the freshly rebuilt seeds. Cargo used Rust 1.98.1, locked resolution, four jobs, the worker's target and no release profile. Docker names use `ws8bm-`; socket tests use 52000–52049. Report commands are exact; output blocks contain raw summary lines.

### Reference identity

```sh
python3 rust/reference-tools/messaging/reference-check.py > .scratch/reference-check-final.log 2>&1
```
```text
WS8bm reference source check: 19 controller, model, helper, template and icon files match d7c7de92
WS8bm reference check self-test: 2 injected source-byte/file-set differences rejected
```

### Locked metadata and duplicate keys

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CARGO_BUILD_JOBS=4 mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1 >/dev/null
python3 - <<'PY' > .scratch/workspace-keys-final.log
from pathlib import Path
import tomllib
root=tomllib.loads(Path('rust/Cargo.toml').read_text())
for manifest in Path('rust/crates').glob('*/Cargo.toml'):
    tomllib.loads(manifest.read_text())
print(f"WS8bm workspace dependency check: {len(root['workspace']['dependencies'])} unique keys; all crate manifests parsed; 0 duplicate TOML keys")
PY
```
```text
WS8bm workspace dependency check: 75 unique keys; all crate manifests parsed; 0 duplicate TOML keys
```

Metadata returned 0 and emitted no stdout. Parsing TOML rejects duplicate keys; the check covers the workspace and every crate manifest. The merge did not change the lockfile.

### Fresh seeds

```sh
PARITY_NAMESPACE=ws8bm PARITY_OWNER=ws8bm PARITY_CPUS=2 PARITY_IMAGE=triage-reference-d7c7de92 TMPDIR="$PWD/.scratch" bash rust/parity/bin/seed build default first_run > .scratch/seeds-final.log 2>&1
```
```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

### Regenerate root oracle and compare bytes

```sh
cp rust/parity/.seed/default/db/production.sqlite3 .scratch/root-reference/db/production.sqlite3 && docker run --rm --cpus 2 --name ws8bm-root-oracle-final --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_LOG_LEVEL=warn -e PARITY_REDIS=1 -v "$PWD/.scratch/root-reference/db:/rails/storage/db" -v "$PWD/.scratch/root-reference/storage:/rails/storage/files" -v "$PWD/.scratch/final-goldens:/out" -v "$PWD/rust:/work:ro" triage-reference-d7c7de92 bash -c 'bin/rails runner --skip-executor /work/reference-tools/messaging/root.rb /out/root.json' > .scratch/root-oracle-final.log 2>&1 && cmp .scratch/final-goldens/root.json rust/vectors/messaging/root.json
```
```text
WS8bm root oracle: 10 real Rails actions responses; 6 updates and saved rows; 2 rejected updates; 9 edit forms and 1 actions menu
```

Runner and `cmp` returned 0. `cmp` emitted no output. The detached menu/edit renders use normal Rails rendering with no token-helper override; request-bound token presence/CSRF behavior is checked separately.

### Regenerate preview oracle and compare bytes

```sh
cp rust/parity/.seed/default/db/production.sqlite3 .scratch/preview-reference/db/production.sqlite3 && docker run --rm --cpus 2 --name ws8bm-preview-oracle-final --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_LOG_LEVEL=warn -e PARITY_REDIS=1 -v "$PWD/.scratch/preview-reference/db:/rails/storage/db" -v "$PWD/.scratch/preview-reference/storage:/rails/storage/files" -v "$PWD/.scratch/final-goldens:/out" -v "$PWD/rust:/work:ro" triage-reference-d7c7de92 bash -c 'bin/rails runner --skip-executor /work/reference-tools/messaging/preview.rb /out/preview.json' > .scratch/preview-oracle-final.log 2>&1 && cmp .scratch/final-goldens/preview.json rust/vectors/messaging/preview.json
```
```text
WS8bm preview oracle: 8 real Rails HTTP responses; 0 messages written
WS8bm invalid-create oracle: 6 real Rails HTTP responses; 0 messages written
WS8bm scalar-cast oracle: 8 actual Rails model assignments
```

Runner and `cmp` returned 0. `cmp` emitted no output. The detached menu/edit renders use normal Rails rendering with no token-helper override; request-bound token presence/CSRF behavior is checked separately.

### Regenerate fragments oracle and compare bytes

```sh
cp rust/parity/.seed/default/db/production.sqlite3 .scratch/fragments-reference/db/production.sqlite3 && docker run --rm --cpus 2 --name ws8bm-fragments-oracle-final --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_LOG_LEVEL=warn -e PARITY_REDIS=1 -v "$PWD/.scratch/fragments-reference/db:/rails/storage/db" -v "$PWD/.scratch/fragments-reference/storage:/rails/storage/files" -v "$PWD/.scratch/final-goldens:/out" -v "$PWD/rust:/work:ro" triage-reference-d7c7de92 bash -c 'bin/rails runner --skip-executor /work/reference-tools/messaging/fragments.rb /out/fragments.json' > .scratch/fragments-oracle-final.log 2>&1 && cmp .scratch/final-goldens/fragments.json rust/vectors/messaging/fragments.json
```
```text
WS8bm fragment oracle: 5 real Rails messages; 2 viewers through one fragment cache; 0 session-bound values
```

Runner and `cmp` returned 0. `cmp` emitted no output. The detached menu/edit renders use normal Rails rendering with no token-helper override; request-bound token presence/CSRF behavior is checked separately.

### Pinned Rails controller reference validation

```sh
python3 - <<'PY'
from pathlib import Path
import subprocess
paths=['test/controllers/messages_controller_test.rb','test/controllers/messages_drive_attachments_test.rb','test/controllers/messages/cached_fragment_csrf_test.rb','test/controllers/messages/legacy_presentation_cache_test.rb']
for path in paths:
    assert Path(path).read_bytes() == subprocess.check_output(['git','show',f'd7c7de92:{path}']),path
print('WS8bm Rails controller test source check: 4 mounted files match d7c7de92')
PY
docker run --rm --cpus 2 --name ws8bm-rails-controllers --entrypoint sh --env-file rust/parity/.env.reference -e RAILS_ENV=test -e PARALLEL_WORKERS=1 -e RAILS_LOG_LEVEL=warn -v "$PWD/test:/rails/test:ro" triage-reference-d7c7de92 -ec 'redis-server --daemonize yes; bin/rails db:prepare >/dev/null; bin/rails test test/controllers/messages_controller_test.rb test/controllers/messages_drive_attachments_test.rb test/controllers/messages/cached_fragment_csrf_test.rb test/controllers/messages/legacy_presentation_cache_test.rb' > .scratch/rails-controllers-final.log 2>&1
```
```text
81 runs, 419 assertions, 0 failures, 0 errors, 0 skips
```

The source check returned 0 and printed `WS8bm Rails controller test source check: 4 mounted files match d7c7de92`. These are 81 Rails reference tests, not 81 newly ported Rust tests.

The Cargo commands below share this environment (executed in the verification shells):

```sh
export CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CARGO_BUILD_JOBS=4 CABLE_TEST_PORT_RANGE=52000-52049 MAIL_TEST_PORT_RANGE=52000-52049
```

### New root tests

```sh
mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire controllers::messages::root_tests -- --test-threads=4 --nocapture > .scratch/root-final.log 2>&1
```
```text
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 324 filtered out; finished in 0.75s
```

### Previous HTTP/fragment tests

```sh
mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire controllers::messages::http_tests -- --test-threads=4 --nocapture > .scratch/http-final.log 2>&1
```
```text
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 317 filtered out; finished in 0.80s
```

### Full seeded app binary

```sh
mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire -- --test-threads=4 --nocapture > .scratch/app-final.log 2>&1
```
```text
test result: ok. 329 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 24.42s
```

### Entire affected DB, richtext and views packages

```sh
mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_db -p campfire_richtext -p campfire_views -- --test-threads=4 --nocapture > .scratch/affected-final.log 2>&1
```
```text
test result: ok. 399 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 38.07s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.93s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.70s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.19s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.81s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

The app executed 329 tests, with three explicit ignores from main: `channels::tests::golden::record_reference`, `jobs::tests::push_latency`, and WS11's `controllers::presenters::accounts::tests::manages_bots`. Both required seeds were built; no integration test took a missing-seed return. Seed-helper tests intentionally exercise local skip behavior in a temporary missing-seed directory.

Affected packages executed 544 tests: 399 DB, 81 richtext and 64 views, plus empty doctest targets. Three DB reference-import/export helpers are explicitly ignored: `scenario_matches_ruby`, `fixtures_match_ruby_row_for_row`, and `export_database_for_rails`; their external cross-read setup was not run here. This is not a complete workspace test run.

The immediate post-merge, pre-implementation full app command was the same binary command above with `.scratch/app-after-merge.log` as the output file. Its raw baseline was:

```text
test result: ok. 321 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 25.27s
```

### Full-workspace/all-targets clippy

```sh
mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/clippy-final.log 2>&1
```
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.69s
```

Returned 0, no warnings; no workspace package was excluded.

### Compiled root regressions

```sh
python3 rust/reference-tools/messaging/root-discriminate.py > .scratch/root-discrimination-final.log 2>&1
```
```text
actions-membership: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.39s
actions-root-scope: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.40s
actions-immutable: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.40s
actions-viewer: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.42s
legacy-rendered-source: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.40s
legacy-snapshot: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.54s
markdown-clear: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.45s
update-job-atomicity: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.49s
update-csrf: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.42s
edit-http-source: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.43s
actions-bot-denial: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.38s
update-bot-denial: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.40s
json-wire-order: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.43s
edit-form-bytes: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.13s
WS8bm root discrimination: 14 compiled regressions detected; sources restored
```

### Cumulative compiled HTTP/fragment regressions

```sh
CI=1 python3 rust/reference-tools/messaging/discriminate.py > .scratch/messaging-discrimination-final.log 2>&1
```
```text
author-edit: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.38s
immutable-notes: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.41s
member-scope: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.60s
member-delete: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.41s
deleted-room: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.42s
root-thread-scope: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.39s
preview-sanitization: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.39s
preview-csrf: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.34s
create-markdown: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.38s
create-dedup: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.41s
scalar-columns: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.43s
raw-retry-blank: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.44s
create-drive: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.41s
create-job-atomicity: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.40s
markdown-fragments: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.09s
mention-shortcode-whitespace: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.40s
preview-json-escaping: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.40s
WS8bm discrimination: 17 compiled regressions detected; sources restored
```

## Failing-first evidence and limits

Before the new route/update implementation, the five initial root tests produced this raw result (`.scratch/root-before.log`):

```text
test result: FAILED. 1 passed; 4 failed; 0 ignored; 0 measured; 324 filtered out; finished in 0.63s
```

Actions returned 501; successful and invalid updates returned 500 instead of 200/422. The inherited rollback case already passed. The real edit-form byte test failed before replacing the stock template (`.scratch/edit-before.log`):

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 329 filtered out; finished in 0.12s
```

The added raw JSON checks failed on field order/entity escaping (`.scratch/json-bytes-before.log`) and the preview byte check failed before correcting its encoder (`.scratch/preview-json-before.log`):

```text
test result: FAILED. 6 passed; 2 failed; 0 ignored; 0 measured; 324 filtered out; finished in 0.91s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 331 filtered out; finished in 0.39s
```

These are recorded development baselines, not final results. The committed, rerun regression scripts above provide repeatable authorization failing-first proof: membership/root scope, immutable notes, viewer-state contamination, author/delete/deleted-room policies, valid bot keys, CSRF and the intended runtime tests all discriminate their compiled regressions. JSON wire-order and escaping injections also prove that parsed-value equality alone cannot pass the new byte gates. All 31 counted injections are named compiled runtime failures.

An initial inline-blob fixture attempt failed setup with `raised GlobalID.find`: the WS5 resolver has no signed ActiveStorage blob variant. That is not counted as failing-first coverage and is still a real attachment gap. The committed preservation case uses an actual Rails legacy OpenGraph attachment; it does not prove preservation of inline blobs. Compiler/setup failures, including the older mutation's ambiguous anchor, were excluded and corrected before the final discrimination runs.

## Deferred Rails tests and ownership

“Partial” below means selected real request/row/render contracts are ported, not the whole Rails file. The 23 focused Rust tests are 15 previous plus eight new. No browser/system test or screenshot/pixel parity was run.

| Rails controller test file under `test/controllers/` | Status and owner |
| --- | --- |
| `messages_controller_test.rb` | Partial, WS8bm: root Markdown create/retry, preview/limit/no-write/CSRF, author-only edits, non-author deletion denial, immutable notes and Markdown/system-note fragments. New additional guards cover removed/non-members, deleted room and root access to thread messages. Added root update rows/JSON bytes, source clearing, inline-unfurl conversion, edit forms, actions metadata, member/root scope, bot denial and update CSRF. Remaining show/index/create/destroy format responses, paging/ETag/unread, nonempty card/work/attachment states and all required broadcasts deferred. |
| `messages_drive_attachments_test.rb` | Partial, WS8bm: create normalization/order/dedup and six invalid-create differential cases, including scalar Drive input, invalid id, Unicode-space-wrapped id and nested object. Added update set replacement/removal, file-only JSON and nine edit-form byte comparisons. Viewer credentials/consent, uploads and broader odd shapes remain deferred. The WS8a max-count/body-with-attachment model behavior was not newly claimed as request coverage. |
| `messages/cached_fragment_csrf_test.rb` | Partial, WS8bm/WS6: new five-message Rails two-viewer cache oracle and Rust detached cache-hit checks contain no session-bound values. Full authenticated HTTP warming/update/broadcast scenarios deferred. |
| `messages/legacy_presentation_cache_test.rb` | Partial, WS8bm/WS6: existing core views pass; added legacy-source conversion through rendered Action Text, formatting-only edits and unfurl preservation. Inline blob conversion, HTTP cache invalidation and broader legacy states remain deferred. |
| `messages/boosts_controller_test.rb` | Deferred, WS8bm; bot subclass integration is WS11. |
| `messages/pins_controller_test.rb` | Deferred, WS8bm. |
| `message_forwards_controller_test.rb` | Deferred, WS8bm: reachable destinations, locked/stale threads, board refusal, direct display names, nested source routes and bounded queries. |
| `message_forward_sources_controller_test.rb` | Deferred, WS8bm: no-store canonical source URL and absence of inaccessible source identity. |
| `channel_threads_controller_test.rb` | Deferred, WS8bm. |
| `channel_thread_messages_controller_test.rb` | Deferred, WS8bm. |
| `channel_thread_messages_drive_attachments_test.rb` | Deferred, WS8bm. |
| `rooms/polls_controller_test.rb` | Deferred, WS8bm. |
| `rooms/pins_controller_test.rb` | Deferred, WS8bm. |
| `rooms/message_links_controller_test.rb` | Deferred, WS8bm. |
| `rooms/slash_commands_controller_test.rb` | Deferred, WS8bm; agent execution WS11 and huddle launch WS13 are integration seams. |
| `rooms/files_controller_test.rb` | Deferred, WS8bm; storage processor implementation stays WS storage. |
| `autocompletable/icons_controller_test.rb` | Deferred, WS8bm. Catalog use inside Markdown is covered; picker HTTP is not. |
| `autocompletable/slash_commands_controller_test.rb` | Deferred, WS8bm. |
| `autocompletable/users_controller_test.rb` | Deferred, WS8bm. |
| `saved_items_controller_test.rb` | Deferred, WS8bm; reminder notification handler WS17 and inbox policy WS12. |
| `scheduled_messages_controller_test.rb` | Deferred, WS8bm. |
| `searches_controller_test.rb` | Deferred, WS8bm. |
| `channel_threads_board_test.rb` | Deferred integration seam with WS12; board/work-thread feature is WS12, thread pane/message HTTP is WS8bm. |

No new browser/system test was ported or executed. Each owned system file is deferred to WS8bm: `boosting_messages_test.rb`, `code_highlighting_test.rb`, `pins_saved_test.rb`, `polls_test.rb`, `scheduled_messages_test.rb`, `search_forward_edit_test.rb`, `sending_messages_test.rb`, `slash_commands_test.rb`, `threads_test.rb`, `workspace_markdown_test.rb`. Additional matching system files are also deferred to WS8bm: `composer_test.rb`, `composer_attach_menu_test.rb`, `message_interactions_test.rb`, `message_actions_mobile_test.rb`, `message_toolbar_test.rb`, `message_list_a11y_test.rb`, `drive_attachments_test.rb`, `icons_test.rb`, `workspace_icons_test.rb`, `global_search_test.rb`, `search_files_test.rb`, `search_recents_clear_test.rb` and `unread_divider_test.rb`. Huddle/agent portions need WS13/WS11 coordination; Drive picker/consent portions need WS14. Message portions of `keyboard_shortcuts_test.rb`, `content_security_policy_test.rb`, `motion_test.rb`, `mobile_layout_test.rb` and `timezone_detection_test.rb` remain WS8bm integration coverage with WS8b-r/WS4. No claim is made for these mixed system files.

`messages/by_bots_controller_test.rb`, `messages/boosts/by_bots_controller_test.rb` and `agents/*` are WS11, not newly ported here. `work_threads*`, board controllers and `threads/work/*` remain WS12. The common room shell/sidebar/room CRUD and user/account pages are WS8b-r. The bots presenter is explicitly ignored by main pending WS11.


## Cross-workstream touches

- WS8a: the small `MessageChanges` extension above; callback/validation/index/reference/queue ownership remains WS8a.
- WS5/WS1/storage/WS15e: rendered-content conversion and existing embed-image signing are wired for legacy unfurls. Inline blob lookup/rendering and image-proxy HTTP remain unresolved integration seams. The standalone WS5 conversion contract was not silently changed.
- WS6/WS8b-r: modern edit template plus existing shared-menu proof; stable message-list boundary documented above. No shell/sidebar changes.
- WS19b: main merge brings strict seeded CI, ordered cable close and the WS11 ignore. Frozen-clock test boot is an additive testing seam.
- WS7: publisher/guard were not changed or relaxed. The inherited immediate create and presentation-only update path remain incomplete.
- WS11/WS12: root payload's read-only agent/work-owner permission lookup is a small presentation seam; nonempty agent/work cases are unproven. Valid bot-key denial is tested; valid agent-token denial remains an integration gap because the shared authenticator currently rejects every bearer token as unknown (401).
- WS14/WS17/WS13/WS15g: Drive consent/name credentials, notification handlers, huddle/agent slash execution and card integrations are still their coordinated boundaries.

## Precise remaining work, in the requested order

1. **Finish root messages first.** Verify/port exact show/index/create/destroy format, status, redirect, JSON no-store and Turbo-Frame behavior; destroy tombstone/thread-refresh effects; conditional ETags for related rows/pins/presentation and removal of the inherited Last-Modified validator. Add before/after tuple/coercion and room-shell around/last-read/unread/required total-count coverage. Complete message partial/cache-key/state goldens: bodyless/malformed, replies/deleted replies, attachment/Drive/credential states, fetched/suppressed cards and quote placeholders, forwards, boosts, pins, polls, streaming/agent steps and joined/closed/locked/work thread indicators. Actions' fetched-embed, icon and nonempty work-owner states and exhaustive odd update parameter shapes remain unproven. Modern composer/quick-edit integration remains.
2. **Rendered message broadcasts.** WS8a append/replace/remove descriptions through WS7 publisher and guard; all eight edit replacements, optional Drive replacement, reply tombstones and parent-thread refresh; exact bytes, recipient scopes and two-user cache/guard tests.
3. **Threads.** Controllers/messages, access/lifecycle/deleted-room/locked policies, pane, indicators, joined/unread state and broadcasts. Coordinate board/work behavior with WS12 and agent cases with WS11.
4. **Forwards/uploads.** Forward/source/destination privacy and bounded queries, `ForwarderCopier`, forwarded attachments, direct uploads, processing/variants/previews and inline blob SGID lookup/render/conversion. Existing happy-path PNG upload is inherited coverage, not completion of this slice.
5. **Polls/pins/saved/scheduled.** HTTP, open/voted/closed polls, pin lists/badges, saves/reminders, scheduled rows/send-now and broadcasts; notification integration with WS17.
6. **Search/slash/autocomplete.** Preloads/operators/cursors/total counts/DST coercion/results/chips, slash HTTP/pickers and `/play`, icons/users; coordinate agent invocation with WS11 and huddle launch with WS13.
7. **Deferred Rails/system cases.** Complete the table/system-file list above with real pinned goldens and browser pixel coverage, then broader package/runtime differentials and valid-agent authentication when WS11 lands.

No allowlist, parity mask, expected-value rewrite from Rust, schema change, Rails source edit, stash, rebase, PR or production action occurred. The branch's only untracked output is worker `.scratch/`. The external requested report and tracked mirror contain the same text. All worker commits end with the required GPT-6.1 Sol co-author.
