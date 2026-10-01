# WS8bm behaviour parity — partial

Worker: GPT-6.1 Sol. Branch: `rust/ws8bm-messages-http`.
Pinned Rails `d7c7de9264c63015be398001d7a1094e7695a6db`, with approved #163 application layout/assets drift only. No pixels or screenshots.

Current mapped system scope: **42 passed / 87 remaining / 6 owner-blocked**, out of 135 exact pinned declarations. This run adds 31 accepted flows to the 12 in frozen PR #182, then returns one earlier flow to the backlog after its stronger fresh-clone assertion fails. Net pass credit increases by 30. These are scoped Playwright equivalents checked on both real applications, not execution of the original Ruby system files. Original source hashes and exact names are verified by the inventory.

Controller attribution remains **146/156**; the only ten unmatched declarations require WS12. The owned controller declaration gate remains met. Behaviour/system sign-off is partial. WS12 activity/work/board routes retain authorized, flagged 501 seams. This worker made no writes to frozen PR #182. Remote readback found its independent review-fix head `8cc1e939`; those production fixes are not in this working branch. They must be retained when the lead merges the PR and this continuation. No reviewer finding was relayed to this worker as a new task.

## Pushed slices

- `976be8ff52aed2ae479d241b357b7622476d711b`: 20 message-list navigation/focus/live-region regressions and four Markdown composer/reply/upload/mention/recovery/in-flight-draft flows. Adds a self-contained pinned-Rails fixture builder and served-implementation mutation checks. Count after this slice: 36 / 95 / 4.
- `91e1f17282d9b31ac6b661a647bf56d08b35bc81`: actual search empty/operator/pagination navigation and Markdown forwarding, with exact rendered ActionText snapshot persistence on both databases. Count after this slice: 38 / 93 / 4.
- `16fc4e3c27d9a255fd8035235dca92f7986b889a`: five unread/divider/jump/around/mark-unread flows. Two previously deferred cases are correctly reclassified as WS12-blocked because their full assertions require board/work or activity pages. Count after this slice: 43 / 86 / 6.

- `ace859e2` / `d7b2c234`: retain thread-create failure diagnostics, including real `.json` responses and actual pane/input state. No acceptance assertion or wait was weakened.
- `a766e287`: restore the exact open-state assertion omitted from the pinned Rails `open_threads` helper before selecting New thread; assert that the completed name survives until submission. An earlier clean batch exposed two thread names lost before submission. No refilling, sleep or increased timeout.
- `206add3f`: retain the unresolved existing stray-reentry flow as deferred, with its prior evidence and exact failure reason. Final inventory: 42 / 87 / 6.

Only reference tools and the named system ledger changed in these slices. Application source, response goldens, shell ownership, presenter inputs and production includes are unchanged from `9883d3fa`.

## Stable room seam

The shell/composer contract remains [ws8bm-integration.md](ws8bm-integration.md). `Presenter::messages(&records)` and `messages::Index { ctx, messages }` remain the scrolling-page entry point. The real shell mounts `Presenter::room_message_list(&records, divider.message_id, divider.count)` verbatim inside the shared fragment-cache scope. The viewer divider remains outside cached message fragments.

The shell supplies request `ViewContext` with verified origin/viewer/assets/signer/CSRF scope, room ID/kind/name, ordered command names, Drive flow, request-owned `PendingTemplate`, and the actual schedule button. Root composers pass no thread; the pane passes its thread ID at `render_thread_schedule_control`. Keep the populated-list boundary, pending-template newline and footer indentation documented in the integration contract. The merged provider renderers remain in use; no provider markup was copied into this slice.

## Behaviour scope and discrimination

The 20 message-list checks cover roving tab stops, arrow/Home/End movement, Turbo/direct-DOM replacement focus, removal focus handoff, refresh/late-autofocus protection, ContextMenu/Escape, edit failure feedback, pending metadata sharing/routing, cache-close cleanup, the live log, actual before-pagination, edit replacement and optimistic-send replacement. Synthetic streams/fetch gates reproduce the pinned deterministic regression bodies; they are not described as server broadcast checks. Pagination, edit and send use real HTTP/Cable on both apps.

The four new Markdown flows cover real file upload as a reply with author notification disabled; mention selection without premature send followed by actual recipient mention delivery; rejected draft recovery/correction; and preserving a newer draft while the submitted source is in flight. The attachment filename, reply flags, 47 bytes and storage contents are checked in both databases. Mention/source/count assertions also check both databases. The recovery check clears the still-present editor before Restore draft, so a broken recovery method cannot pass by retaining the old text.

Search navigates the empty state and `NOT` operator, then appends older results from 40 to 42 with no duplicate IDs. Forwarding uses the actual actions menu/destination dialog and checks tables/code on both viewers. Rails stores NULL `markdown_source` and `forwarded_markdown=1` on the copy; both applications must persist the exact original rendered ActionText body, rather than a copied Markdown source.

All five unread flows cover the first divider boundary, initial scrolling, jump-pill landing, an older off-page cursor through the actual around link, and Mark unread through the public endpoint/menu. The seven-unread fixture adds twelve already-read history posts and uses a 700px viewport to make scrolling necessary. The same pinned upper-half assertion remains unchanged. The short original fixture alone also passed with initial scrolling disabled; it was insufficient discrimination. Both apps pass the stronger setup and the disabled-scroll implementation is rejected.

Each of the 31 new cases must reject its own deliberately broken served asset or HTTP response. The check refuses to credit startup failure, an unapplied mutation, an unexpected adapter error or an actual network failure as mutant rejection. Failure-only mutation runs use short waits; positive acceptance waits and the pin's 50ms focus window, 500ms reconnect, 30ms quiet-after-insertion threshold and scroll bounds are unchanged. No concurrency or timing assertion was weakened.

The first seventeen read-only list cases share one server fixture, with fresh browser contexts per case and saved message rows verified unchanged afterwards. Room reads retain their ordinary membership/read side effects. All writing/history/unread/search/forward cases get independent database/storage/server copies. Pinned Rails creates the extra fixture rows, then identical bytes are copied to the candidate. Seeds, binary, npm/Chromium and metadata are generated by the committed runner; no test requires an old `.scratch` or Cargo target.

## Current verification

The full Rust gate ran `python3 rust/reference-tools/messaging/fresh-check.py` from committed input `16fc4e3c27d9a255fd8035235dca92f7986b889a`. It created a new tracked-source clone with no prior scratch or Cargo target, generated both seeds, and ran locked metadata, the seeded workspace suite/doctests (excluding the third-party html5ever test crate), and all-target clippy with warnings denied. Eight test threads, two build jobs, pinned media/toolchain and the unchanged shared rustc throttle were used. **58 raw summaries: 3,353 passed / zero failed / 12 inherited ignores.** All current application/crate source, Cargo manifests and lockfile are unchanged from that tested input; subsequent commits change tools, ledger and this report only. No new production includes were added.

The inherited `controllers::rooms::directs_rails_cases::the_last_member_out_destroys_the_group` passed in this run. Its known queue-observation race remains with the separate worker; no queue-observation test, implementation, concurrency or timing threshold was changed here. The three application ignores remain the reference recorder, Node/gateway-only suite and push-latency measurement. Other ignores are the existing reference export/rollback/ACME setup probes and two doctests. None was added in this run.

Raw fresh-clone output (all test/doctest summary lines):

```text
WS8bm fresh checkout: 16fc4e3c27d9a255fd8035235dca92f7986b889a; no pre-existing scratch or Cargo target; /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-fresh-quf6uqai
WS8bm fresh concurrency: eight test threads; two build jobs; no timing threshold changes
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 5m 46s
test result: ok. 1700 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 1640.57s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.62s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.90s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 951 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 140.51s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.47s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.96s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.03s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.64s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.22s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.43s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.77s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.29s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.84s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.30s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.63s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.56s
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 44s
WS8bm fresh target removed
WS8bm fresh-check: committed inputs only; generated default/first_run seeds; workspace tests/doctests/clippy passed
```

From that same clone, after the Rust helper removed its target, `python3 rust/reference-tools/messaging/behavior-check.py --keep-going` rebuilt the binary, seeds and npm/Chromium inputs and attempted all 43 mapped checks. The first batch at `16fc4e3c` exited 1 with two existing thread-create timeouts:

```text
WS8bm behaviour check: 41 named cases passed on Rails and Rust; 2 failed; no pixel checks
WS8bm failed named checks:
threads: browses active and closed threads and can join or leave a closed one
threads: renders untrusted thread metadata as text
```

Diagnostics were committed in `ace859e2` / `d7b2c234` and fast-forwarded into the clean clone. The unchanged acceptance commands were rerun there individually:

- `python3 rust/reference-tools/messaging/behavior-check.py threads --case 'browses active and closed threads and can join or leave a closed one' --keep-going`: exit 0, including the closure, memberships and raw source rows.
- `python3 rust/reference-tools/messaging/behavior-check.py threads --case 'renders untrusted thread metadata as text' --keep-going`: exit 1 on pinned Rails. The entered name was cleared; Rails returned HTTP 201 with `threadName: 'New thread'`. The same focused command at `a766e287`, after restoring the pinned helper's omitted panel-open state assertion and adding a stronger pre-submit assertion, exited 0 on both apps. No production behavior was changed.

Raw diagnostic summaries, in order:

```text
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
WS8bm behaviour check: 0 named cases passed on Rails and Rust; 1 failed; no pixel checks
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

The full `--keep-going` batch was rerun once because that helper changed. Final input: `a766e287`; same clean checkout and unchanged application source. It exited 1: all 31 new flows passed, but the stronger assertion found the existing stray-reentry name field empty before submission. That assertion's trace did not identify which app's iteration failed; it is not claimed to be a Rust-only or Rails-only defect. Future failures label the responding app in `206add3f`. The earlier untrusted-name diagnostic was specifically Rails, as above. No network error appeared in these creation traces. The underlying cause of the cleared draft is unresolved. No further retry or refill was used to obtain a green batch. The stray-reentry declaration is explicitly **deferred**, and its earlier evidence is retained separately. This is **not a green browser/system gate**.

Raw final browser acceptance lines:

```text
WS8bm behaviour: sending_messages: sending messages between two users: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: sending_messages: editing messages: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: sending_messages: deleting messages: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour source: test/system/sending_messages_test.rb SHA256 4c637558cc041e470c6378f43e67d9d5b5b483ed0aae746c50eb64221027d116
WS8bm behaviour: workspace_markdown: Markdown messages reach other users and editing preserves the original source: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: workspace_markdown: desktop keyboard composition keeps line breaks and sends once after composition ends: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: workspace_markdown: untrusted markup stays inert in the delivered message: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: workspace_markdown: Markdown replies and file attachments remain usable: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: workspace_markdown: mention suggestions select a room member without sending the unfinished message: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: workspace_markdown: a rejected message can be recovered corrected and sent: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: workspace_markdown: sending preserves the submitted source and a newer draft: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour source: test/system/workspace_markdown_test.rb SHA256 b7b47897db51c2603fb733e90ecc7d2bf611a0f6d600926550ac80732e7b61fd
WS8bm behaviour: threads: creates a thread from a channel message and keeps the channel draft separate: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: threads: the thread root counts its replies live and hides the count when none remain: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: threads: browses active and closed threads and can join or leave a closed one: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: threads: rejects an external thread deep link before fetching it: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: threads: renders untrusted thread metadata as text: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour source: test/system/threads_test.rb SHA256 e927613f9e78905aa072e0530339678b8443166b6f6d3c052d22cb3b797faafc
WS8bm behaviour: message_list_a11y: the message list is a single tab stop with a roving tabindex: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_list_a11y: arrow keys move between messages: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_list_a11y: a stream replacing the focused message keeps focus and the tab stop on its replacement: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_list_a11y: a stream replacing the tab-stop message while focus is elsewhere keeps the tab stop on the replacement: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_list_a11y: a direct DOM swap of the focused message keeps focus and the tab stop on its replacement: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_list_a11y: deleting the focused message moves focus to the surviving tab stop: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_list_a11y: deleting an older focused message hands focus to its neighbour, not the newest: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_list_a11y: a focus move during a stream render survives Turbo's focus restore: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_list_a11y: a no-change room refresh does not yank focus back to the composer: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_list_a11y: the ContextMenu key opens the shared menu and Escape returns focus: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_list_a11y: a late composer autofocus does not steal focus from a message: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_list_a11y: up arrow from an empty composer still edits my last message: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_list_a11y: up-arrow-to-edit shows an error when the actions endpoint fails: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_list_a11y: forward reuses the menu-open metadata request instead of fetching again: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_list_a11y: a menu opened while an action waits does not redirect the pending action: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_list_a11y: the menu closes before Turbo caches the page: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_list_a11y: the main message list is a live log: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_list_a11y: paginated history stays quiet past the insert, then the live region comes back: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_list_a11y: an edit replacement is not announced as an addition: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_list_a11y: an own message is not re-announced when its broadcast replaces the pending copy: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour source: test/system/message_list_a11y_test.rb SHA256 9c17b5c9dccb9b10eb3c56e52aed5583c43e0a47166218ecfd9c0ec2bdd169fa
WS8bm behaviour: search_forward_edit: search tolerates operators, shows an empty state and pages older results: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: search_forward_edit: forwarded Markdown keeps tables and code blocks: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour source: test/system/search_forward_edit_test.rb SHA256 60c7ecfe60cd0a51ec1235f547795c2400e8f02a830ef0991af9dea833d89366
WS8bm behaviour: unread_divider: few unread render the divider above the first new message and keep the bottom scroll: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: unread_divider: many unread scroll the room to the divider: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: unread_divider: the jump pill shows while the divider is off-screen and returns to it: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: unread_divider: unread older than the last page keeps the last page and the pill links to the first unread: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: unread_divider: mark unread from the message menu points the divider at that message: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour source: test/system/unread_divider_test.rb SHA256 006147f6e9ffb9066a4e82e8b50e841f150ce7c079047833efcf560f93546a1e
WS8bm behaviour check: 42 named cases passed on Rails and Rust; 1 failed; no pixel checks
WS8bm failed named checks:
threads: a stray create re-entry does not wipe the half-filled thread name
```

`python3 rust/reference-tools/messaging/behavior-check.py --negative --keep-going` ran all 31 new served mutants from the clean clone at `16fc4e3c`, with unchanged new-flow acceptance code and fixtures. It exited 0. Subsequent helper changes apply only to the existing thread cases, which have no new mutation in this selection. Every selected mutant reached the actual case, was applied, had no recorded network failure, and was rejected. Raw discrimination lines:

```text
WS8bm discrimination: workspace_markdown: Markdown replies and file attachments remain usable: served mutant REJECTED (TimeoutError)
WS8bm discrimination: workspace_markdown: mention suggestions select a room member without sending the unfinished message: served mutant REJECTED (TimeoutError)
WS8bm discrimination: workspace_markdown: a rejected message can be recovered corrected and sent: served mutant REJECTED (TimeoutError)
WS8bm discrimination: workspace_markdown: sending preserves the submitted source and a newer draft: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_list_a11y: the message list is a single tab stop with a roving tabindex: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_list_a11y: arrow keys move between messages: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_list_a11y: a stream replacing the focused message keeps focus and the tab stop on its replacement: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_list_a11y: a stream replacing the tab-stop message while focus is elsewhere keeps the tab stop on the replacement: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_list_a11y: a direct DOM swap of the focused message keeps focus and the tab stop on its replacement: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_list_a11y: deleting the focused message moves focus to the surviving tab stop: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_list_a11y: deleting an older focused message hands focus to its neighbour, not the newest: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_list_a11y: a focus move during a stream render survives Turbo's focus restore: served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_list_a11y: a no-change room refresh does not yank focus back to the composer: served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_list_a11y: the ContextMenu key opens the shared menu and Escape returns focus: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_list_a11y: a late composer autofocus does not steal focus from a message: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_list_a11y: up arrow from an empty composer still edits my last message: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_list_a11y: up-arrow-to-edit shows an error when the actions endpoint fails: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_list_a11y: forward reuses the menu-open metadata request instead of fetching again: served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_list_a11y: a menu opened while an action waits does not redirect the pending action: served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_list_a11y: the menu closes before Turbo caches the page: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_list_a11y: the main message list is a live log: served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_list_a11y: paginated history stays quiet past the insert, then the live region comes back: served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_list_a11y: an edit replacement is not announced as an addition: served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_list_a11y: an own message is not re-announced when its broadcast replaces the pending copy: served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: search_forward_edit: search tolerates operators, shows an empty state and pages older results: served mutant REJECTED (TimeoutError)
WS8bm discrimination: search_forward_edit: forwarded Markdown keeps tables and code blocks: served mutant REJECTED (TimeoutError)
WS8bm discrimination: unread_divider: few unread render the divider above the first new message and keep the bottom scroll: served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: unread_divider: many unread scroll the room to the divider: served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: unread_divider: the jump pill shows while the divider is off-screen and returns to it: served mutant REJECTED (TimeoutError)
WS8bm discrimination: unread_divider: unread older than the last page keeps the last page and the pill links to the first unread: served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: unread_divider: mark unread from the message menu points the divider at that message: served mutant REJECTED (TimeoutError)
WS8bm discrimination check: 31 named checks rejected their served mutants; 0 invalid or escaped
```

`python3 rust/reference-tools/messaging/deferred-system-inventory.py` was rerun after updating the clone to committed `206add3f`. It checks every exact pinned name/source hash and the pass-evidence invariant. Raw inventory output:

```text
test/system/boosting_messages_test.rb: 4 named declarations; 0 mapped behaviour passes; 4 deferred; 0 WS12 blocked
test/system/code_highlighting_test.rb: 6 named declarations; 0 mapped behaviour passes; 6 deferred; 0 WS12 blocked
test/system/sending_messages_test.rb: 3 named declarations; 3 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/threads_test.rb: 15 named declarations; 5 mapped behaviour passes; 6 deferred; 4 WS12 blocked
test/system/workspace_markdown_test.rb: 8 named declarations; 7 mapped behaviour passes; 1 deferred; 0 WS12 blocked
test/system/composer_test.rb: 11 named declarations; 0 mapped behaviour passes; 11 deferred; 0 WS12 blocked
test/system/composer_attach_menu_test.rb: 9 named declarations; 0 mapped behaviour passes; 9 deferred; 0 WS12 blocked
test/system/message_interactions_test.rb: 10 named declarations; 0 mapped behaviour passes; 10 deferred; 0 WS12 blocked
test/system/message_actions_mobile_test.rb: 2 named declarations; 0 mapped behaviour passes; 2 deferred; 0 WS12 blocked
test/system/message_toolbar_test.rb: 13 named declarations; 0 mapped behaviour passes; 13 deferred; 0 WS12 blocked
test/system/message_list_a11y_test.rb: 29 named declarations; 20 mapped behaviour passes; 8 deferred; 1 WS12 blocked
test/system/drive_attachments_test.rb: 3 named declarations; 0 mapped behaviour passes; 3 deferred; 0 WS12 blocked
test/system/unread_divider_test.rb: 5 named declarations; 5 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/search_forward_edit_test.rb: 3 named declarations; 2 mapped behaviour passes; 1 deferred; 0 WS12 blocked
test/system/motion_test.rb: 9 named declarations; 0 mapped behaviour passes; 9 deferred; 0 WS12 blocked
test/system/mobile_layout_test.rb: 5 named declarations; 0 mapped behaviour passes; 4 deferred; 1 WS12 blocked
WS8bm system inventory: 135 named declarations; 42 mapped behaviour passes; 87 deferred; 6 WS12 blocked; no pixel checks
```

The Rust helper removed its generated target. The browser runner demonstrably rebuilt it from that removed-target state; its generated target was then deleted after both acceptance/discrimination runs. Cleanup verification found no scratch targets; the normal worktree target was untouched:

```text
WS8bm cleanup: 0 scratch targets remain; normal worktree target untouched
```

The twelve inherited ignores and the failed browser traces above remain visible. No screenshots, pixel diffs, reduced test concurrency or widened acceptance timing threshold were used. The Python model server was not touched.


## Exact remaining scope

| Pinned system file | Named | Passed | Remaining | WS12 blocked |
|---|---:|---:|---:|---:|
| `boosting_messages_test.rb` | 4 | 0 | 4 | 0 |
| `code_highlighting_test.rb` | 6 | 0 | 6 | 0 |
| `sending_messages_test.rb` | 3 | 3 | 0 | 0 |
| `threads_test.rb` | 15 | 5 | 6 | 4 |
| `workspace_markdown_test.rb` | 8 | 7 | 1 | 0 |
| `composer_test.rb` | 11 | 0 | 11 | 0 |
| `composer_attach_menu_test.rb` | 9 | 0 | 9 | 0 |
| `message_interactions_test.rb` | 10 | 0 | 10 | 0 |
| `message_actions_mobile_test.rb` | 2 | 0 | 2 | 0 |
| `message_toolbar_test.rb` | 13 | 0 | 13 | 0 |
| `message_list_a11y_test.rb` | 29 | 20 | 8 | 1 |
| `drive_attachments_test.rb` | 3 | 0 | 3 | 0 |
| `unread_divider_test.rb` | 5 | 5 | 0 | 0 |
| `search_forward_edit_test.rb` | 3 | 2 | 1 | 0 |
| `motion_test.rb` | 9 | 0 | 9 | 0 |
| `mobile_layout_test.rb` | 5 | 0 | 4 | 1 |
| Total | 135 | 42 | 87 | 6 |


The inventory remains `ws8bm-system-cases.json`, with every exact pinned name, source hash, status and scoped evidence. The following complete list is still unaccepted:


## Remaining system declarations (87)

### test/system/boosting_messages_test.rb

- boosting a message
- deleting a boost
- message update preserves the input state
- boost by another user preserves the input state

### test/system/code_highlighting_test.rb

- language fences highlight common code without changing its text
- unlabelled code is detected while text unknown languages and inline code stay literal
- search results highlight code on initial load and after returning to the channel
- code and copying remain available when the highlighter cannot load
- editing a code block replaces its language colors and copied source
- thread code stays readable in both themes and scrolls within a narrow screen

### test/system/threads_test.rb

- a stray create re-entry does not wipe the half-filled thread name — The final fresh-clone browser batch failed the stronger pre-submit name assertion: the field was empty after the pinned same-context re-entry. Earlier runs passed, but current acceptance is unresolved. Preserve the assertion and 30-second waits; do not refill the draft or change timing to get a pass.
- keeps the thread drawer usable on a phone and preserves the channel
- marks a joined thread read only while the conversation is visible
- opens a shared thread message link around an older post
- keeps an anchored older thread unread when a new reply arrives
- discusses a pull request from its card

### test/system/workspace_markdown_test.rb

- workspace follows the system theme and mobile navigation remains reachable

### test/system/composer_test.rb

- blurring an open autocomplete does not leave a zombie that swallows Enter
- a stale icon response does not poison the suggestion commit
- mention queries are URL-encoded
- composer autocomplete exposes combobox semantics over a polite listbox
- composing text does not commit a suggestion or send the message
- clicking a reply preview scrolls to the loaded message instead of navigating
- clicking a reply preview falls back to the permalink when the target is not loaded
- deleting a replied-to message turns open reply previews into a tombstone
- two typers with the same name do not merge
- composer drafts persist per room and clear on send
- thread drafts persist per thread without touching the channel draft

### test/system/composer_attach_menu_test.rb

- + shows both attach options when Drive is available
- From this device triggers the file input
- From Google Drive starts the legacy picker flow
- From Google Drive starts the enhanced share flow when sharing is configured
- + opens the file picker directly without Drive
- arrow keys move between items and Escape closes back onto +
- a tap outside closes the menu
- phone layout keeps the menu above the composer with no horizontal overflow
- device files, paste, and drag-and-drop still preview uploads

### test/system/message_interactions_test.rb

- opens message actions from context menu and keyboard, and cancels a moving long press
- a release click landing on the just-opened menu does not activate it
- shows the message action menu as a bottom sheet on phones
- edits through the normal composer and restores the saved draft on cancel and success
- a duplicate delivery does not replace the message while its actions are open
- keeps newer typing through an asynchronous edit and leaves failures in edit mode
- replies with notify off and renders a tombstone when the target is deleted
- copies message text and link and forwards to a server-provided thread destination
- forwarding twice in a row submits only once
- groups emoji reactions, updates the live count, and highlights the current user

### test/system/message_actions_mobile_test.rb

- message action menu is a bottom sheet with touch-sized targets on phones
- message action menu stays a floating popover on desktop

### test/system/message_toolbar_test.rb

- the toolbar stays hidden until hover or focus and labels every action
- quick-react creates a boost from the toolbar
- reply and thread buttons drive the composer and the thread panel
- the more button opens the shared menu for its message
- keyboard users reach the toolbar from a focused message
- the emoji picker searches and reacts
- the picker shows category tabs and switches between them
- the picker loads its emoji data only on first open
- the picker remembers recent reactions
- the picker Custom tab reacts with a workspace icon
- the picker reacts with a brand icon shortcode
- picker arrows move through options, Enter selects, and Escape returns focus
- picker tabs move with arrow keys and switch the grid

### test/system/message_list_a11y_test.rb

- search results keep their menus and focusability
- the message-list top padding does not apply to search results
- the standalone thread page keeps menus and focusability
- the standalone message page keeps its menu and focusability
- the viewport allows pinch zoom
- profile message and ban buttons have accessible names
- flash persists its 5-second minimum under reduced motion
- flash dismisses on demand under reduced motion

### test/system/drive_attachments_test.rb

- attach Drive files from the picker, send textless, and remove through edit
- edit a room message in the composer and remove one of two attachments
- attach a Drive file from the thread composer

### test/system/search_forward_edit_test.rb

- editing to add a URL renders its card live and the edited marker on load

### test/system/motion_test.rb

- motion is off by default in the test environment
- mobile drawer animates in, lands in place, and returns focus with motion on
- member selection mode moves no rows and resizes nothing
- people directory bar shifts no rows when toggling
- people directory bar stays stuck while scrolling
- room menu measures at full scale when clamping to the viewport edge
- mobile drawer keeps the room list scroll position across close and reopen
- mobile drawer reveals a current room far down the list on first open
- mobile drawer reopens on the current room when it is already in view

### test/system/mobile_layout_test.rb

- the profile page fits phone widths without scrolling sideways
- headers outside the workspace shell stay opaque over scrolled content
- headers outside the workspace shell never cover the page or its scrollbar
- pages outside the workspace shell show no drawer toggle that opens nothing
## Owner-blocked system declarations (six WS12)

### test/system/threads_test.rb

- tracks work, assigns an owner, completes and reopens it without losing the conversation
- shows work-thread guidance in the new-thread form and on the work page
- keeps the new-thread guidance usable on a phone
- shows work assignment activity to the owner and opens the exact thread

### test/system/message_list_a11y_test.rb

- text fields stay at 16px on touch devices without changing the desktop look — Pinned flow requires the authorized, flagged WS12 board/work page or activity inbox; WS12 has not started. No partial pass credited.

### test/system/mobile_layout_test.rb

- every drawer destination has one toggle that opens the drawer on itself — Pinned flow requires the authorized, flagged WS12 board/work page or activity inbox; WS12 has not started. No partial pass credited.

## Owner-blocked controller declarations (unchanged: ten WS12)

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

Case attribution remains in `ws8bm-controller-cases.md`. No one-to-one controller test count is claimed. Keyboard shortcuts (14) and CSP (5) remain assigned to WS8b-r; timezone detection (2) to WS8br2, outside this 135-flow ledger.

## Merge handoff

All owned controller declarations are attributed; only ten WS12 declarations remain owner-blocked. The current system remainder is exactly 87 deferred declarations and six WS12-blocked declarations, enumerated above. The unresolved stray-reentry flow is part of those 87 and remains in the default runner so its failure is observable. Next user-impact priority is the remaining list/standalone/search-menu flows, composer/attach-menu/Drive cases and the edited URL-card flow, followed by reactions/toolbars and the rest of the listed thread/mobile/theme/motion cases. Preserve WS12's flagged activity/work/board seams.

This continuation has not merged the independent PR #182 review-fix branch. Remote readback found `8cc1e939` (cached freshness/reply-source keys and Rails reaction classification, following that branch's own main merge). Those fixes must be retained when the lead merges PR #182 and this branch. This worker made no changes or pushes to the frozen branch. The reviewed branch's own report is separate from this run's verification; no results from it are substituted here.

Partial delivery: three new-flow slices, diagnostic/pinned-helper corrections and the honest backlog update are pushed on `rust/ws8bm-messages-http`. Public list/presenter/composer seams remain stable. Behaviour system sign-off is still pending the listed declarations and the unresolved browser failure; no blanket E2E sign-off is claimed.
