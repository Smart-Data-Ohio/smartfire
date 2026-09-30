# WS8bm messaging HTTP — partial four-slice continuation

Date: 2026-09-30. Worker: GPT-6.1 Sol. Branch: `rust/ws8bm-messages-http`.
Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm`.
Rails pin: `d7c7de92`. Main previously merged: `21a7332f`, merge commit `1caa4c63`.
Accepted starting slice: `9c8efaef`. Latest pushed implementation: `8a66d07b2f77ebb3919dc4493ee5e720f177b5cd`.
This report is committed and pushed afterward.

**PARTIAL.** Four coherent implementation slices were committed and pushed this continuation:

| Commit | Slice |
| --- | --- |
| `70c57a62` | Root paging, aggregate ETags, formats and destroy stream bytes |
| `41767551` | Rendered message edits/deletion tombstones and domain append/replace/indicator publishing |
| `b2d27dd3` | Thread join/read/leave HTTP and Rails pagination cursor coercions |
| `8a66d07b` | Collection dependency keys, ten cached states and four standalone message wrappers |

The final seeded app binary has **341 passed, zero failed, three explicit ignores**. Full-workspace/all-targets clippy passes with warnings denied. Ten owned Rails controller reference files pass: 156 runs and 949 assertions. Those reference counts do not mean all 156 cases were ported to Rust.

Per the lead's split, WS8b-m2 owns polls, pins, saved/reminder items, scheduled messages, search, slash/autocomplete, message links and room files. No HTTP implementation for those features was added here. Reading existing facts for message composition/cache keys is the shared root seam.

## Changes by file

Paths below are relative to `rust/`.

- `crates/campfire/src/controllers/messages.rs`: actual empty/nonempty root format behavior; aggregate validators before presentation; no Last-Modified; scalar/array/hash cursor behavior; all edit replacements; deletion tombstones and parent-thread refresh. Individual create/broadcast renders bypass collection caching. Earlier root create/update/edit/actions/JSON remains included.
- `controllers/messages/freshness.rs`: aggregate page/source/reply/card/poll/creator timestamps, sorted pin identities and presentation version. The timestamp query follows the actual pinned controller's list; the index digest is produced by ActionView in Rails.
- `controllers/messages/rendered.rs`: eight normal edit replacements, optional ninth Drive replacement, maintain-scroll; reply tombstones on their conversation streams and parent-thread refresh to room members; detached rendering of WS8a `Message`, `MessageReplace` and `ThreadIndicator` descriptions. Other descriptors remain for their owners.
- `channels/sink.rs`: synchronous DB-reader rendering preserves callback/publisher order, then uses the existing WS7 publisher and unchanged guard. Template-free frames retain prior handling.
- `controllers/channel_threads.rs`, `controllers.rs`: **join/read/leave only**. Alive parent membership, nested room/thread scope, involvement validation, atomic join/preference changes, idempotent reads/leaves, actual raw JSON/headers/redirects. Read never silently joins. Other thread actions remain unimplemented.
- `controllers/messages/payload.rs`: additive `pub(crate)` visibility for the existing thread payload adapter, with signature and shape retained.
- `controllers/presenters/message_cache.rs`, `controllers/presenters.rs`: collection-key adapter for Rails card/embed/PR-discussion/pin/thread-count/poll/system-note/streaming/agent-step/quote dependencies. Detached cached renders return the existing `MessageItem::Fragment` on misses and hits. No origin produces the existing `View` variant; presenter method signatures remain stable.
- `controllers/presenters/page.rs`: request render origin preserves **nonstandard ports**, correcting the previous report's statement about Rails dropping the port. `render_detached_at` now accepts `&AppState`; existing `&App` calls coerce unchanged.
- `controllers/presenters/test_support.rs`, `channels/tests/hub_test.rs`: additive frozen-clock boot/explicit Host; the older socket test drains all new edit frames before expecting its boost frame.
- `crates/kit/src/ctx.rs`: exact Rails ETag component order: template, flash, Turbo Frame. Small WS4 seam, checked by exact framed-request ETags and the full kit suite.
- `crates/views/src/messages.rs`, `templates/messages/{index.html,destroy.turbo_stream.html,show.html}`: exact index/destroy whitespace; additive meta/Drive/indicator partials, collection-key and uncached helpers; actual standalone message-format/list wrapper. Original record-version cache APIs remain available for existing consumers.
- `crates/views/src/fragment_cache/keys.rs`: additive nullable quote-name digest. Direct room names use Ruby `nil`; mixed nil/named rooms for one author reproduce Ruby's comparison error. Existing helper and `MessageKey` fields are unchanged.
- `controllers/messages/{paging_tests.rs,collection_tests.rs,root_tests.rs,tests.rs}`, `controllers/channel_threads/tests.rs`, `channels/tests/hub_test/message_parity.rs`: real request/row/header/HTML/frame comparisons, scope/CSRF checks and cache-state coverage; inherited index expectation corrected to no Last-Modified.
- `reference-tools/messaging/{paging.rb,broadcasts.rb,thread-memberships.rb,collection.rb,root.rb}`, `vectors/messaging/*`: real Rails requests/model callbacks/detached rendering. Broadcast capture records actual `ActionCable.server.broadcast` calls and invokes `super`; no substitute renderer or Rust-generated expected HTML.
- `reference-tools/messaging/{reference-check.py,check-goldens.py,check-controller-files.py,continuation-discriminate.py}`: exact pin/drift checks, fresh-seed oracle replay, per-file pinned Rails tests, seven named compiled regressions restored in `finally`.

## Stable entry point for WS8b-r and WS8b-m2

Use `campfire_views::messages::Index { ctx, messages: &[MessageItem] }`, building items with `Presenter::messages(&records)`. `Presenter::new`, `messages`, `message`, `message_item`, `MessageItem` variants and `Index` fields are unchanged. The template still calls `cached_message_item`.

Set `Presenter::cache_base_url` to the verified request origin and render under the app fragment-cache context, as `messages::present` does. Collection keys now include the full presentation dependencies, template digest and origin. Individual `View` items are uncached. `uncached_message(ctx, view)` and `uncached_message_html` are additive standalone/broadcast APIs; the original record-version `message`/`cached_message_fragment` APIs remain compatible but do not supply the full collection contract.

Root index recognizes `before`/`after` and **ignores `around`**, matching pinned Rails. Room-shell around selection is WS8b-r and can call WS8a's `Message::page_around`. Last-read/unread-divider/total-count integration remains.

The new cache-key adapter currently makes per-message aggregate queries; page-wide preloading/query-count parity remains required. Generic populated card components still default to empty content. Feature/integration owners must fill their presentation seams and verify the composed states after merge.

## Verification and raw evidence

All commands below ran in this worktree this continuation. Rust 1.98.1, locked resolution, four jobs, worker target and dev/test profiles; worker-prefixed Docker containers, pinned reference and test secrets. App tests use `CI=1` with freshly rebuilt default/first_run seeds, socket/mail range 52000–52049. No seed-dependent test silently skipped.

### Locked metadata and duplicate keys

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CARGO_BUILD_JOBS=4 mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1 >/dev/null
python3 - <<'PY' > .scratch/workspace-keys-continuation-final.log
from pathlib import Path
import subprocess
import tomllib
manifests=[Path(p) for p in subprocess.check_output(['rg', '--files', 'rust', '-g', 'Cargo.toml'], text=True).splitlines()]
root=tomllib.loads(Path('rust/Cargo.toml').read_text())
for manifest in manifests:
    tomllib.loads(manifest.read_text())
print(f"WS8bm workspace dependency check: {len(root['workspace']['dependencies'])} unique keys; {len(manifests)} Rust manifests parsed; 0 duplicate TOML keys")
PY
```

```text
WS8bm workspace dependency check: 75 unique keys; 15 Rust manifests parsed; 0 duplicate TOML keys
```

Metadata returned 0 without stdout. All 15 Rust manifests, including vendor/benchmark manifests, parsed; TOML rejects duplicate keys. No lockfile/dependency changes.

### Fresh pinned seeds

```sh
PARITY_NAMESPACE=ws8bm PARITY_OWNER=ws8bm PARITY_CPUS=2 PARITY_IMAGE=triage-reference-d7c7de92 TMPDIR="$PWD/.scratch" bash rust/parity/bin/seed build default first_run > .scratch/seeds-continuation.log 2>&1
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

### Source identity

```sh
python3 rust/reference-tools/messaging/reference-check.py > .scratch/reference-check-final.log 2>&1
```

```text
WS8bm reference source check: 29 controller, model, helper, template and icon files match d7c7de92
WS8bm reference check self-test: 2 injected source-byte/file-set differences rejected
```

### Fresh golden replay

```sh
python3 rust/reference-tools/messaging/check-goldens.py > .scratch/goldens-final.log 2>&1
```

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
WS8bm golden check: 7 Rails oracles re-run; 8 golden files byte-identical
```

Broadcast vectors include multiple recipients; socket tests compare all frames for the subscribed user's room/thread/unread streams, not a claim one user receives every reference frame. Collection states: initial, pinned/unpinned, empty thread/reply, streaming/final, edited, Drive-linked, reacted; each rendered twice. Three nullable-name probes evaluate the Ruby digest expression, not complete quote-card fixtures.

### Compiled authorization and other regressions

```sh
python3 rust/reference-tools/messaging/continuation-discriminate.py > .scratch/continuation-discrimination-final.log 2>&1
```

```text
root-page-scope: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 343 filtered out; finished in 0.62s
validator-pin-set: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 343 filtered out; finished in 0.55s
publisher-rendered-message: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 343 filtered out; finished in 0.15s
thread-cross-room: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 343 filtered out; finished in 0.41s
thread-join-csrf: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 343 filtered out; finished in 0.37s
collection-streaming: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 343 filtered out; finished in 0.13s
collection-null-room: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 343 filtered out; finished in 0.00s
WS8bm continuation discrimination: 7 compiled regressions detected; sources restored
```

Only intended named compiled runtime failures count. Sources were restored before final app/view/kit/clippy runs. The older 31 accepted mutations remain in their scripts but were not rerun or newly claimed this continuation.

Recorded development baselines before implementation, not final results:

Page bytes, validators and formats (`.scratch/paging-before.log`):

```text
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 332 filtered out; finished in 0.65s
```

Real publisher bytes: request port lost (`.scratch/broadcast-before.log`):

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 335 filtered out; finished in 0.55s
```

Scope/CSRF and JSON/rows initially reached 501 (`.scratch/thread-membership-before.log`):

```text
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 337 filtered out; finished in 0.42s
```

Existing-id array cursor must raise 500 after resolution (`.scratch/paging-coercion-before.log`):

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 340 filtered out; finished in 0.65s
```

Streaming cache invalidation (`.scratch/collection-before.log`):

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 341 filtered out; finished in 0.22s
```

Missing standalone wrapper (`.scratch/show-before.log`):

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 343 filtered out; finished in 0.12s
```

A later fixture setup mistakenly named an updated_at column on Drive attachments; corrected and excluded from failing-first proof.

### Final seeded app binary

```sh
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CARGO_BUILD_JOBS=4 CABLE_TEST_PORT_RANGE=52000-52049 MAIL_TEST_PORT_RANGE=52000-52049 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire --bin campfire > .scratch/app-continuation-final.log 2>&1
```

```text
test result: ok. 341 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 22.33s
```

Explicit ignores: reference recording (`channels::tests::golden::record_reference`), latency measurement (`jobs::tests::push_latency`), and main's WS11 `manages_bots`. All twelve new continuation tests and earlier messaging tests ran. One earlier full run returned an unexpected 302 in `administers_the_account`; isolated and subsequent full reruns passed. Cause unestablished; not labeled inherited.

### Shared views

```sh
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CARGO_BUILD_JOBS=4 CABLE_TEST_PORT_RANGE=52000-52049 MAIL_TEST_PORT_RANGE=52000-52049 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire_views --test core > .scratch/views-continuation-final.log 2>&1
```

```text
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
```

### Shared kit

```sh
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CARGO_BUILD_JOBS=4 CABLE_TEST_PORT_RANGE=52000-52049 MAIL_TEST_PORT_RANGE=52000-52049 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire_kit > .scratch/kit-continuation-final.log 2>&1
```

```text
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Kit ignores are two existing documentation examples (`error::halt` and crate introduction), not seed skips. No full-workspace test-suite claim.

### Full-workspace/all-targets clippy

```sh
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CARGO_BUILD_JOBS=4 CABLE_TEST_PORT_RANGE=52000-52049 MAIL_TEST_PORT_RANGE=52000-52049 mise exec rust@1.98.1 -- cargo clippy --locked -j4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/clippy-continuation-final.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.54s
```

Clippy exit 0; no warnings/errors.

### Owned Rails controller files, grouped reference counts

```sh
python3 rust/reference-tools/messaging/check-controller-files.py > .scratch/rails-controller-files-final.log 2>&1
```

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

The tool mounts `git archive d7c7de92 test` read-only into the pin, prepares the test DB and runs each file with one worker. These are 156 Rails passes/949 assertions; no entire file is claimed fully ported.

| File under `test/controllers/` | Reference passes | Rust coverage / remaining owner |
| --- | ---: | --- |
| `messages_controller_test.rb` | 56 | WS8bm partial: create/retry/preview/update/actions, root access/deletion policies, 16 pages/12 format requests, exact ETags/four show bodies. Unread/around, authenticated layout bytes, exhaustive attachment/card/agent/work/odd-input states, composer and query budgets remain. |
| `messages_drive_attachments_test.rb` | 19 | WS8bm partial: create/update sets, scalar/nested/id rejection, file-only JSON, edit forms/cached Drive links. Consent/credentials, wider shapes and uploads remain; service seam WS14. |
| `messages/cached_fragment_csrf_test.rb` | 4 | WS8bm/WS6 partial: five messages/two viewers, ten cache states and real delivered frames without session-bound values. Full authenticated warming/edit/broadcast and retraction permutations remain. |
| `messages/legacy_presentation_cache_test.rb` | 2 | WS8bm/WS6 partial: source conversion, formatting-only edits, unfurl preservation and whole-message/show composition. Inline blobs and broader legacy cache states remain, resolver WS5. |
| `messages/boosts_controller_test.rb` | 17 | Deferred WS8bm modern toggle/duplicate/limits/icon/clear/JSON/broadcast cases; inherited happy-path tests retained, bot subclass WS11. |
| `channel_threads_controller_test.rb` | 24 | WS8bm partial: eleven join/read/leave requests, rows/headers/raw JSON and security. Index/show/content/new/create/update/destroy, locks/lifecycle/pane and work/PR/agent payloads/broadcasts remain. |
| `channel_thread_messages_controller_test.rb` | 12 | Deferred WS8bm: all thread message index/show/actions/create/update/destroy and permission/format/cache cases. |
| `channel_thread_messages_drive_attachments_test.rb` | 13 | Deferred WS8bm: thread Drive HTTP/rows, locks/deletion/processing/uploads; credentials WS14. |
| `message_forwards_controller_test.rb` | 7 | Deferred WS8bm: destinations, stale/locked threads, board refusal, direct names, nested scopes, bounded queries and ForwarderCopier. |
| `message_forward_sources_controller_test.rb` | 2 | Deferred WS8bm: no-store canonical source URL, inaccessible identity protection. |

Now **WS8b-m2**, with no reference count/port claim here: `rooms/polls_controller_test.rb`, `messages/pins_controller_test.rb`, `rooms/pins_controller_test.rb`, `saved_items_controller_test.rb`, `scheduled_messages_controller_test.rb`, `searches_controller_test.rb`, `rooms/slash_commands_controller_test.rb`, `autocompletable/{icons,slash_commands,users}_controller_test.rb`, `rooms/message_links_controller_test.rb`, `rooms/files_controller_test.rb`.

No browser/system test or screenshot/pixel capture ran. WS8bm system files remain deferred: `boosting_messages_test.rb`, `code_highlighting_test.rb`, `sending_messages_test.rb`, `threads_test.rb`, `workspace_markdown_test.rb`, `composer_test.rb`, `composer_attach_menu_test.rb`, `message_interactions_test.rb`, `message_actions_mobile_test.rb`, `message_toolbar_test.rb`, `message_list_a11y_test.rb`, `drive_attachments_test.rb`, `unread_divider_test.rb`, and the forward/edit portions of `search_forward_edit_test.rb`. Search/pin/poll/scheduled/slash/autocomplete coverage is m2. Message portions of `keyboard_shortcuts_test.rb`, `content_security_policy_test.rb`, `motion_test.rb`, `mobile_layout_test.rb` and `timezone_detection_test.rb` remain WS8bm/WS8b-r/WS4 integration. `channel_threads_board_test.rb` and other board/work tests are WS12 with our pane/message seam; bot/agent controllers are WS11.

## Cross-workstream seams and precise remaining work

WS7 publisher/guard unchanged; WS8a domain callbacks/transactions stay separate from rendering. ETag ordering is WS4; cache helpers/standalone templates WS6; list entry point is stable for WS8b-r/m2. Test boot/Host additions are small seams. No room-shell/sidebar, m2 controller, Rails source, schema/migration/dependency, mask or allowlist changed.

Continue in the requested order:

1. **Root/composition:** room-shell around/last-read/unread-divider/total-count with WS8b-r; full authenticated show/edit/layout/Turbo-Frame bytes; exhaustive parameters and message states. Missing state coverage includes uploaded/inline blobs, credential/attachment variants, populated/suppressed cards, quote placeholders/renames/deletion, forwarded content, agent/work/PR/locked threads and modern boosts. Integrate m2's poll/pin states after merge. Page-wide preloading/query budgets and composer/quick-edit remain.
2. **Broadcast matrix:** basic append/replace/remove, edit targets, tombstones and indicator callbacks are verified. Finish populated card/quote/attachment/reaction/streaming-final bytes, source-card refresh/removal descriptors, two-user warm/cold permutations, recipient changes/deleted memberships and rollback/failure delivery. Poll/pin/scheduled descriptors are m2; agent streaming WS11.
3. **Threads:** all actions beyond join/read/leave, thread-message HTTP, pane and lifecycle/lock/deleted-room/work/PR payload/indicator/broadcast matrices. Work logic WS12, agent auth/invocation WS11.
4. **Forwards/uploads:** HTTP ForwarderCopier, source/destination privacy/stale/locked/board policies, copied attachments/bounded queries; multipart/direct signed uploads, size/processing/variants/previews/retention and inline ActiveStorage SGID lookup/render/conversion. Existing PNG happy path is inherited, not completion.
5. **Deferred cases:** complete Rust/Rails mapping for the owned files and browser parity. Validated reference pass counts are a baseline, not replacement Rust tests.

Open integration questions: providers for each populated generic card at merge (WS15g/WS15e/m2/WS14), availability of WS5 inline-blob resolver and WS11 valid agent authentication. Root's read-only agent permissions are unproven for valid bearer tokens while the shared authenticator returns 401. Nullable quote digest intentionally preserves Ruby's comparison failure.

Only worker scratch is untracked after the report commit. No stash, rebase, PR or production action. All commits have the required GPT-6.1 Sol co-author. External report and tracked mirror are identical.
