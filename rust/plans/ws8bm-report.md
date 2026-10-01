# WS8bm controller declaration gate — integration partial

Date: 2026-10-01. Worker: GPT-6.1 Sol. Branch: `rust/ws8bm-messages-http`.
Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm`.
Pinned Rails: `d7c7de92`; approved #163 layout drift at `2e20b24c` only.
Implementation/fresh-clone input: **`cd717f274cf93154094700a00bbf2b92822e4f20`**. The later report-only commit changes no implementation or normal test inputs.

**The named-controller declaration gate is met: 143/156 have scoped Rust evidence; all remaining 13 are blocked only on other owners.** This closes 40 of the 53 declarations from the previous report. Case attribution is explicit in `plans/ws8bm-controller-cases.md`; it is not a claim of 156 one-to-one Rust tests or complete browser acceptance. Full owner-shell/schedule/state integration and behavior system sign-off remain **PARTIAL**.

Fetched origin twice in this continuation. Main remained `65ad0d391b4655bd2533771934e418bcd0d2efa2`, already merged by `e32d20ab`; `git rev-list --count HEAD..origin/main` returned `0`. No new merge was necessary. No stash, concurrency reduction, timing threshold change, expectation adjustment or pixel work. Earlier #171 attachment/after-commit fixes, real WS15g thread header/card, Drive matrix and live icon/history chrome are retained and rerun by the fresh workspace suite.

## Coherent pushed slices

- `5ca37a84`: cached-CSRF regressions run real two-viewer room message pages, refreshes and nested lists through the same fragment cache. Six Rails page responses and four owned reaction/legacy/GitHub forms are exercised with the actual thread page header token; missing/foreign tokens reject with 422 and unchanged rows. Rendering includes poll forms, but their submissions and the room-shell header remain owner-blocked. Neither session's token appears in shared fragments.
- `a8268d03`: ordinary-thread declarations cover stale unlock and sibling archive writes, direct-room refusal, locked/stale/closed ordering and one thread SELECT for closed JSON. HTML index reads batch owners, room members, eligible agent IDs and message counts instead of constructing full JSON for every row. Test-only, per-database reader SQL tracing observes the actual router; it does not claim writer-query instrumentation.
- `1534b7ac`: rendered message edits use the real WS15g GitHub and WS15e Twitter presenters. Five actual root/nested edits compare all 40 complete Rails replacement frames through WS7's publisher, guard and socket, along with reference rows and empty containers. A real root create delivers the exact Rails unread frames to human members and none to a signed-in non-member. Thread unread state is checked for both nothing/everything preferences; the nested HTML redirect is attributed to its exact Rails status/Location, with destination rendering still owned by WS8b-r.
- `c0e72e06`: the real forward picker excludes accessible boards, labels direct rooms with the other participant, remains no-store, and keeps reader-query cost constant after six reachable rooms and six threads are added. Existing complete Rails picker/refusal/source goldens remain unchanged. Board refusal/exclusion are implemented forward behavior; WS12 still owns board panes.
- `3e2f8889`: root edits compare exact Rails responses, body and edited/updated timestamps for identical Markdown, identical attachment saves, identical rich text, bodyless blanks and formatting-only changes. Real boosts, provider fetches and source destruction leave the corresponding messages unedited. Complete UTC meta matches in two viewer time zones. Real conditional GETs compare ETags after an off-page source edit and provider fetch without message touches. Actual v2 vulnerable cache entries and old validators cannot supply the v3 safe fragment or return 304. Admin delete and non-admin edit refusals are asserted through the real router and saved rows.
- `cd717f27`: all six bot/agent root-controller declarations are attributed through actual HTTP writes and durable jobs. Rich-text/Markdown responses match Rails in full; agent-backed bots enqueue no legacy jobs, deliver one real external POST with the Rails agent identity, and replay no second POST. A revoked grant delivers nothing. A bot without an agent row enqueues/delivers only the legacy webhook. Only external DNS/HTTP is redirected to a held listener; controller/model/jobs are real. The legacy delivery function now exposes the existing caller-owned transport seam with unchanged production behavior. Six new runtime negative controls discriminate these assertions. A separate actual HTTP event-reference probe records the WS14e blocker.

The message-list/presenter signatures stay stable for WS8b-r and M2. No M2 endpoint, room shell, event subsystem or WS12 work/activity/board pane is implemented here.

## Failing-first evidence and controls

The thread-index regression failed naturally at 33 versus 69 reader statements as threads grew, before batching. The provider regression failed naturally with an empty 216-byte GitHub replacement versus Rails's 1,274-byte real card, at byte 183, before using the real edit renderer. Original assertions were retained. The following are raw runtime results from this continuation (initial compile/oracle setup errors are not evidence):

```text
test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 1020 filtered out; finished in 1.38s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1020 filtered out; finished in 1.38s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1025 filtered out; finished in 1.96s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1029 filtered out; finished in 4.87s
```

The earlier already-correct security/provider paths were deliberately mutated to prove their regressions discriminate. Commands executed in this continuation:

```sh
python3 rust/reference-tools/messaging/check-owned-mutations.py cached-token-leak csrf-check-bypassed github-card-omitted drive-author-bypassed
python3 rust/reference-tools/messaging/check-owned-mutations.py thread-unread-uses-notification-preference forward-picker-n-plus-one identical-save-edited legacy-v2-cache-reused agent-root-delivery-omitted revoked-agent-delivery-allowed
```

The first group was rerun with the explicit four selectors after expanding the helper. Ten broken production paths were rejected across the two runs. Every mutation is restored in `finally`; no mutated file is committed.

```text
cached-token-leak: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1035 filtered out; finished in 3.19s
csrf-check-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1035 filtered out; finished in 2.28s
github-card-omitted: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1035 filtered out; finished in 1.00s
drive-author-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1035 filtered out; finished in 5.21s
WS8bm owned mutations: 4 rejected; 0 survived; production files restored
agent-root-delivery-omitted: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1035 filtered out; finished in 1.71s
revoked-agent-delivery-allowed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1035 filtered out; finished in 1.79s
thread-unread-uses-notification-preference: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1035 filtered out; finished in 18.44s
forward-picker-n-plus-one: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1035 filtered out; finished in 2.85s
identical-save-edited: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1035 filtered out; finished in 3.43s
legacy-v2-cache-reused: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1035 filtered out; finished in 1.02s
WS8bm owned mutations: 6 rejected; 0 survived; production files restored
```

The WS14e natural probe runs a legacy root PATCH containing PR, Twitter and event URLs, and compares Rails's actual event row. The request succeeds in both apps; Rails has `[390339825]`, Rust has `[]`. It is a new explicitly owner-flagged probe, not an ignored pre-existing test. Enable it when WS14e's transactional event-reference callback lands:

```sh
CI=1 CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=8 CABLE_TEST_PORT_RANGE=52000-52049 MAIL_TEST_PORT_RANGE=52000-52049 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire legacy_rich_text_edit_synchronizes_event_reference -- --ignored --nocapture
```

The probe first failed before adding the owner flag and was then rerun with `--ignored`; the selector and assertion are unchanged. Raw rerun:

```text
  left: Array []
 right: Array [Number(390339825)]
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1035 filtered out; finished in 2.05s
```

## Fresh checkout verification

Executed from a newly cloned committed branch, with no prior scratch or Cargo target. Generated default and first_run seeds, then ran locked metadata, the full workspace tests/doctests (excluding only vendored html5ever), and all-target workspace clippy with `-D warnings`. Eight test threads, two build jobs and the shared machine rustc flock slots were retained. No timing test failed. Aggregate: 2372 passed, 0 failed, 12 ignored across 48 raw result summaries; seeded app: 1033 passed, 0 failed, 3 ignored. The new explicit ignore is the proven WS14e owner blocker; existing ignores are retained.

```sh
python3 rust/reference-tools/messaging/fresh-check.py
```

The helper executes these Cargo checks under the documented pinned container/native setup:

```sh
cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1
cargo test --locked -j2 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast
cargo clippy --locked -j2 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
```

All 15 tracked Cargo manifests were separately parsed with `tomllib`; duplicate TOML keys would fail parsing. Raw result:

```text
WS8bm manifest check: 15 tracked Cargo manifests parsed; zero duplicate workspace dependency keys
```

Every fresh-run summary line is reproduced verbatim below:

```text
WS8bm fresh checkout: cd717f274cf93154094700a00bbf2b92822e4f20; no pre-existing scratch or Cargo target; /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-fresh-6v08xjlb
WS8bm fresh concurrency: eight test threads; two build jobs; no timing threshold changes
WS8bm pinned processing: campfire-toolchain; shared machine rustc flock slots
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4m 53s
test result: ok. 1033 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 1081.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.02s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.11s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.82s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 660 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 89.64s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.50s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.62s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.28s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.59s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 39.64s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.90s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.18s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.96s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.38s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.48s
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.84s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 16.25s
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 03s
WS8bm fresh target removed
WS8bm fresh-check: committed inputs only; generated default/first_run seeds; workspace tests/doctests/clippy passed
```


## Pinned Rails goldens and grouped controller reference

All commands below were rerun in this continuation. Rails reference counts are reference passes, separate from Rust case attribution.

```sh
python3 rust/reference-tools/messaging/check-goldens.py
python3 rust/reference-tools/messaging/check-controller-files.py
python3 rust/reference-tools/messaging/deferred-system-inventory.py
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
WS8bm cached CSRF: 6 actual two-viewer pages; 4 successful reaction/legacy/GitHub forms with the page header; polls and room-shell header separately owned
WS8bm thread declarations: closed/locked/stale ordering, direct refusal, stale unlock and sibling sweep through real Rails requests
WS8bm provider declarations: 5 actual root/thread edits; 40 exact message publisher frames; PR/Twitter add/remove and legacy reference synchronization
WS8bm legacy cache: vulnerable v2 fragment and validator; safe v3 fragment; both real conditional Rails requests return 200
WS8bm root declarations: 5 actual advancing-clock saves with exact response/row fields; reaction/fetch/tombstone no-edit facts; UTC meta in two actual viewer zones
WS8bm validator declarations: actual page 304, then byte-exact changed ETags after off-page source edit and card fetch without a message touch
WS8bm root recipients: 2 actual Rails personal unread frames; room members only
WS8bm bot controller declarations: 3 actual rich-text/Markdown root POSTs; exact response bytes and agent-only durable job counts
WS8bm event owner probe: actual legacy PATCH; Rails synchronizes the event reference (WS14e)
WS8bm golden check: 38 Rails oracles re-run; 39 golden files byte-identical
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

| Pinned controller file | Rails runs | Scoped declarations | Owner-blocked |
| --- | ---: | ---: | ---: |
| messages_controller_test.rb | 56 | 54 | 2 |
| messages_drive_attachments_test.rb | 19 | 19 | 0 |
| messages/cached_fragment_csrf_test.rb | 4 | 3 | 1 |
| messages/legacy_presentation_cache_test.rb | 2 | 2 | 0 |
| messages/boosts_controller_test.rb | 17 | 17 | 0 |
| channel_threads_controller_test.rb | 24 | 14 | 10 |
| channel_thread_messages_controller_test.rb | 12 | 12 | 0 |
| channel_thread_messages_drive_attachments_test.rb | 13 | 13 | 0 |
| message_forwards_controller_test.rb | 7 | 7 | 0 |
| message_forward_sources_controller_test.rb | 2 | 2 | 0 |
| Total | 156 | 143 | 13 |

## Stable shell/list/composer contract

The complete contract remains in `plans/ws8bm-integration.md`:

- Scrolling entry: `Presenter::messages(&records)` -> `messages::Index { ctx, messages }`. Set the presenter cache base URL to the verified request origin and render in the app's fragment-cache context.
- Shell-owned selection passes root records plus `divider.message_id` and `divider.count` to `Presenter::room_message_list`. Mount the returned list verbatim in `ShellComponents::message_list`, including its invitation-boundary whitespace. WS8b-r owns selection, membership cursors/counts, scroll/jump facts and read effects; the viewer divider stays outside shared fragments.
- Composer receives the request view context, room ID/kind/viewer display name, optional thread ID/name, ordered built-in then agent command names, and resolved Drive flow (`Share`, `Metadata`, `None`). `Presenter::composer_facts` and `composer_drive_flow` remain the read-only fact providers. Use `messages::composer::Composer` in the pane and `FooterComposer` for the room footer.
- M2 supplies its real `scheduled_messages::ComposerButton { ctx, room_id, thread_id }` as trusted `scheduled_control`. The clearly named `render_thread_schedule_control` remains explicitly empty until that provider is merged; no full pane/schedule claim is made.
- Mount `channel_threads::PendingTemplate` using the presenter viewer `UserView`. `Conversation` takes thread ID, parent room update time, scoped anchor, selected message items, viewer, ordered agent steps, composer facts and schedule child. WS7 stream signing/guarding remains active.
- The named `render_thread_pull_request_header` uses WS15g's actual private-safe renderer after room/thread authorization. The real GitHub message container is now populated in both list and edit broadcasts. The former 1,089-byte owned shell-card gap remains closed; a current combined nine-component shell pass still requires the owner merge.

Historical owner-integration scripts/patches are not current acceptance evidence. In particular their old merge handling must be refreshed before use; the normal suite never depends on those scratch integrations. The current branch does not merge the unmerged M2/room-shell branches.

## Exact remaining declarations and integration

No WS8b-m declaration remains without scoped evidence or an identified owner blocker. The 13 blocked declarations are:

1. WS8b-r: `room message list announces live appends` — actual merged room show route must mount the one `role=log`, `aria-live=polite`, `aria-relevant=additions` container.
2. M2/WS8b-r: `every form in a cached message submits with the page's header token` — all four currently rendered owned GitHub/reaction/legacy forms pass; actual poll submissions and merged room-shell header still need their owner endpoints/shell. Three other CSRF declarations are covered.
3. WS14e: `legacy rich-text edits re-sync card references` — GitHub/Twitter add/remove and rows pass; the event portion fails as recorded above pending WS14e synchronization.
4. WS12/WS11 thread work declarations: `converts a thread to work, assigns an eligible owner, and keeps an audit trail`; `work owner must be an eligible parent-room member and a revoked owner stays visible as unavailable`; `assigned owner can change work status but cannot reassign it`; `only a thread manager can remove work tracking`; `the work model also protects conversion when the owner field is omitted`; `work status updates from separate stale instances produce one event per real change`; `a manager can assign an eligible agent and the agent is notified`; `the owner picker lists eligible agents with profiles and excludes ineligible ones`; `a member who cannot manage the thread cannot assign an agent`; `ordinary thread fields remain separate from work tracking`. These ten stay flagged on the WS12 work/board controllers, not claimed by ordinary thread or WS11 delivery tests.

After the owner merges, remaining acceptance work is: wire the real room/thread schedule child; verify all nine current combined room components and complete live shell/chrome/provider pages; rerun the two-viewer cached-form submission across poll and shell contexts; check combined root/around/unread navigation and nested redirects; run combined message-state/upload/forward/recipient scenarios including M2 features and WS14e event cards; execute the behavioral system files below. Existing byte-exact 26-state cold/warm message corpus, Drive/media/forward/boost matrices and real multi-viewer socket sequences are scoped coverage, not a browser-system or full application cross-product pass. WS12 activity/work/board seams stay authorized/flagged 501. No pixel phase remains.

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

No deploy, PR creation or outbound message was performed. All launched commands have finished. The fresh helper removed its Cargo target, and the final recursive scratch check found zero remaining target directories. The ordinary worktree target remains the allowed build cache. Normal tests consume committed vectors and generated parity seeds, never scratch outputs. The final report-only commit changes this report and the documented integration contract; implementation/test code is exactly the fresh input above.
