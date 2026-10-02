# WS8bm message actions and code behaviour — PR-ready partial slice

Worker: GPT-6.1 Sol. Branch: `rust/ws8bm-messages-http`.
Reference: Rails `d7c7de9264c63015be398001d7a1094e7695a6db`, plus the approved drift in wave4/_common.md. No screenshots or pixel comparisons.

**103 passed / 26 remaining / 6 WS12-blocked**, from 135 exact named system declarations. This continuation credits **30 new declarations**, retaining the received 73. The complete fresh source-clone gate passed: **3,704 Rust tests / zero failures / 12 inherited ignores**, plus all thirty paired browser cases and thirty valid specific mutant rejections. No invalid run is counted.

This continuation credits all thirteen toolbar declarations, both mobile message-action declarations, all ten message-interaction declarations, and five code-highlighting declarations. These are scoped Playwright equivalents against both actual applications with saved-row assertions, not execution of the original Ruby system files. Each newly credited declaration rejects a specific served mutant. No production Rust, dependency, lockfile, asset override, or test timing policy changed.

Controller attribution remains **146/156; all ten remaining declarations are blocked only on WS12**. The owned controller declaration gate is met. The six WS12 system items stay flagged pending board/work writes (#187) and agent work services. Activity/work/board seams keep their authorized 501 responses. Behaviour sign-off remains partial; this is a coherent PR-ready continuation, not complete system sign-off.

## Pushed changes and assertion scope

- `0677829e154fccba16307159f052c65c432a3329`: add scoped message actions, toolbar, mobile menu and code behaviour checks, exact storage assertions, source-anchored served mutants, and the optional browser portion of the fresh-clone gate. The uncredited GitHub discussion fixture retains its failed-card diagnostic. The received base remains `4f0472908784e4e0ade5902ce2eb1c9f2111c85d`.
- `behavior-actions.mjs`: original keyboard/context-menu/long-press controls, cancelled movement and release-click suppression, phone sheets and desktop dismissal, real edits with draft restoration and peer delivery, notify-off reply/deleted-source tombstone, actual forwarding to a server-provided thread, duplicate-submit suppression and two-user reaction counts/toggles. The asynchronous-edit test keeps the original deterministic PATCH gate and 422 response boundary; it does not claim a domain write. Duplicate redelivery runs the real Turbo render queue on the actual mounted row and checks object identity; it does not claim an extra server-published duplicate frame. Clipboard spies keep the original permission-independent boundary and execute the actual copy handlers.
- `behavior-toolbar.mjs`: all thirteen original declarations, including actual reaction writes and peer counts, keyboard focus, categories, data fetched only on first opening, recent persistence, workspace and brand icons, and option/tab navigation.
- `behavior-code.mjs`: twelve fenced languages and aliases, unchanged text, literal/untrusted code, search/channel/back navigation, copying through worker failure, and real edits with changed language, copied source and peer delivery. The original 20-second highlight wait is extracted from the pin. No screenshot or computed-color assertion was added.
- `behavior_action_rows.py`: compare every saved message projection and boost against the Rails-created seed; require exact reaction actor/content, edit source/edited flag, reply notify flag/deleted source, one forward with the real copied richtext snapshot, requested thread/note and unchanged thread count for open controls.
- `behavior-fixtures.rb`: Rails creates the actual workspace icon, thread destination and code/search rows. Code literals and waits come from test files materialized with `git show` at the pin, because the production reference image intentionally excludes tests. The tool builds its own seeds and inputs; no fixture depends on old scratch contents or a pre-existing target.
- `behavior.mjs`, `behavior-check.py`, `behavior-mutations.mjs` and `behavior-action-mutations.mjs`: dispatch exact pinned names, use the original actors, isolate browser contexts and writing fixtures, verify persisted rows and reject applied mutants only after the named case starts. A startup, missing mutation, network failure or unexpected adapter error cannot count as mutant rejection. Mutation contexts alone block service workers so navigation cannot bypass the served mutation; positive acceptance uses the application's real worker.
- `fresh-check.py`: optional `--behavior-files` runs requested paired groups and their mutants inside the same fresh source clone after workspace tests and clippy. The target is removed after all checks. Existing eight test threads, two build jobs, machine-wide rustc slots and timing thresholds remain unchanged.
- `plans/ws8bm-system-cases.json`: exact source hashes/names, scoped evidence and explicit deferred/owner statuses. Previous deliveries and evidence remain available in the [received report at 4f047290](https://github.com/Smart-Data-Ohio/smartfire/blob/4f0472908784e4e0ade5902ce2eb1c9f2111c85d/rust/plans/ws8bm-report.md); they are not represented as rerun commands in this report.

## Stable integration contract

The shell seam in [ws8bm-integration.md](ws8bm-integration.md) is unchanged: `Presenter::messages(&records)`, `messages::Index { ctx, messages }`, and `Presenter::room_message_list(&records, divider.message_id, divider.count)`. The shell passes verified request origin/viewer/assets/signer/CSRF context, selected records and divider facts. The composer receives room kind/name/ID, ordered commands, Drive flow, thread scope, the real scheduled-message child and request-owned PendingTemplate. Mount list/template boundaries verbatim. Real WS15g GitHub rendering and merged WS14e/M2 children remain in use. There is no new cross-owner production touch.

## Failing-first evidence and invalid runs

The initial action mutant run rejected 24 distinct checks and correctly marked one invalid: the shared menu helper waited for an author-only Edit action when David opened his menu. The helper now waits for the universally available Copy text action; geometry still waits for the author's metadata-dependent Edit action. The unchanged named reaction mutant then rejected correctly. The five code mutants also rejected before positive acceptance. Complete fresh results are recorded below rather than adding overlapping run totals.

Earlier acceptance exposed adapter errors, not established application defects: accessible names are Copy link and Add a note (optional); the production image lacks test source; Back to Designers needs the navigation completion that Capybara itself waits for. These were corrected without changing any application expectation. A repeat batch failed Rails Stimulus startup with ERR_NETWORK_CHANGED, then candidate startup with Address already in use (os error 98). Those are invalid runs and are not credited as parity evidence or repaired timing bugs. An earlier picker run completed its named assertions and saved-row checks while logging a failed background connection refresh; it does not establish that every background refresh passed.

The GitHub discussion declaration remains deferred. Its first real POST and thread header/files pass on pinned Rails; after returning to the room, the live provider worker records `Could not reach GitHub (Resolution Error)`. The diagnostic confirms the existing Discuss link is present, but the original Fix login title has been replaced by the provider-error rendering. This live-worker fixture differs from the original Rails system test's unperformed job adapter. The title/header assertions were not weakened, no candidate parity pass was credited, and the applied header-stripping mutant alone does not establish positive parity.

The previously intermittent reply-upload declaration remains deferred. No queue-observation race, inherited test timing failure, ignore, concurrency reduction or threshold widening was changed to obtain a green run. In particular, `the_last_member_out_destroys_the_group` stays with the deflake owner.

## Current verification

From the worktree root, reran:

```sh
python3 rust/reference-tools/messaging/fresh-check.py --behavior-files message_toolbar message_actions_mobile message_interactions code_highlighting
python3 rust/reference-tools/messaging/check-controller-files.py
```

Raw fresh output (all 58 Rust summaries, exact browser case results and applied-mutant results):

```text
WS8bm fresh checkout: 0677829e154fccba16307159f052c65c432a3329; no pre-existing scratch or Cargo target; /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-fresh-yz61rhi9
WS8bm fresh concurrency: eight test threads; two build jobs; no timing threshold changes
WS8bm pinned processing: campfire-toolchain; shared machine rustc flock slots
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4m 35s
test result: ok. 1859 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 1155.16s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.59s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.13s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 1143 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 141.10s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.38s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.95s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.48s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.16s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.14s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.69s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.22s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.85s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.75s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.57s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.79s
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 35s
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 05s
WS8bm behaviour: message_toolbar: the toolbar stays hidden until hover or focus and labels every action: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: reply and thread buttons drive the composer and the thread panel: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: the more button opens the shared menu for its message: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: the picker shows category tabs and switches between them: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: the picker loads its emoji data only on first open: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: picker tabs move with arrow keys and switch the grid: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: quick-react creates a boost from the toolbar: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: keyboard users reach the toolbar from a focused message: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: the emoji picker searches and reacts: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: the picker remembers recent reactions: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: the picker Custom tab reacts with a workspace icon: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: the picker reacts with a brand icon shortcode: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: picker arrows move through options, Enter selects, and Escape returns focus: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour source: test/system/message_toolbar_test.rb SHA256 d0e63bfb2c31db13f5786afd5ff28728f7dffead32468143fe3cb513598fdc12
WS8bm behaviour: message_actions_mobile: message action menu is a bottom sheet with touch-sized targets on phones: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_actions_mobile: message action menu stays a floating popover on desktop: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour source: test/system/message_actions_mobile_test.rb SHA256 4d930fbf29081825c2696f1be81ec062b0e6968031b93fef02d47b08818e856d
WS8bm behaviour: message_interactions: opens message actions from context menu and keyboard, and cancels a moving long press: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_interactions: a release click landing on the just-opened menu does not activate it: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_interactions: shows the message action menu as a bottom sheet on phones: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_interactions: a duplicate delivery does not replace the message while its actions are open: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_interactions: keeps newer typing through an asynchronous edit and leaves failures in edit mode: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_interactions: edits through the normal composer and restores the saved draft on cancel and success: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_interactions: replies with notify off and renders a tombstone when the target is deleted: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_interactions: copies message text and link and forwards to a server-provided thread destination: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_interactions: forwarding twice in a row submits only once: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_interactions: groups emoji reactions, updates the live count, and highlights the current user: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour source: test/system/message_interactions_test.rb SHA256 4c867b252b0697ea694a9a3637b4242195638e261b62ccfe6e90ff21ea20d8db
WS8bm behaviour: code_highlighting: language fences highlight common code without changing its text: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: code_highlighting: unlabelled code is detected while text unknown languages and inline code stay literal: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: code_highlighting: search results highlight code on initial load and after returning to the channel: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: code_highlighting: code and copying remain available when the highlighter cannot load: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: code_highlighting: editing a code block replaces its language colors and copied source: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour source: test/system/code_highlighting_test.rb SHA256 bd8e2522a1fab8e0054b401d199acc3a942f4e4be866b7b1c011de650bed2014
WS8bm behaviour check: 30 named cases passed on Rails and Rust; 0 failed; no pixel checks
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.14s
WS8bm discrimination: message_toolbar: the toolbar stays hidden until hover or focus and labels every action: served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_toolbar: reply and thread buttons drive the composer and the thread panel: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: the more button opens the shared menu for its message: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: the picker shows category tabs and switches between them: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: the picker loads its emoji data only on first open: served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_toolbar: picker tabs move with arrow keys and switch the grid: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: quick-react creates a boost from the toolbar: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: keyboard users reach the toolbar from a focused message: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: the emoji picker searches and reacts: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: the picker remembers recent reactions: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: the picker Custom tab reacts with a workspace icon: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: the picker reacts with a brand icon shortcode: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: picker arrows move through options, Enter selects, and Escape returns focus: served mutant REJECTED (TimeoutError)
WS8bm behaviour source: test/system/message_toolbar_test.rb SHA256 d0e63bfb2c31db13f5786afd5ff28728f7dffead32468143fe3cb513598fdc12
WS8bm discrimination: message_actions_mobile: message action menu is a bottom sheet with touch-sized targets on phones: served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_actions_mobile: message action menu stays a floating popover on desktop: served mutant REJECTED (ERR_ASSERTION)
WS8bm behaviour source: test/system/message_actions_mobile_test.rb SHA256 4d930fbf29081825c2696f1be81ec062b0e6968031b93fef02d47b08818e856d
WS8bm discrimination: message_interactions: opens message actions from context menu and keyboard, and cancels a moving long press: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: a release click landing on the just-opened menu does not activate it: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: shows the message action menu as a bottom sheet on phones: served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_interactions: a duplicate delivery does not replace the message while its actions are open: served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_interactions: keeps newer typing through an asynchronous edit and leaves failures in edit mode: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: edits through the normal composer and restores the saved draft on cancel and success: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: replies with notify off and renders a tombstone when the target is deleted: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: copies message text and link and forwards to a server-provided thread destination: served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_interactions: forwarding twice in a row submits only once: served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_interactions: groups emoji reactions, updates the live count, and highlights the current user: served mutant REJECTED (TimeoutError)
WS8bm behaviour source: test/system/message_interactions_test.rb SHA256 4c867b252b0697ea694a9a3637b4242195638e261b62ccfe6e90ff21ea20d8db
WS8bm discrimination: code_highlighting: language fences highlight common code without changing its text: served mutant REJECTED (TimeoutError)
WS8bm discrimination: code_highlighting: unlabelled code is detected while text unknown languages and inline code stay literal: served mutant REJECTED (TimeoutError)
WS8bm discrimination: code_highlighting: search results highlight code on initial load and after returning to the channel: served mutant REJECTED (TimeoutError)
WS8bm discrimination: code_highlighting: code and copying remain available when the highlighter cannot load: served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: code_highlighting: editing a code block replaces its language colors and copied source: served mutant REJECTED (TimeoutError)
WS8bm behaviour source: test/system/code_highlighting_test.rb SHA256 bd8e2522a1fab8e0054b401d199acc3a942f4e4be866b7b1c011de650bed2014
WS8bm discrimination check: 30 named checks rejected their served mutants; 0 invalid or escaped
WS8bm fresh target removed
WS8bm fresh-check: committed inputs only; generated default/first_run seeds; workspace tests/doctests/clippy passed; requested browser groups and their mutants passed
```

The seeded workspace tests/doctests exclude third-party html5ever; clippy covers all workspace targets with warnings denied. Subsequent attribution/report-only edits do not change these tested source inputs.

The controller command runs the original pinned Ruby files; its counts are reference counts, separate from Rust attribution:

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

Python compilation, Node syntax checks for the changed modules and `git diff --check` also passed. The pinned system inventory was rechecked against `git show`: all 135 exact names and source hashes match. No new non-test production include exists outside crates because this slice changes only reference tools and plans. The fresh gate removed its own Cargo target. A final scan found zero scratch Cargo targets; `ss` showed no assigned browser listeners and Docker showed no parity-owner containers. Normal worktree build output is not a test input dependency.

```text
WS8bm cleanup: zero scratch Cargo targets remain; assigned 52020/52021/52022 listeners and parity-owner containers absent
```

## Exact remaining work

### Remaining WS8bm behaviour (26)

`test/system/code_highlighting_test.rb`

- thread code stays readable in both themes and scrolls within a narrow screen

`test/system/threads_test.rb`

- keeps the thread drawer usable on a phone and preserves the channel
- marks a joined thread read only while the conversation is visible
- opens a shared thread message link around an older post
- keeps an anchored older thread unread when a new reply arrives
- discusses a pull request from its card

`test/system/workspace_markdown_test.rb`

- Markdown replies and file attachments remain usable
- workspace follows the system theme and mobile navigation remains reachable

`test/system/composer_attach_menu_test.rb`

- From Google Drive starts the legacy picker flow
- From Google Drive starts the enhanced share flow when sharing is configured

`test/system/drive_attachments_test.rb`

- attach Drive files from the picker, send textless, and remove through edit
- edit a room message in the composer and remove one of two attachments
- attach a Drive file from the thread composer

`test/system/motion_test.rb`

- motion is off by default in the test environment
- mobile drawer animates in, lands in place, and returns focus with motion on
- member selection mode moves no rows and resizes nothing
- people directory bar shifts no rows when toggling
- people directory bar stays stuck while scrolling
- room menu measures at full scale when clamping to the viewport edge
- mobile drawer keeps the room list scroll position across close and reopen
- mobile drawer reveals a current room far down the list on first open
- mobile drawer reopens on the current room when it is already in view

`test/system/mobile_layout_test.rb`

- the profile page fits phone widths without scrolling sideways
- headers outside the workspace shell stay opaque over scrolled content
- headers outside the workspace shell never cover the page or its scrollbar
- pages outside the workspace shell show no drawer toggle that opens nothing

### Owner-blocked system declarations (6)

`test/system/threads_test.rb`

- tracks work, assigns an owner, completes and reopens it without losing the conversation
- shows work-thread guidance in the new-thread form and on the work page
- keeps the new-thread guidance usable on a phone
- shows work assignment activity to the owner and opens the exact thread

`test/system/message_list_a11y_test.rb`

- text fields stay at 16px on touch devices without changing the desktop look

`test/system/mobile_layout_test.rb`

- every drawer destination has one toggle that opens the drawer on itself

### Owner-blocked controller declarations (10)

All are in `test/controllers/channel_threads_controller_test.rb`, awaiting WS12 board/work writes and agent work services.

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
