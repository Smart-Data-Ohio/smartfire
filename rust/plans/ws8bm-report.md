# WS8bm messaging HTTP — partial four-slice continuation

Date: 2026-09-30. Worker: GPT-6.1 Sol. Branch: `rust/ws8bm-messages-http`.
Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm`.
Rails reference pin: `d7c7de92`. Started from accepted report commit `5795df58`.
Main advanced during verification to `4278cb1e7a4529d5e2ecee51fccf69be6ed45257` (WS9 #161).
Merged with merge commit `4fd87662a7c4adb3681b4bdfdd1597c241b85c6d`; no rebase.
Subsequent test-scope fix: `62b83cc9`; final bounded fresh-verification commit: `62c7d045`. The report-only commit follows that verified input SHA.

**PARTIAL.** Four implementation slices and a validation slice were pushed separately:

| Commit | Slice |
| --- | --- |
| `68f6615b` | Ordinary thread creation, metadata/lifecycle updates and deletion; permissions and atomic row differentials |
| `fbf88b56` | Scoped thread content, conversation pane and additive Markdown composer/optimistic template |
| `5237eb59` | Forward endpoints through `ForwarderCopier`, private source resolver, copied-file rollback and initial thread upload processing |
| `3dc3d6e3` | Media, sounds and legacy forward states: 19 whole-message cold/warm goldens and 57 delivered publisher frames |
| `f5fe4741` | Eight compiled regression discriminators, stronger forward CSRF test and explicit deferred system inventory |
| `4fd87662` | Merge WS9 main; preserve both clock helpers and self-contained message failure artifacts |
| `62b83cc9` | Fix the main quote-job test's global-queue assertion; preserve job completion/failure checks |
| `62c7d045` | Bound fresh suite runtime concurrency to four threads; no assertion/threshold/ignore changes |

Final fresh clone: **460 app passes, zero failures, three explicit ignores; 445 DB passes, zero failures, four reference hooks ignored; 28 views core passes, zero failures; full-workspace/all-targets clippy passes with warnings denied.** Thirteen new named Rust integration tests cover many vector requests. These are not one-to-one counts of fully ported Rails cases. All ten owned Rails controller reference files passed (156 runs, 949 assertions, zero failures/errors/skips); those are reference baseline counts. No browser/system execution or pixel parity is claimed. WS8b-m2 retains polls, pins, saved/reminder items, scheduled messages, search, slash/autocomplete, message links and room files.

## Changes by file

Paths in this section are relative to `rust/`. Accepted earlier slices remain intact.

- `crates/campfire/src/controllers/channel_threads/writes.rs`, `controllers.rs`: route ordinary `new/create/update/destroy`. Alive parent-room membership, scoped root starter, normal bot/CSRF guards, direct-room refusal. Ordinary `new` returns Rails's 404; board new/create and work/board mutations remain explicit 501 pending WS12. Creation accepts initial message parameters, resolves retry client IDs before new attributes, joins/posts through WS8a inside one writer transaction, and preserves durable enqueue rollback. Metadata/lifecycle updates enforce creator versus moderator versus joined-member rules, validate before writes, and recheck room membership in the writer. Deletion uses `manageable_by` and emits the parent indicator. JSON/HTML statuses, cache/type headers, redirects, attempted HTML on validation errors, and rows match the ordinary lifecycle oracle. JSON compares whole response bytes; HTML checks the owned template as a contiguous response substring. Whole authenticated layout bytes remain deferred.
- `crates/db/src/models/channel_thread.rs`: small additive `update_metadata` method with Rails validation/no-op timestamp behavior. Additive `for_rooms` and `status_in_room` support batch destination reads without stale-thread write effects. Work/board owner/status side effects are not implemented here.
- `controllers/channel_threads.rs`: scoped `content` selects `last_page` or `page_around` using a verified same-thread anchor, preserves no implicit join, no-store and `X-Thread-Content-At-Latest`. Rails content JSON missing-template 500 is covered. Other content formats remain deferred. Existing standalone rendering was extracted to accept attempted metadata for 422 HTML without changing index/show callers.
- `controllers/channel_threads/write_tests.rs`: five named tests covering authorization first, the 27 actual lifecycle requests and row snapshots, creator/moderator separation, queue rejection rollback, and initial uploaded PNG analysis/downloadability. `content_tests.rs`: three named tests covering scope/bot denial/no join, actual selected windows/header/form-token behavior, and complete fixed-token conversation/room-composer bytes.
- `crates/views/src/channel_threads.rs`, `templates/channel_threads/_conversation.html`: additive `Conversation` and `PendingTemplate`, the single log live region, thread steps, message area/list controls, thread-only signed stream, jump controls and composer. `templates/messages/_pending.html` supplies the pinned Markdown optimistic message template. The old foundation Lexxy entry points remain available.
- `crates/views/src/messages/composer.rs`, `messages.rs`, `templates/messages/_composer.html`, `lib.rs`: additive stable Markdown `Composer { ctx, facts, scheduled_control }`, namespaced field IDs and thread/root form routes. `helpers/forms.rs` adds optional namespace handling for field IDs without changing parameter names. Full component bytes use the same fixed CSRF provider as the Rails render oracle; separate HTTP tests verify real tokens rather than masking them.
- `controllers/presenters.rs`: additive read-only `composer_facts`, `composer_drive_flow`, and ordered `thread_steps`. Built-in command names come from the existing domain registry, room command names/steps from current rows; no M2 command execution or WS11 agent mutation is added. Enhanced Drive Picker availability is an explicit Google-owner input.
- **Pane integration limitation:** the runtime content controller passes an empty schedule child. The byte comparison supplies the real Rails child as an explicit feature input. After merging M2, the lead must wire its `scheduled_messages::ComposerButton` into the same slot. Complete HTTP pane bytes are consequently still partial; this report does not confuse the component comparison with that integration.
- `controllers/message_forwards.rs`, `controllers.rs`, `message_forwards_tests.rs`: create/destinations/private-source endpoints. Source nested scope/reachability/bot/CSRF checks, writer recheck, domain forwarder with `ForwarderCopier`, batch room/thread/direct-member picker reads, board/locked refusal, no-store JSON and canonical source privacy. Copies are processed/reloaded before message broadcasts. Four named tests compare 18 actual refusal/picker/private-source responses, and check positive snapshots/Drive IDs/reopened membership plus separate private blob IDs/keys/bytes/checksums and rejection rollback of rows/copied files. Successful UUID-bearing forward bodies and delivered forward frame bytes still need differentials. Processing failures after the transaction remain a known atomicity gap below.
- `controllers/messages.rs`: additive visibility for the shared Rails string-column caster; root behavior unchanged. Initial thread uploads now call shared attachment processing before returning. No storage schema/service changes.
- `controllers/messages/state_tests.rs`, `channels/tests/hub_test/message_parity.rs`, `reference-tools/messaging/message-states.rb`, `vectors/messaging/message-states.json`: extend the accepted matrix to 19 actual states, complete cold/warm message bytes and all 57 append/replace/remove frames through real WS7 publisher sockets/guard. Added square/wide images, video, file, unrepresentable image, text/image sounds and legacy-forward flag false. Existing legacy/action/emoji/streaming/bot/system/forward/reply/deleted-reply/agent-step/bodyless Drive states remain covered. The lifecycle socket test checks frames and silence after all 27 actions; Rails produces no append for the initial thread post and a parent indicator on deletion.
- `reference-tools/messaging/thread-lifecycle.rb`, `thread-content.rb`, `forwards.rb`, associated `vectors/messaging/*.json`: pinned real Rails requests and model rows, full owned template bytes, actual publisher capture. Positive-forward properties do not use invented UUID goldens.
- `reference-tools/messaging/check-goldens.py`: copy storage generated by the seed for media replay instead of assuming prior scratch files. `fresh-check.py` now also runs DB tests and sets `RUST_TEST_THREADS=4` to bound runtime concurrency as well as build jobs. `reference-check.py` tracks the 54 source files actually read. New `lifecycle-discriminate.py` compiles eight regressions and restores source in `finally`; compiler/setup failures do not count as detection. `deferred-system-inventory.py` reports literal pinned declarations and zero execution, not pass counts.
- `controllers/presenters/test_support.rs`: merge resolution preserves main's `boot_with_clock`/`boot_with_clock_and_env`, additive `boot_with_test_clock` delegates to them, conditional default Host handling and self-created `rails_mismatch` directories remain. No test depends on a pre-existing `.scratch` or `rust/target` directory.
- `crates/campfire/src/jobs/tests.rs`: the fresh merged suite and a separately cloned main both exposed quote-refresh's assertion that the entire queue was empty while a periodic retention job was running. The test now intentionally leaves an unrelated future retention job queued, still waits for the quote job, fails if that job fails, and asserts its completion specifically. No runner/queue/production behavior changed.
- `plans/ws8bm-integration.md`: exact shell/list/composer seam. This report has an identical external copy at `wave4/ws8bm-report.md`.

## Stable shell inputs

No existing `Presenter::messages`, `MessageItem` variant, or `campfire_views::messages::Index { ctx, messages }` signature changed. Individual/broadcast renders retain `uncached_message`/`uncached_message_html`. WS8b-r owns room shell selection, membership cursors/read effects and the unread facts; this branch does not edit the shell.

The merged shell must supply the following:

1. For its message-list slot: a presenter using the verified request origin as `cache_base_url`, the app fragment-cache context, scoped root records from its room/anchor selector, unread divider `message_id: Option<i64>` and `count: i64`. Call `presenter.room_message_list(&records, divider.message_id, divider.count)?` and place the string in `ShellComponents::message_list`. The divider stays outside shared message fragments; no/off-page divider renders the ordinary list. Full merged HTTP unread/around/read-effect coverage remains WS8bm/WS8b-r integration.
2. For its composer slot: normal request `ViewContext` with verified origin, viewer, asset resolver, signer and CSRF secrets lent by layout rendering; `Facts { room_id, room_kind, room_name, thread, slash_commands, drive }`. `room_name` is viewer-specific for direct rooms. Use `thread: None` for root or `Some(Thread { id, name })` for a pane. `Presenter::composer_facts` supplies built-in names followed by ordered room agent-command names.
3. Drive mode: `Share` only when the Google owner resolves enhanced Picker availability, otherwise `Metadata` for the stored drive.file grant or `None`; `composer_drive_flow(viewer, share_picker_available)` reads the grant. Supply M2's rendered `scheduled_messages::ComposerButton { ctx, room_id, thread_id }` as trusted `helpers::Html` to `scheduled_control`. M2 owns the feature; no copied schedule implementation exists here.
4. Optimistic template: `channel_threads::PendingTemplate { ctx, user: presenter.user_view(viewer.id)? }`. For `Conversation`, also supply thread ID, parent room `updated_at`, scoped optional anchor ID, selected message items, viewer `UserView`, ordered thread step facts and the same composer/child inputs. It signs only the thread's messages stream and does not join the viewer.

These inputs and entry points are documented in `plans/ws8bm-integration.md` for the lead's merge. No full room-shell or pane integration claim is made before that merge.

## Verification and raw summaries

Every command cited below ran in this continuation. Rust 1.98.1 via mise, locked dependencies, four jobs, own test/dev targets. Original cable/mail ports 52000–52049; fresh clone 52050–52099. Docker ws8bm prefixes, two CPUs, pinned reference image. No schema, migration, dependencies, Cargo.lock, Rails source, parity masks or allowlists changed. The final fresh run follows the WS9 merge; the earlier Rails oracle/controller replays read the same unchanged reference pin.

### Merge metadata and duplicate keys

```sh
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1 >/dev/null
python3 - <<'WS8BM_WORKSPACE_KEYS_END' >.scratch/workspace-keys-final.log
from pathlib import Path
import subprocess
import tomllib
manifests=[Path(p) for p in subprocess.check_output(['rg','--files','rust','-g','Cargo.toml'],text=True).splitlines()]
root=tomllib.loads(Path('rust/Cargo.toml').read_text())
for manifest in manifests: tomllib.loads(manifest.read_text())
print(f"WS8bm workspace dependency check: {len(root['workspace']['dependencies'])} unique keys; {len(manifests)} Rust manifests parsed; 0 duplicate TOML keys")
WS8BM_WORKSPACE_KEYS_END
```

Metadata exited 0 without stdout. TOML parsing rejects duplicate keys, including workspace dependencies.

```text
WS8bm workspace dependency check: 75 unique keys; 15 Rust manifests parsed; 0 duplicate TOML keys
```

### Fresh main comparison and quote-job test correction

The first post-merge fresh app run failed with 459 passes, one failure and three ignores. The sole failure was `jobs::tests::ws8_quote_refresh_jobs_execute_in_the_real_app_runner`, which had completed quote refresh but asserted that a concurrently running retention job was absent. A fresh detached `origin/main` checkout at `4278cb1e` with new scratch, target and generated default/first_run seeds reproduced that same failure. This was not called inherited before that comparison.

The comparison executed:

```sh
python3 - <<'WS8BM_MAIN_COMPARE_END' >.scratch/main-compare-final.log 2>&1
import os
from pathlib import Path
import subprocess
import tempfile
root=Path.cwd()
clone=Path(tempfile.mkdtemp(prefix='ws8bm-main-check-',dir=root/'.scratch'))
subprocess.run(['git','clone','--quiet','--no-local','--single-branch','--branch','rust/ws8bm-messages-http',str(root),str(clone)],check=True)
subprocess.run(['git','checkout','--quiet','--detach','4278cb1e7a4529d5e2ecee51fccf69be6ed45257'],cwd=clone,check=True)
assert not (clone/'.scratch').exists() and not (clone/'rust/target').exists()
(clone/'.scratch').mkdir()
env=dict(os.environ,CI='1',TMPDIR=str(clone/'.scratch'),CARGO_TARGET_DIR=str(clone/'rust/target'),CABLE_TEST_PORT_RANGE='52050-52099',MAIL_TEST_PORT_RANGE='52050-52099',CAMPFIRE_REFERENCE=str(clone),PARITY_NAMESPACE='ws8bm-main-check',PARITY_OWNER='ws8bm',PARITY_CPUS='2',PARITY_IMAGE='triage-reference-d7c7de92')
print(f'WS8bm fresh main comparison: 4278cb1e7a4529d5e2ecee51fccf69be6ed45257; new scratch and Cargo target; {clone}',flush=True)
for name,args in [('seeds',['bash','rust/parity/bin/seed','build','default','first_run']),('app',['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j4','--manifest-path','rust/Cargo.toml','-p','campfire','--bin','campfire'])]:
    with (clone/'.scratch'/f'{name}.log').open('w') as log:
        result=subprocess.run(args,cwd=clone,env=env,stdout=log,stderr=subprocess.STDOUT)
    output=(clone/'.scratch'/f'{name}.log').read_text()
    for line in output.splitlines():
        if line.startswith(('seed:','test result:','    Finished')):print(line,flush=True)
    print(f'{name}: exit {result.returncode}',flush=True)
    if name=='seeds': assert result.returncode==0
print('WS8bm main comparison finished; inspect per-test failures in the fresh checkout',flush=True)
WS8BM_MAIN_COMPARE_END
```

Raw comparison summaries:

```text
WS8bm fresh main comparison: 4278cb1e7a4529d5e2ecee51fccf69be6ed45257; new scratch and Cargo target; /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-main-check-l50cg8cd
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
seeds: exit 0
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2m 27s
test result: FAILED. 397 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 46.64s
app: exit 101
WS8bm main comparison finished; inspect per-test failures in the fresh checkout
```

The corrected quote test deliberately schedules an unrelated retention job to reproduce the old assertion deterministically. It keeps its quote-job timeout and failed-job discrimination. Before and after that test assertion correction, the same focused command ran:

```sh
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52000-52049 MAIL_TEST_PORT_RANGE=52000-52049 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire --bin campfire jobs::tests::ws8_quote_refresh_jobs_execute_in_the_real_app_runner -- --exact
```

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 462 filtered out; finished in 0.68s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 462 filtered out; finished in 0.33s
```

The next unbounded fresh run passed the quote test but hit the unchanged OpenGraph parser's one-second performance limit (3.127915722s) under default test concurrency. Its raw summary was `test result: FAILED. 459 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 104.55s`. The final fresh run below bounds runtime concurrency to four test threads (`RUST_TEST_THREADS=4`), keeps the timing threshold and all app assertions unchanged, and has no new ignores or production runner changes.

### Fresh committed clone, new target, new seeds

```sh
python3 rust/reference-tools/messaging/fresh-check.py >.scratch/fresh-final.log 2>&1
```

```text
WS8bm fresh checkout: 62c7d045d434d8d81e43937888782ba655467e3c; no pre-existing scratch or Cargo target; /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-fresh-7lcjxtz5
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 48s
test result: ok. 460 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 97.48s
    Finished `test` profile [unoptimized + debuginfo] target(s) in 26.04s
test result: ok. 445 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 62.24s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
    Finished `test` profile [unoptimized + debuginfo] target(s) in 21.02s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 50.93s
WS8bm fresh-check: committed inputs only; generated default/first_run seeds; app/db/views/clippy passed
```

The script clones the committed branch with `--no-local`, verifies neither `.scratch` nor `rust/target` exists, creates its own directories, generates default/first_run seeds, and sets `CI=1`, `RUST_TEST_THREADS=4`, `CAMPFIRE_REFERENCE` to the clone, owned TMPDIR/target and port ranges. It runs locked metadata; seeded `campfire --bin campfire`; all `campfire_db` tests/doctests; `campfire_views --test core`; and full workspace/all-targets clippy with `-D warnings` (all via mise rust@1.98.1, `-j4`). No full-workspace runtime-test claim beyond those suites.

Explicit app ignores: WS11 `controllers::presenters::accounts::tests::manages_bots`, the running-reference recorder and push latency measurement. DB has four explicit main ignores: `tests::differential_test::scenario_matches_ruby`, `tests::fixtures_test::export_database_for_rails`, `tests::fixtures_test::fixtures_match_ruby_row_for_row`, and WS9's `tests::two_factor_rollback_test::read_rails_rollback_changes`; these need separate Rails mutation/export tooling. This continuation added no ignores. No missing seed silently skipped. Views core has no ignores.

### Reference identity and golden replay

```sh
python3 rust/reference-tools/messaging/reference-check.py >.scratch/source-final.log 2>&1
python3 rust/reference-tools/messaging/check-goldens.py >.scratch/goldens-final.log 2>&1
```

```text
WS8bm reference source check: 54 controller, model, helper, template and icon files match d7c7de92
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
WS8bm message-states oracle: 19 real Rails states rendered cold/warm; 57 actual append/replace/remove frames; 0 session-bound values
WS8bm thread-message read oracle: 18 actual Rails requests; scoped pages, empty formats, raw JSON/actions/HTML and locked reads
WS8bm thread-message write oracle: 15 actual Rails writes; 50 publisher frames; retries, rows, Drive sets, locks and tombstones
WS8bm thread-pages oracle: 23 actual Rails requests; state lists, standalone HTML/JSON, latest replies and deleted starter
WS8bm thread-lifecycle oracle: 27 actual Rails actions; creation retries, metadata/tags, lifecycle permissions, rollback rows and delete frames
WS8bm thread-content oracle: 9 actual requests; anchor scope and fixed-secret conversation/composer bytes
WS8bm forwards oracle: 18 real picker/refusal/source-privacy requests; exact JSON bytes
WS8bm golden check: 15 Rails oracles re-run; 16 golden files byte-identical
```

These invoke our actual pinned Rails controllers/models/templates and broadcaster. Goldens are committed; no expected body was generated from Rust. Full conversation byte goldens use an explicit deterministic token provider on both renderers, with the real Rails schedule child passed as the separate owner input. HTTP tests verify real form CSRF/window/header behavior. Successful forward UUIDs are tested as properties rather than claimed byte-identical.

### Failing first and compiled regressions

Development authorization tests ran before their routes were implemented. The stubs returned 501 instead of expected scoped 404/403. Initial upload processing first failed the analyzed-metadata assertion. Captured raw summaries (development logs, not current failures):

```text
thread-lifecycle-before: test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 358 filtered out; finished in 0.49s
thread-content-before: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 363 filtered out; finished in 0.37s
forwards-before: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 366 filtered out; finished in 0.46s
thread-upload-before: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 370 filtered out; finished in 0.43s
thread-upload-after: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 370 filtered out; finished in 0.44s
```

```sh
python3 rust/reference-tools/messaging/lifecycle-discriminate.py >.scratch/discrimination-final.log 2>&1
```

```text
creator-lock-permission: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 370 filtered out; finished in 0.52s
creator-delete-permission: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 370 filtered out; finished in 0.56s
initial-upload-processing: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 370 filtered out; finished in 0.58s
tag-validation-atomicity: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 370 filtered out; finished in 0.58s
content-anchor-scope: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 370 filtered out; finished in 0.42s
forward-source-scope: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 370 filtered out; finished in 0.37s
forward-source-privacy: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 370 filtered out; finished in 0.51s
forward-create-csrf: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 370 filtered out; finished in 0.39s
WS8bm lifecycle discrimination: 8 compiled regressions detected; sources restored
```

These checks ran before merging WS9; the fresh post-merge suite independently verifies restored committed code. Each mutation must compile and fail exactly the named runtime test with a FAILED summary. The first CSRF mutation attempt exposed a weak request: malformed input returned 422 even without the guard. The committed test now submits a valid forward; removing the guard produces a write/201 and the test fails. Creator lock/delete permission, upload processing, tag validation atomicity, anchor/source scope and source privacy are also discriminated. Setup/compiler/fixture mistakes are excluded from failing-first evidence.

### Owned Rust file groups in the final fresh run

The completed fresh app log was parsed by test module prefix, requiring every named result in each group to be `ok`. The actual command was:

```sh
python3 - <<'WS8BM_OWNED_COUNTS_END' >.scratch/owned-counts-final.log
from pathlib import Path
import re
log=Path('.scratch/ws8bm-fresh-7lcjxtz5/.scratch/app.log').read_text()
groups=[('controllers/messages/http_tests.rs','controllers::messages::http_tests::'),('controllers/messages/paging_tests.rs','controllers::messages::paging_tests::'),('controllers/messages/collection_tests.rs','controllers::messages::collection_tests::'),('controllers/messages/room_list_tests.rs','controllers::messages::room_list_tests::'),('controllers/messages/state_tests.rs','controllers::messages::state_tests::'),('controllers/channel_thread_messages/tests.rs','controllers::channel_thread_messages::tests::'),('controllers/channel_thread_messages/write_tests.rs','controllers::channel_thread_messages::write_tests::'),('controllers/channel_threads/tests.rs','controllers::channel_threads::tests::'),('controllers/channel_threads/page_tests.rs','controllers::channel_threads::page_tests::'),('controllers/channel_threads/content_tests.rs','controllers::channel_threads::content_tests::'),('controllers/channel_threads/write_tests.rs','controllers::channel_threads::write_tests::'),('controllers/message_forwards_tests.rs','controllers::message_forwards_tests::'),('channels/tests/hub_test/message_parity.rs','channels::tests::hub_test::message_parity::')]
rows=re.findall(r'^test ([^ ]+) \.\.\. (ok|FAILED|ignored[^\n]*)$',log,re.M)
for file,prefix in groups:
    statuses=[status for name,status in rows if name.startswith(prefix)]
    assert statuses and all(status=='ok' for status in statuses),(file,statuses)
    print(f'{file}: {len(statuses)} passed; 0 failed; 0 ignored')
print('WS8bm owned Rust modules: 13 file groups passed; named tests are not one-to-one Rails case port counts')
WS8BM_OWNED_COUNTS_END
```

```text
controllers/messages/http_tests.rs: 15 passed; 0 failed; 0 ignored
controllers/messages/paging_tests.rs: 4 passed; 0 failed; 0 ignored
controllers/messages/collection_tests.rs: 2 passed; 0 failed; 0 ignored
controllers/messages/room_list_tests.rs: 1 passed; 0 failed; 0 ignored
controllers/messages/state_tests.rs: 1 passed; 0 failed; 0 ignored
controllers/channel_thread_messages/tests.rs: 3 passed; 0 failed; 0 ignored
controllers/channel_thread_messages/write_tests.rs: 3 passed; 0 failed; 0 ignored
controllers/channel_threads/tests.rs: 5 passed; 0 failed; 0 ignored
controllers/channel_threads/page_tests.rs: 2 passed; 0 failed; 0 ignored
controllers/channel_threads/content_tests.rs: 3 passed; 0 failed; 0 ignored
controllers/channel_threads/write_tests.rs: 5 passed; 0 failed; 0 ignored
controllers/message_forwards_tests.rs: 4 passed; 0 failed; 0 ignored
channels/tests/hub_test/message_parity.rs: 5 passed; 0 failed; 0 ignored
WS8bm owned Rust modules: 13 file groups passed; named tests are not one-to-one Rails case port counts
```

These 53 named tests include accepted earlier slices and this continuation; a named test can drive multiple real reference requests. They are separate from the Rails reference counts below.

### Owned controller files: reference passes and Rust deferrals

```sh
python3 rust/reference-tools/messaging/check-controller-files.py >.scratch/controllers-final.log 2>&1
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

The script mounts tests from `git archive d7c7de92 test` read-only, prepares the pinned Rails test DB and runs each file with one worker. The **156 Rails runs/949 assertions are reference passes**, not proof that every case is ported. No full file is claimed completely ported.

| File under `test/controllers/` | Rails passes | Rust coverage / precise deferred cases and owner |
| --- | ---: | --- |
| `messages_controller_test.rb` | 56 | Accepted root writes/paging/formats/ETags/actions and this matrix/publisher expansion. Deferred WS8bm: complete authenticated layout/frame and unusual input shapes, all upload/inline/card/quote combinations, modern boosts, recipient/cache permutations and measured preload/query budgets. Merged room unread/around/read effects need WS8b-r seam integration. |
| `messages_drive_attachments_test.rb` | 19 | Partial root Drive sets/file-only/edit/cache/link bytes remain. Deferred WS8bm/WS14: consent/credential and wider shape variants, signed/direct uploads and processing/variant failures; Google owner supplies credential/Picker inputs. |
| `messages/cached_fragment_csrf_test.rb` | 4 | Two viewers, dependency-key states, 19 complete cold/warm states and session-free publisher output covered. Deferred WS8bm: complete authenticated warming/edit/retraction and recipient-change permutations. |
| `messages/legacy_presentation_cache_test.rb` | 2 | Conversion/format-only edits/unfurl preservation/legacy-forward snapshots covered. Deferred WS8bm/WS5: inline ActiveStorage resolver/conversion and wider quote/card/cache combinations. |
| `messages/boosts_controller_test.rb` | 17 | Inherited happy paths. Deferred WS8bm: modern toggle/duplicate/limits/icon/clear/JSON/broadcast cases; bot subclass/agent auth WS11. |
| `channel_threads_controller_test.rb` | 24 | All ordinary read/membership/lifecycle endpoints covered in partial matrices; 27 lifecycle plus 9 content requests, pane/composer component bytes, initial PNG, rollback and deletion frames. Deferred WS8bm: other content formats, malformed params, remaining race/recipient/indicator and full HTTP pane/layout cases, measured query/preload counts. Board new/create/work/owner/status/delete hooks WS12 (explicit 501). Populated work/board HTML WS12 + our composition; PR HTML/data WS15g. M2 child wiring after merge. |
| `channel_thread_messages_controller_test.rb` | 12 | Accepted six message actions, 18 reads/15 writes/50 reference frames, nested/author/immutable/lock/retry/rollback gates remain. Deferred WS8bm: broader parameter/form/cache/race/recipient and upload/card/quote combinations. |
| `channel_thread_messages_drive_attachments_test.rb` | 13 | Accepted thread Drive/file-only/retry/delete/lock sets remain. Deferred WS8bm/WS14: credentials/wider shapes, deletion/processing/direct-upload combinations and storage failure atomicity. |
| `message_forwards_controller_test.rb` | 7 | This slice adds endpoints/copier; 18 refusal/picker/source responses, snapshot persistence, Drive/membership properties, private copies/bytes/checksums and enqueue rollback. Deferred WS8bm: successful JSON/HTML wire byte and delivered forward-frame differentials, malformed shapes/nil bodies, processing failure atomicity, full destination recipient races and measured bounded-query assertion. |
| `message_forward_sources_controller_test.rb` | 2 | Exact no-store canonical URL/null privacy and nested scope covered. Deferred WS8bm: full source deletion/membership races and unusual coercions/formats. |

New named Rust tests by file: `channel_threads/write_tests.rs` 5; `channel_threads/content_tests.rs` 3; `message_forwards_tests.rs` 4; `channels/tests/hub_test/message_parity.rs` 1 (lifecycle). Existing state/socket tests were broadened, not recounted as new tests.

M2-only controllers remain with M2: room/message polls/pins, saved items, scheduled messages, searches, room slash commands, autocomplete, room message links and files. No pass/port counts for them are claimed here.

### System files: zero executed, all explicitly deferred

```sh
python3 rust/reference-tools/messaging/deferred-system-inventory.py >.scratch/systems-final.log 2>&1
```

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

These are literal `test` declarations from the pinned files, not discovered/executed case counts. Every file has **zero system passes claimed**. Message/controller interaction coverage belongs to WS8bm; merged room/composer/unread/layout/keyboard/mobile integration also needs WS8b-r. Search portions of `search_forward_edit` belong to M2. Google attachment flows need WS14, agent/auth flows WS11, board/work flows WS12, CSP integration WS4 and populated PR/cards WS15. Controller/vector tests do not replace browser or screenshot/pixel evidence.

## Design and remaining work in requested order

Authorization stays in controllers; transactions/domain/copier remain in WS8a/WS3, HTML in views/presenters, broadcasts through the existing WS7 publisher and conservative guard. Atomic thread/post/enqueue and copied-file enqueue rollback are tested. Neither existing presenter APIs nor the shell's list entry point changed. The WS9 merge retains main authentication semantics; no bypass was added.

1. **Threads:** ordinary new/create/update/destroy/content and pane components are implemented as above. Complete merged HTTP pane bytes by supplying M2's schedule child; prove other content formats/unusual params, membership/lock/owner races and recipient/indicator permutations. Finish populated work/board/PR sections with their owners, work/board mutations after WS12, and measured constant-query/preload cases. Ordinary initial creation correctly emits no append in the captured Rails actions.
2. **Forwards/uploads:** positive snapshot/copier/PNG processing is implemented. Finish positive JSON/HTML response and actual forward-broadcast differentials, malformed/nil inputs and recipient/destination races. **Known parity gap:** forward attachment analysis/variant processing currently runs after the domain transaction, while Rails processes inside the forward transaction; a processing exception can leave committed forward rows. Initial thread/root upload error atomicity likewise needs its failure matrix. Add multipart/signed/direct-upload cases for media/variants/previews/metadata/retention/failed processing and inline SGID resolution with WS5. Batch picker queries are implemented but have no measured query-budget assertion yet.
3. **Full message state matrix:** 19 complete states/57 delivered frames pass, but this is not the full matrix. Finish inline ActionText blobs, missing/corrupt metadata, quote placeholders and rename/delete, attachment plus reply/forward/reaction combinations, populated/suppressed cards, work/PR/agent lifecycle and locked-thread combinations; integrate M2 poll/pin facts. Finish modern boosts, all warm/cold viewer/recipient/membership permutations and measured preload/query budgets. WS5/WS11/WS12/WS15 own the named foundation data/processing seams, M2 its features.
4. **Deferred cases:** port remaining grouped Rust controller cases and execute the 19 owned/shared system files with the merged shell/feature/auth branches; browser/pixel parity is still unverified. Full shell HTTP unread/around/read effects remain to verify after the lead wires the stable inputs above. Reference controller passes are recorded separately.

No Rails source, room shell/sidebar, M2 feature implementation, schema/migrations/dependencies/lockfile, mask or allowlist changed. No stash, rebase, PR or production action. Co-author trailers are present. Only owned `.scratch/` remains untracked; external report and tracked mirror are identical after the final report commit. The WS8bm brief remains partial.
