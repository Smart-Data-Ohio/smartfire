# WS8bm merged integration and behaviour gate — partial

Worker: GPT-6.1 Sol. Branch: `rust/ws8bm-messages-http`.
Pinned Rails `d7c7de92`, plus approved #163 application layout/assets drift `2e20b24c` only.
Implementation and final fresh-clone input: `a64079e113a3d89a85d0cee2abce30f356e99a3c`.

The controller declaration gate is met: **146/156 have scoped Rust evidence; only ten WS12 work declarations remain blocked.** Behaviour system sign-off is partial: **12/135 mapped flows pass on both actual apps; 119 remain deferred and four are WS12-blocked.** These counts describe the assertion scopes in the named ledgers; they do not claim execution of the original Ruby system files or one-to-one Rust controller test counts.

## Pushed slices

- `b526ee4c0b4610854b76442c2bbc66bd9c8a3e15`: explicit merge commit with parents `98eadea6` and main `434d1c14`. Preserves both sides' behavior/tests and main's literal `INDEX_TEMPLATE_DIGEST` with test-only vector check. First fresh-clone workspace tests/doctests and clippy passed before this push, as requested; this is the PR-opening SHA. Release-input-only binaries build successfully.
- `2a819793`: real room list/composer/schedule integration, all six cached form CSRF checks through the actual room header, one actual live message region and the enabled WS14e legacy event-reference regression. The mounted room list now enters the shared collection-cache scope before rendering, rather than rebuilding outside it. Full room pages, all nine native list/composer/template components and actual complete thread pane/schedule bytes pass. Public message-list/presenter/composer inputs are unchanged.
- `b14a4a0e`: self-contained browser runner and exact 135-name ledger; six sending/editing/deleting/Markdown/keyboard/security flows against fresh Rails/Rust seeds, with two actual viewer contexts, real HTTP/Cable and raw persisted row checks.
- `6f4b2056`: three ordinary thread flows: create with separate channel draft, nested reply/edit/reaction, involvement/archive settings, live reply indicators through one/two/hidden, and same-context create re-entry preserving the name. No screenshot or pixel checks.

- `9782651d4c9bea95e32c3cde1c130678fe8e7173`: a second explicit main merge after the shared remote advanced to `b908ebc28f5b13039ce4dcca427bb1fb314b3d88` (#177 users/accounts and #172 huddles). Retains the worker's request-host Markdown plaintext, real shared list mount and main's equivalent ordered icon-name loader plus built-in icon helper. All other hunks merge both sides, including huddle delivery and both test sets. No stash or reduced tests.
- `a64079e113a3d89a85d0cee2abce30f356e99a3c`: three more ordinary thread behaviours on both apps: active/closed browsing and join/leave, external thread-link rejection before any external request, and literal malicious-name rendering. Exact closure, creator, membership and source rows are checked; the external-link case also preserves all seed message/thread/membership counts.

## Integration contract

Current stable entry points and every shell/composer input are documented in `plans/ws8bm-integration.md`.

`Presenter::messages(&records)` and `messages::Index { ctx, messages }` remain the scrolling-page seam. The real room adapter is `Presenter::room_message_list(&records, divider.message_id, divider.count)`, mounted verbatim through `room_native::message_list` and `ShowView.shell.message_list` under the app cache with verified `cache_base_url`. The populated list already owns the `\n    \n` invitation-expression boundary. The empty-room wrapper retains a single newline. The per-viewer divider stays outside shared fragments.

The shell supplies request `ViewContext`/origin/viewer/assets/signer/CSRF scope, room ID/kind/viewer-specific name, ordered built-in and agent command names, Drive flow and the real `scheduled_messages::ComposerButton { ctx, room_id, thread_id }`. Root composers pass `None`; `render_thread_schedule_control` passes `Some(thread_id)` at the named M2 seam. Preserve `Composer`/`FooterComposer` fields and the room caller's two-space footer prefix. `PendingTemplate` already includes its trailing source newline and stays request-owned. The pane also receives room updated time, scoped anchor, selected items, viewer and ordered agent steps. Its reads do not join the viewer.

WS15g's actual card/header providers remain mounted with verified origin and private-safe room/thread scoping. WS12 activity/work/board show and pane seams remain explicitly authorized, flagged 501 responses. No WS12 work implementation is claimed.

## Failing-first and preserved checks

Three natural merge/integration regressions were shown failing before fixes, with original expectations retained:

1. Main's M2 constant-query reader tracing was cleared by the worker's trace guard even when capture was disabled. The existing regression failed at `few > 0`. Install/clear the worker trace only when its own capture is active; preserve main's existing hook. Its regression and the worker's thread/forward reader traces pass.
2. The existing nine-component comparison failed because the pending template's source newline was appended twice. Preserve the owned template newline and populated list boundary in the shell wrapper. The first reported mismatch was the pending-template byte; no separate failing-first list-byte claim is made.
3. A new inert cache witness distinguishes mounting the actual shared collection value from recreating identical bytes. The actual room GET failed before moving both list and message-facts presentation into the cache scope, then passed for two real viewers. Cached payloads remain tokenless.

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1464 filtered out; finished in 1.53s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1464 filtered out; finished in 1.27s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1462 filtered out; finished in 1.72s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1464 filtered out; finished in 2.55s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1464 filtered out; finished in 0.89s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1466 filtered out; finished in 2.42s
```

The six-form oracle uses the real Rails room header and real form action/method/params/status. Missing and foreign tokens return 422 with unchanged message/boost/thread/vote counts. The original four-form thread-header and cold/warm two-session cache regressions remain. The merged Events probe performs a real legacy PATCH and matches Rails's `[390339825]` reference, status and location.

The merged review regressions also pass: `review_failed_thread_upload_rolls_back_like_rails`, `review_boolean_client_retry_matches_rails_one_row`, `human_attachment_edits_enqueue_atomically_on_roots_and_threads`, and `initial_jpeg_after_commit_matches_rails`. Pre-commit failures roll back the request's message/membership/reopen writes; the first initial JPEG's after-commit failure deliberately returns Rails's 500 **with** its committed message and thread. Do not describe every rendering/analysis failure as a rollback. Main's durable analysis path, sanitized filenames and the avatar/bot-icon/account-logo regressions are retained.

## Executed verification

- `python3 rust/reference-tools/messaging/fresh-check.py` ran the committed `a64079e1` in a new clone with no prior scratch or Cargo target, generated `default`/`first_run` seeds, and used eight test threads, two build jobs, the pinned toolchain/media image and shared machine rustc slots. All 58 workspace/doctest summaries total **3352 passed, one failed, 12 ignored**. All-target clippy with `-D warnings` passed. The helper exits 1, correctly retaining the main-owned room-race failure; this is **not a green final workspace run**. It removed its generated target.

The single failure is unchanged main/room-owner code, `controllers::rooms::directs_rails_cases::the_last_member_out_destroys_the_group`: line 155, “the deletion must have a durable job.” Earlier reads see a deletion-marked room and zero members; the final separate autocommit SELECT sees no queued job while the real worker can finish it between reads. This is the observed worker/queue timing failure, left for the deflake owner per instruction. No threshold, concurrency, assertion or expectation was weakened. An unchanged native diagnostic reran the whole directs group on Rust 1.98.1 with eight test threads and affinity limited to two CPU cores, against the generated seed. All 29 passed, including the failed case. This does not replace the full-run failure or constitute a fresh full-suite retry:

```text
test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 1674 filtered out; finished in 17.83s
```

The first explicit merge `b526ee4c` was pushed only after its full fresh gate passed (2899 workspace/doctest passes, zero failures, 12 ignored), as requested:

```text
test result: ok. 1462 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 1443.73s
```

The intermediate committed `6f4b2056` also completed a fresh full workspace/doctest/clippy run (2902 passes, zero failures, 11 ignored) before the asynchronous second main merge. The latest merge's raw output follows, including its failure rather than substituting a focused retry:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 7m 44s
test result: FAILED. 1699 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 1817.37s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.94s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 951 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 158.24s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.40s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.95s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.33s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 22.72s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.30s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.45s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.36s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.04s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.03s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.84s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.74s
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 20s
```

- From the same committed clean checkout, `python3 rust/reference-tools/messaging/behavior-check.py` ran **all twelve** mapped browser cases. The Cargo target had been removed after the full Rust run; this invocation rebuilt the candidate binary and both seeds, installed its own npm/Chromium inputs, and checked both actual apps. It exited 0. These raw lines are the clean-checkout result:

```text
WS8bm behaviour: sending_messages: sending messages between two users: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: sending_messages: editing messages: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: sending_messages: deleting messages: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour source: test/system/sending_messages_test.rb SHA256 4c637558cc041e470c6378f43e67d9d5b5b483ed0aae746c50eb64221027d116
WS8bm behaviour: workspace_markdown: Markdown messages reach other users and editing preserves the original source: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: workspace_markdown: desktop keyboard composition keeps line breaks and sends once after composition ends: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: workspace_markdown: untrusted markup stays inert in the delivered message: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour source: test/system/workspace_markdown_test.rb SHA256 b7b47897db51c2603fb733e90ecc7d2bf611a0f6d600926550ac80732e7b61fd
WS8bm behaviour: threads: creates a thread from a channel message and keeps the channel draft separate: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: threads: the thread root counts its replies live and hides the count when none remain: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: threads: a stray create re-entry does not wipe the half-filled thread name: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: threads: browses active and closed threads and can join or leave a closed one: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: threads: rejects an external thread deep link before fetching it: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: threads: renders untrusted thread metadata as text: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour source: test/system/threads_test.rb SHA256 e927613f9e78905aa072e0530339678b8443166b6f6d3c052d22cb3b797faafc
WS8bm behaviour check: 12 named cases passed on Rails and Rust; 0 failed; no pixel checks
```



- `CARGO_BUILD_JOBS=2 mise exec rust@1.98.1 -- bash rust/ci/with-release-inputs.sh cargo build --locked --workspace --bins -j2` copied only Cargo manifests/lock/crates plus the separate explicit asset context; no vectors, parity or reference tools were present in release inputs.

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 18s
```

- Locked metadata `mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1 >/dev/null` passed. All 15 tracked Cargo manifests parse without duplicate keys. The added-include audit reviewed 25 additions, all of which belong to a parent-confirmed `#[cfg(test)]` module. No new production include outside crates. Main's freshness literal is retained.
- `PARITY_IMAGE=triage-reference-d7c7de92 PARITY_CPUS=2 python3 rust/reference-tools/messaging/check-goldens.py` reran all 39 Rails oracles; all 40 golden files are byte-identical. The approved layout is separately regenerated and every non-layout field compared with the pin. No existing response golden was adjusted.

```text
WS8bm golden check: 39 Rails oracles re-run; 40 golden files byte-identical
```

- Focused real-router checks rerun the CSRF four-test group, Events probe, five native component/provider checks, two complete room pages and complete thread conversation/composer/schedule comparison:

```text
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 1463 filtered out; finished in 2.72s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1466 filtered out; finished in 2.34s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 1462 filtered out; finished in 2.77s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1465 filtered out; finished in 4.20s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1466 filtered out; finished in 19.05s
```

- `python3 rust/reference-tools/messaging/behavior-check.py sending_messages`, `... workspace_markdown`, and `... threads` automatically build the binary, both seeds, npm dependencies, Chromium and pinned browser reference image. Each named case owns fresh databases/servers and separate viewer contexts; no pre-existing scratch/target/bin or copied node_modules is required. These are mapped Playwright equivalents of the pinned case bodies, with source hashes verified, not an execution of the Ruby system suite. Initial/root textarea source remains literal; browser multipart edits save CRLF, verified from pinned Rails and checked as raw bytes on both databases. Server test ports are owned and occupied ports are refused; processes/containers are cleaned in `finally`.

```text
WS8bm behaviour check: 3 named cases passed on Rails and Rust; 0 failed; no pixel checks
WS8bm behaviour check: 3 named cases passed on Rails and Rust; 0 failed; no pixel checks
WS8bm behaviour check: 6 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

The first sending invocation timed out at the unchanged 30-second Rails Stimulus startup wait; the repeat with startup diagnostics passed. During the newer-main browser run, concurrent reference-container creation/removal produced actual Chromium `net::ERR_NETWORK_CHANGED` failures and another startup/menu timeout. After the oracle container churn ended, the six-thread-case run passed. Diagnostics retain actual network failures; expected navigation cancellations are omitted from diagnostic logging only, without removing any assertion. No timing threshold or concurrency was lowered. Thread-adapter setup errors (partial optional-label lookup, a mistyped preview string, right-clicking the whole row, and confusing the API status with its stored closure timestamp/creator fixture ID) were corrected to the original source's labeled targets, exact preview text, row menu-readiness and body right-click helper. These were not Rust application defects.

A further PR discussion experiment reached the real header/files child, but the production refresh worker fetched live GitHub data and changed the prepared card; the original system case captures the queued job. Full named-case acceptance needs a deterministic external GitHub transport fixture. That experiment is not in the committed default runner and is not counted as a pass. The case remains explicitly deferred.

## Original Rails controller files, grouped

`PARITY_IMAGE=triage-reference-d7c7de92 PARITY_CPUS=2 python3 rust/reference-tools/messaging/check-controller-files.py` reran the ten pinned controller files. These are Rails reference execution counts, not Rust case counts:

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

## Exact remaining scope

All owned named controller declarations are attributed; only the following ten work declarations remain behind WS12's flagged seams:

- converts a thread to work, assigns an eligible owner, and keeps an audit trail
- work owner must be an eligible parent-room member and a revoked owner stays visible as unavailable
- assigned owner can change work status but cannot reassign it
- only a thread manager can remove work tracking
- the work model also protects conversion when the owner field is omitted
- work status updates from separate stale instances produce one event per real change
- a manager can assign an eligible agent and the agent is notified
- the owner picker lists eligible agents with profiles and excludes ineligible ones
- a member who cannot manage the thread cannot assign an agent
- ordinary thread fields remain separate from work tracking

The 135 system declarations are enumerated by exact pinned name/source hash in `plans/ws8bm-system-cases.json`; `python3 rust/reference-tools/messaging/deferred-system-inventory.py` verifies that ledger. The earlier 156-name inventory also included 21 declarations already assigned elsewhere: keyboard shortcuts 14 and CSP 5 to WS8b-r, timezone detection 2 to WS8br2. Those remain with their owners.

| Pinned system file | Named | Passed flows | Deferred | WS12 blocked |
|---|---:|---:|---:|---:|
| `boosting_messages_test.rb` | 4 | 0 | 4 | 0 |
| `code_highlighting_test.rb` | 6 | 0 | 6 | 0 |
| `sending_messages_test.rb` | 3 | 3 | 0 | 0 |
| `threads_test.rb` | 15 | 6 | 5 | 4 |
| `workspace_markdown_test.rb` | 8 | 3 | 5 | 0 |
| `composer_test.rb` | 11 | 0 | 11 | 0 |
| `composer_attach_menu_test.rb` | 9 | 0 | 9 | 0 |
| `message_interactions_test.rb` | 10 | 0 | 10 | 0 |
| `message_actions_mobile_test.rb` | 2 | 0 | 2 | 0 |
| `message_toolbar_test.rb` | 13 | 0 | 13 | 0 |
| `message_list_a11y_test.rb` | 29 | 0 | 29 | 0 |
| `drive_attachments_test.rb` | 3 | 0 | 3 | 0 |
| `unread_divider_test.rb` | 5 | 0 | 5 | 0 |
| `search_forward_edit_test.rb` | 3 | 0 | 3 | 0 |
| `motion_test.rb` | 9 | 0 | 9 | 0 |
| `mobile_layout_test.rb` | 5 | 0 | 5 | 0 |
| Total | 135 | 12 | 119 | 4 |

Next work: the remaining ordinary thread browser cases (phone/focus, visible/hidden read state, shared older-message links and anchored unread behavior); deterministic GitHub discussion transport; remaining Markdown upload/mention/recovery/busy/mobile behaviors; then composer/attach menu, reactions/toolbars/actions, message-list accessibility, Drive/unread, search-forward-edit, motion/mobile behavior. Keep all four work flows and ten controller declarations flagged until WS12 lands. No system sign-off or pixel phase is claimed.

Cleanup completed: the full fresh helper removed its Cargo target. After the clean-checkout browser rebuilt one, that generated target was also removed. `find .scratch -type d -name target -prune -print` now returns no paths. Only logs and scratch source checkouts remain; no scratch target is an input to the committed tests. The normal worktree `rust/target` is not a scratch target.

The first merge and all implementation/browser slices above are pushed. The final workspace failure is retained for the main/room deflake owner, per the instruction to report timing flakes and move on; it must not be presented as a green final full-suite gate. The first PR-opening merge passed its complete fresh gate before being pushed. No screenshots or pixel diffs were performed.
