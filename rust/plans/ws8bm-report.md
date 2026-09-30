# WS8bm messaging HTTP — partial six-slice continuation

Date: 2026-09-30. Worker: GPT-6.1 Sol. Branch: `rust/ws8bm-messages-http`.
Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm`.
Rails pin: `d7c7de92`. Main remains `21a7332f2d3c324f0862cdf448baf17a84395aa0`, previously merged with merge commit `1caa4c63`.
This continuation started at report commit `d6f8c524`; its preceding implementation was `8a66d07b`.
Latest pushed implementation: `217886fb9bbdf6847cc626e16af3a8196aa57e70`. The report commit follows it.

**PARTIAL.** Six coherent slices were committed and pushed this continuation:

| Commit | Slice |
| --- | --- |
| `2e851955` | Verification from committed inputs; failure artifacts create their own directories; fresh-clone check |
| `2db55561` | Exact unread list-slot bytes and Rails around-page selection through an additive room-list adapter |
| `7e909c18` | Whole-message state and actual append/replace/remove publisher goldens |
| `4c46c88d` | Nested thread-message index/show/actions, scope and formats |
| `1884bd99` | Nested thread-message create/update/destroy, Drive sets, retry/lock policies and delivered broadcasts; bodyless Drive presentation |
| `217886fb` | Thread state listings and standalone index/show reads, work history/owner-picker JSON, ordinary thread HTML |

The final fresh-clone app run has **355 passed, zero failures, three explicit ignores**. Views core has **28 passed, zero failures**. Full-workspace/all-targets clippy passes with warnings denied. Fourteen new named Rust integration tests ran, in addition to the accepted earlier suite. The 18 compiled mutation checks each fail the intended runtime test, then restore the source. Ten owned Rails controller reference files pass: **156 runs, 949 assertions, zero failures/errors/skips**. Those Rails counts are a reference baseline, not a claim that every case was ported.

WS8b-m2 owns polls, pins, saved/reminder items, scheduled messages, search, slash/autocomplete, message links and room files. This continuation implements none of their controllers. Existing feature facts remain available to shared message composition.

## Changes by file

Paths below are relative to `rust/`.

- `crates/campfire/src/controllers/presenters/test_support.rs`: `rails_mismatch` creates a unique artifact directory under this worktree's target before writing a mismatch. It preserves the assertion failure even if writing artifacts fails. It no longer unwraps an assumed TMPDIR/artifact parent. Root/paging/collection and socket comparisons use it.
- `reference-tools/messaging/fresh-check.py`: clones the committed branch using `--no-local` into an owned, newly created directory; asserts the clone has neither `.scratch` nor `rust/target`; creates its own scratch/target and freshly generates default/first_run seeds before locked metadata, all seeded app tests, views core and full clippy. Tests read committed vectors. Neither this script nor the tests require a pre-existing untracked fixture/artifact directory.
- `controllers/presenters/room_list.rs`, `presenters.rs`, `crates/views/src/messages.rs`, `templates/messages/room_index.html`, `templates/messages/_unread_divider.html`: additive `Presenter::room_message_list`, `RoomIndex` and `UnreadDivider`. Exact Rails show-slot indentation and marker bytes, placement by record ID, and per-viewer markers outside shared message fragments. The adapter requires a verified render origin. It does not calculate room membership cursors/counts or modify WS8b-r's shell.
- `controllers/messages/room_list_tests.rs`, `reference-tools/messaging/room-list.rb`, `vectors/messaging/room-list.json`: nine real RoomsController GETs, selected root IDs/unread facts and the real pinned show list slot. Tests check WS8a around windows, invalid-anchor fallback/root scope, marker placement and exact list bytes. Full merged room-shell HTTP integration remains below.
- `controllers/messages/state_tests.rs`, `channels/tests/hub_test/message_parity.rs`, `reference-tools/messaging/message-states.rb`, `vectors/messaging/message-states.json`: eleven real message states, cache-miss/cache-hit bytes and 33 actual Rails append/replace/remove frames delivered by the Rust publisher to real sockets. States: legacy body, action, all-emoji, streaming, bot icon, system note, forwarded body/note, reply, deleted reply target, agent steps, and bodyless Drive-only message. Four real AgentStep states exercise position ordering, null/zero/subsecond/second durations and escaped multiline input/output. This verifies step presentation and message publisher output, not WS11's agent streaming protocol.
- `controllers/presenters.rs`: Drive-only messages with no ActionText body render the actual empty presentation after the attachment/sound paths, matching the Rails bodyless path. This corrects the previous empty trix-wrapper output. General unsupported cards/inline blobs remain partial.
- `controllers/channel_thread_messages.rs`, `controllers.rs`: all six nested message actions routed. Alive parent-room membership, nested room/thread/message/cursor scope, normal bot/agent/CSRF guards, latest/before/after reads, Rails's ignored around parameter on this endpoint, empty formats, canonical show redirect, JSON no-store behavior, actions JSON, create/update/destroy formats. Edits enforce author/system-note/lock rules; deletes allow the author or administrator, including locked threads. Duplicate client IDs are resolved before new attributes/lock checks, matching Rails and suppressing retry broadcasts.
- `controllers/messages.rs`: additive shared human-parameter/update/create helpers. Root entry points retain their behavior. Thread creation calls WS8a `post_message` inside the existing writer transaction, so membership join, reopening, the post and durable jobs are atomic. Update rechecks the lock inside the writer. Root webhook behavior stays on its root path. Broader odd shapes and uploaded attachments need further differentials.
- `controllers/messages/payload.rs`: existing method signatures retained, with additive visibility for shared message/actions/thread payloads. `thread_message` omits the root's thread-summary field. `thread_details` supplies the show-only work history and eligible, sorted human/agent owner options, with Rails compact/null rules. It reads existing WS12/WS11 rows and performs no work/agent mutation.
- `controllers/messages/rendered.rs`: additive `broadcast_thread_edit`, keeping the existing root edit entry point. Nested edits target the thread's messages stream; existing tombstones and thread-indicator refreshes render through WS7. The delivered-frame test now checks the actual channel identifier/stream as well as payload bytes, preventing a correct payload on the wrong conversation stream from passing.
- `crates/views/src/messages.rs`, `templates/channel_thread_messages/create.turbo_stream.html`: additive thread create stream, exact append target/newline and safe already-rendered message HTML.
- `controllers/channel_thread_messages/tests.rs`, `write_tests.rs`, `channels/tests/hub_test/message_parity.rs`, corresponding read/write reference tools and vectors: 18 real nested reads and 15 real writes; raw responses/headers, rows, Drive sets, retry behavior, lock/immutable-note restrictions and enqueue-rejection rollback. The write oracle captures 50 real publisher frames; the socket comparison checks all 44 frames on the subscribed user's four streams, plus silence after each request. It does not claim one user receives the other recipients' six frames.
- `controllers/channel_threads.rs`, `channel_threads/tests.rs`, `page_tests.rs`: index/show added to the existing join/read/leave slice. State aliases/default, ordering, stale/closed/locked/work/done lists, last-page replies, explicit deleted starter null, scoped browsing without join. Stale listing reads do not write closed_at. Actual work-history/eligible-owner JSON is verified, including a populated assignment event/note. Ordinary index/show HTML is exact inside the chosen application/frame layout; titles are checked against actual Rails response tags.
- `crates/views/src/channel_threads.rs`, `templates/channel_threads/index.html`, `show.html`, `crates/views/src/lib.rs`: additive thread list and ordinary standalone thread view models/templates. Populated work/board/PR HTML and the thread conversation/composer remain deferred; the current show template is only the ordinary-thread composition.
- `controllers/presenters/page.rs`: additive `titled_content` supplies Rails's page title to the application layout; the existing content helper delegates with no title. Existing callers and ViewContext fields remain unchanged.
- `reference-tools/messaging/thread-pages.rb`, `vectors/messaging/thread-pages.json`: 23 actual Rails GETs for state lists, standalone JSON/HTML/frame formats, work details, locked/empty threads and deleted starter. HTML goldens contain the real owned template output; they do not normalize session tokens out of the authenticated layout.
- `reference-tools/messaging/reference-check.py`, `check-goldens.py`, `continuation-discriminate.py`, `check-controller-files.py`: 41 pinned source files, fresh replay of 12 oracles/13 golden files, 18 compiled regressions, and per-file Rails controller counts. No masks or allowlists added or widened.
- `plans/ws8bm-integration.md`: stable list entry point and exact lead merge hook documented. This report is mirrored in `plans/ws8bm-report.md`.

## Stable message-list and presenter entry points

`Presenter::new`, `messages`, `message`, `message_item`, all `MessageItem` variants, and
`campfire_views::messages::Index { ctx, messages: &[MessageItem] }` remain unchanged. `Index` still calls `cached_message_item`. Set `cache_base_url` to the verified request origin and render under the app fragment-cache context. Individual/broadcast renders keep using additive `uncached_message`/`uncached_message_html`; they do not put viewer state into collection fragments.

For WS8b-r's message-owned slot inside `rooms/show`, the additive adapter is:

```rust
presenter.room_message_list(&messages, divider.message_id, divider.count)?
```

Pass that string as WS8b-r's `ShellComponents.message_list`. WS8b-r owns `find_messages` and `unread_divider`: room/anchor selection, membership cursors, unread counts, scroll/jump facts and read effects. The exact marker/list bytes and WS8a's selection window are tested here. The lead must connect this hook when merging both branches; **full HTTP shell unread/around behavior is not claimed by the adapter test**. The adapter has a narrow dead-code annotation stating that the separately owned shell will call it after merge. It adds no global warning suppression.

Root `messages#index` and nested thread-message index recognize before/after and ignore around, as pinned Rails does. Around belongs to the room shell and thread content endpoint. Existing APIs remain stable for m2. Page-wide message preloading and constant-query thread listings remain outstanding; the current presenters still make per-record aggregate/payload queries.

## Verification and raw evidence

Every command below was rerun in this continuation. Rust 1.98.1, locked dependencies, four jobs, worker-owned target directories, test/dev profiles. Docker names are ws8bm-prefixed, cpus capped at two, and the reference uses test secrets. Main is unchanged, so no additional merge was needed; the existing merge commit is retained. No schema, migration, dependency or Cargo.lock change.

### Locked metadata and duplicate workspace dependency keys

```sh
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1 >/dev/null
python3 - <<'WS8BM_WORKSPACE_KEYS_END' > .scratch/workspace-keys-thread-pages-final.log
from pathlib import Path
import subprocess
import tomllib
manifests=[Path(p) for p in subprocess.check_output(['rg','--files','rust','-g','Cargo.toml'],text=True).splitlines()]
root=tomllib.loads(Path('rust/Cargo.toml').read_text())
for manifest in manifests: tomllib.loads(manifest.read_text())
print(f"WS8bm workspace dependency check: {len(root['workspace']['dependencies'])} unique keys; {len(manifests)} Rust manifests parsed; 0 duplicate TOML keys")
WS8BM_WORKSPACE_KEYS_END
```

Metadata returned 0, with no stdout. Raw key summary:

```text
WS8bm workspace dependency check: 75 unique keys; 15 Rust manifests parsed; 0 duplicate TOML keys
```

TOML parsing rejects duplicate keys, including duplicate workspace dependencies. All 15 Rust manifests were parsed.

### Committed branch, new clone, new target, generated seeds

```sh
python3 rust/reference-tools/messaging/fresh-check.py > .scratch/fresh-thread-pages-final.log 2>&1
```

Raw output:

```text
WS8bm fresh checkout: 217886fb9bbdf6847cc626e16af3a8196aa57e70; no pre-existing scratch or Cargo target; /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-fresh-xdb_neuu
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 39s
test result: ok. 355 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 22.37s
    Finished `test` profile [unoptimized + debuginfo] target(s) in 17.52s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 03s
WS8bm fresh-check: committed inputs only; generated default/first_run seeds; app/views/clippy passed
```

The script runs, in that clone: locked metadata; parity seed build default/first_run; `cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire --bin campfire`; the same flags for `-p campfire_views --test core`; and `cargo clippy --locked -j4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings`, all via mise rust@1.98.1. CI=1 makes a missing/failed seeded app setup a failure. It sets CAMPFIRE_REFERENCE to the clone, owned TMPDIR/CARGO_TARGET_DIR, and cable/mail range 52050–52099. The regular worktree uses 52000–52049. The script's inputs are committed source/vectors and generated seeds, not pre-existing worker scratch files.

App explicit ignores remain: `channels::tests::golden::record_reference`, `jobs::tests::push_latency`, and main's WS11 `controllers::presenters::accounts::tests::manages_bots`. No seed-dependent test silently skipped. Views core has no ignores. No full-workspace test-suite claim; the clippy gate covers the full workspace and all targets.

### Pinned source identity and fresh golden replay

```sh
python3 rust/reference-tools/messaging/reference-check.py > .scratch/reference-thread-pages.log 2>&1
python3 rust/reference-tools/messaging/check-goldens.py > .scratch/goldens-thread-pages.log 2>&1
```

```text
WS8bm reference source check: 41 controller, model, helper, template and icon files match d7c7de92
WS8bm reference check self-test: 2 injected source-byte/file-set differences rejected
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
WS8bm message-states oracle: 11 real Rails states rendered cold/warm; 33 actual append/replace/remove frames; 0 session-bound values
WS8bm thread-message read oracle: 18 actual Rails requests; scoped pages, empty formats, raw JSON/actions/HTML and locked reads
WS8bm thread-message write oracle: 15 actual Rails writes; 50 publisher frames; retries, rows, Drive sets, locks and tombstones
WS8bm thread-pages oracle: 23 actual Rails requests; state lists, standalone HTML/JSON, latest replies and deleted starter
WS8bm golden check: 12 Rails oracles re-run; 13 golden files byte-identical
```

These oracles exercise our pinned Rails controller/model/template code. The publisher capture invokes the actual ActionCable broadcaster and its normal renderer. New state records come from real Rails model writes; goldens are committed. The Rails file-set checker rejects injected byte drift and missing files.

### Compiled authorization/rendering regressions

```sh
python3 rust/reference-tools/messaging/continuation-discriminate.py > .scratch/discrimination-thread-pages-final.log 2>&1
```

```text
root-page-scope: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 0.92s
validator-pin-set: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 0.87s
publisher-rendered-message: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 0.27s
thread-cross-room: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 0.59s
thread-join-csrf: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 0.57s
collection-streaming: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 0.16s
collection-null-room: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 0.00s
room-unread-marker: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 0.56s
forward-note-bytes: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 0.20s
agent-step-order: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 3.10s
nested-thread-room: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 0.56s
nested-message-scope: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 0.61s
nested-edit-author: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 0.76s
nested-edit-stream: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 1.10s
bodyless-presentation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 0.26s
thread-pages-bot: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 0.41s
thread-list-stale: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 0.77s
work-event-note: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 1.10s
WS8bm continuation discrimination: 18 compiled regressions detected; sources restored
```

Each mutation must compile, fail exactly the named runtime test and produce a failed test summary; compiler/setup errors do not count. All mutated sources are restored in `finally`. The final fresh-clone suite independently uses the unmutated committed implementation. These checks discriminate scope, CSRF, author gates, unread marker placement, cache keys, complete state/step bytes, the actual edit stream, bodyless presentation, bot denial, stale listing and populated event-note JSON.

Security tests were also run before route implementation. The recorded development baselines were 501 where 404/403 was expected:

- Nested reads: three named tests failed before the read routes.
- Nested writes: two named tests failed before the write routes.
- New thread pages (`.scratch/thread-pages-before.log`): raw summary below.

```text
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 354 filtered out; finished in 0.36s
```

The unread-slot differential first detected missing six-space show-slot indentation. The write-frame differential detected escaped Drive markup, an extra bodyless trix wrapper and the wrong edit conversation stream. Those are runtime parity findings. Incorrect fixture IDs/names, missing fixture timestamps, model/foreign-key setup mistakes, compiler errors, the title expectation corrected to real Rails tags, and stale fixture closure caused by its own setup posts are excluded from failing-first evidence.

### Owned controller files, grouped Rails reference pass counts

```sh
python3 rust/reference-tools/messaging/check-controller-files.py > .scratch/rails-files-thread-pages-final.log 2>&1
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

The tool mounts the pinned test archive read-only, prepares the reference test DB and runs each file with one worker. These 156 Rails passes/949 assertions do not replace Rust parity tests. No entire file below is claimed fully ported.

| File under `test/controllers/` | Rails reference passes | Rust coverage and precise deferred cases |
| --- | ---: | --- |
| `messages_controller_test.rb` | 56 | Partial root create/retry/preview/update/actions/destroy, 16 paging/12 format requests, exact ETags, four standalone bodies, legacy/action/emoji/streaming/system/forward/reply/deleted-reply/step/bodyless Drive states and actual frames. Still deferred: merged room unread/around HTTP, authenticated full layout/frame bytes, exhaustive odd params, uploaded/inline blobs and attachment states, populated cards/quotes/work/PR states, composer/quick edit and preload/query budgets. |
| `messages_drive_attachments_test.rb` | 19 | Partial root create/update Drive sets, scalar/nested/id rejection, file-only JSON, edit/cache/link bytes and bodyless state. Still deferred: consent/credential variants, broader shapes, upload/processing variants; service credentials WS14. |
| `messages/cached_fragment_csrf_test.rb` | 4 | Partial two-viewer cache proof from the earlier slice, 10 dependency states plus 11 new cold/warm complete states, and session-free actual publisher output. Still deferred: complete authenticated warming/edit/broadcast/retraction permutations and every recipient lifecycle. |
| `messages/legacy_presentation_cache_test.rb` | 2 | Partial source conversion, formatting-only edits, unfurl preservation, complete legacy/forward snapshot bodies and cache hits. Still deferred: inline blob resolver/conversion (WS5) and wider legacy/quote/card cache combinations. |
| `messages/boosts_controller_test.rb` | 17 | Inherited happy paths remain. Modern toggle/duplicate/limits/icon/clear/JSON/broadcast cases deferred to WS8bm; bot subclass WS11. |
| `channel_threads_controller_test.rb` | 24 | Partial join/read/leave plus 23 actual index/show reads: state aliases/staleness, locked/empty/deleted starter, last page, scope/no implicit join, ordinary HTML and work history/owner-picker JSON. Still deferred: content/new/create/update/destroy, lifecycle permissions/writes, pane/composer, populated work/board/PR HTML and broadcasts, constant-query/preload checks, fuller owner revocation/agent variants. Work domain WS12, agent auth/ledger WS11, PR presentation WS15g. |
| `channel_thread_messages_controller_test.rb` | 12 | Partial all six actions, 18 actual reads/15 writes, nested cursors, empty/XML/HTML/frame/JSON/Turbo formats, author/admin/immutable/lock gates, retries, rows, correct publisher streams, and enqueue rollback. Still deferred: exhaustive input/form/redirect/cache and lock/membership race permutations, populated card/quote/attachment states and multi-recipient changes. |
| `channel_thread_messages_drive_attachments_test.rb` | 13 | Partial thread Drive sets/file-only create/update/delete/retry/lock validation and output. Still deferred: broader Drive credentials/shapes, attachment deletion/processing/uploads; credentials WS14. |
| `message_forwards_controller_test.rb` | 7 | Deferred WS8bm HTTP entirely: destinations/ForwarderCopier, direct display names, nested scope, board/stale/locked refusal, copied attachments and bounded queries. |
| `message_forward_sources_controller_test.rb` | 2 | Deferred WS8bm HTTP entirely: no-store canonical source URL and inaccessible identity protection. |

Fourteen newly added Rust tests this continuation are grouped in `messages/room_list_tests.rs` (1), `messages/state_tests.rs` (1), `channel_thread_messages/tests.rs` (3), `channel_thread_messages/write_tests.rs` (3), `channel_threads/tests.rs` (2), `channel_threads/page_tests.rs` (2), and `channels/tests/hub_test/message_parity.rs` (2). A vector-driven Rust test often covers many real Rails requests; the grouped counts are named Rust tests, not one-to-one Rails case completion.

No browser/system test or screenshot/pixel capture ran. Owned system files remain deferred: `boosting_messages_test.rb`, `code_highlighting_test.rb`, `sending_messages_test.rb`, `threads_test.rb`, `workspace_markdown_test.rb`, `composer_test.rb`, `composer_attach_menu_test.rb`, `message_interactions_test.rb`, `message_actions_mobile_test.rb`, `message_toolbar_test.rb`, `message_list_a11y_test.rb`, `drive_attachments_test.rb`, `unread_divider_test.rb`, and forward/edit portions of `search_forward_edit_test.rb`. Message parts of `keyboard_shortcuts_test.rb`, `content_security_policy_test.rb`, `motion_test.rb`, `mobile_layout_test.rb`, `timezone_detection_test.rb` remain WS8bm/WS8b-r/WS4 integration. Board/work system cases stay WS12 with our pane/message seam; bot/agent controllers stay WS11.

Now WS8b-m2, with no count or port claim here: `rooms/polls_controller_test.rb`, `messages/pins_controller_test.rb`, `rooms/pins_controller_test.rb`, `saved_items_controller_test.rb`, `scheduled_messages_controller_test.rb`, `searches_controller_test.rb`, `rooms/slash_commands_controller_test.rb`, `autocompletable/{icons,slash_commands,users}_controller_test.rb`, `rooms/message_links_controller_test.rb`, `rooms/files_controller_test.rb` and their feature system coverage.

## Design and cross-workstream seams

Controllers authorize, call the existing WS8a writer/domain APIs, then present data or render views. No HTML moved into the domain. The nested post rollback test proves the durable job insert shares the post/join/reopen transaction. The publisher and conservative WS7 guard are unchanged; all delivered owned frames pass the guard, and the test compares the stream identifier.

WS8b-r owns the room shell/unread facts and must call the additive adapter after merge. WS8b-m2 can build on unchanged presenter/list signatures. WS6 owns shared layouts; our additive titled content helper and templates are small view seams. WS12 work event/owner rows are read for JSON, not mutated here. WS11's agent invocation/auth/streaming remains untouched; valid agent-token behavior is unproven while the shared authenticator returns 401, and a Bender-key-as-Bearer check must not be described as valid agent authentication. WS14 supplies real Drive credentials/composer flows. WS5 supplies inline blob resolution. WS15g/e supply populated card/work/PR seams.

No Rails source, room-shell/sidebar implementation, m2 controller, schema/migration/dependency, Cargo.lock, mask or allowlist changed. No stash, rebase, PR or production action. All six implementation commits were pushed and have the required GPT-6.1 Sol co-author. Only owned scratch is untracked after the report commit; external report and tracked mirror are identical.

## Remaining work, in requested order

1. **Root/unread/around and message states:** lead wire the documented adapter into the merged WS8b-r shell, then prove full HTTP unread/around/read effects and headers. Finish complete authenticated layouts/Turbo frames, odd parameter coverage, uploaded/inline attachments and media, quote placeholders/rename/delete, populated/suppressed cards, agent/work/PR/locked-thread combinations and modern boosts. Integrate m2 poll/pin presentation facts after merge. Finish message preload/query budgets and composer/quick edit. Bare append/replace/remove and the covered thread write frames are verified; recipient membership changes, warm/cold user permutations, populated card/quote/attachment/reaction variants and source refresh/removal descriptors remain.
2. **Thread controllers/pane:** `index`, `show`, `join`, `read`, `leave` and all six nested message actions are implemented as described. `content`, `new`, `create`, `update`, `destroy` are still unimplemented (501); thread conversation/composer, content anchoring/header bytes, lifecycle/metadata permissions and writes, populated work/board/PR sections, locks/deletion/recipient matrices, full indicators/broadcast combinations and constant-query counts remain. WS12 owns work/board domain changes; WS11 agent invocation/auth; WS15g PR data/views.
3. **Forwards/uploads:** both forward controllers remain unimplemented (501). Wire existing `ForwarderCopier`, authorize source/destination/privacy/direct/stale/locked/board policies, copy attachments and test bounded queries/source URL hiding. Finish multipart/signed/direct-upload parity, processing/variants/previews/sizing/retention and inline ActiveStorage SGID resolution/conversion. Inherited PNG upload coverage is not completion; no new upload differential was added this continuation.
4. **Deferred owned cases:** finish Rust mappings for the remaining grouped controller cases above and run the owned browser/system parity. Reference pass counts are recorded, but the remaining cases stay explicitly deferred to WS8bm or the named integration owner.

Open integration questions: which merged component providers supply populated work/board/PR/embed/quote/attachment content; WS5 inline-blob resolver availability; WS11 valid agent-token authentication. This report does not claim the full WS8bm brief is complete.
