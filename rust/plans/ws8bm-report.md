# WS8bm PR #189 opacity correction — review-ready partial slice

Worker: GPT-6.1 Sol. Branch: `rust/ws8bm-messages-http`.
Reference: Rails `d7c7de9264c63015be398001d7a1094e7695a6db`, plus the approved drift in wave4/_common.md. Behaviour and response parity only; no pixel checks.

The remaining P2 is corrected: all messaging visibility states and counts use the **unmodified pinned Selenium 4.35.0 visibility atom**, including effective opacity through ancestors. This is a reference-tools-only correction; no production Rust or Rails code changed. Full system sign-off remains partial. The received inventory remains **102 passed / 27 deferred / 6 owner-blocked** out of 135 exact system declarations, and **146/156 controller declarations**, with the ten controller deferrals owned by WS12. This correction adds no declaration credit. The release-click case passed in this fresh run, but its received deferral remains explicit because of the two earlier reference failures.

**Duplicate-delivery limitation:** the check injects Turbo markup in the browser using the actual mounted row and render queue. It verifies the browser render guard and object identity. It **does not verify server-originated redelivery**; the original Rails case calls `message.broadcast_create`.

## Correction and failing-first evidence

`behavior-visibility.mjs` installs the pinned gem's `isDisplayed.js` atom and calls it with `ignoreOpacity=false`. The source is vendored verbatim with the gem's LICENSE and NOTICE and its provenance in `selenium/UPSTREAM.md`. Each paired run compares the vendored bytes with the atom inside the pinned reference image before running checks. Its SHA256 is `ae26018c01cd27448b250f8e55a094cbfcd2e2cbbe171c78aaa906e1b5c3ed7c`.

All messaging assertion modules share the helper for visible/hidden waits, visible counts and visible element selection. The helper retains the original deadlines for the thirty reviewed cases: default two seconds, delivery ten seconds and highlighting twenty seconds. The 700 ms hold is unchanged. Explicit DOM-presence checks and security assertions originally using `visible: :all` retain their DOM semantics; JavaScript geometry probes retain the original Ruby tests' own rectangle filters. Tests cover ancestor opacity, partial opacity, clipping, closed details, shadow DOM, navigation and visible/hidden/attached/detached states.

Astra's read-only probe at `/home/riels/.cache/rust-port/ws8bmbr/smartfire/.scratch/rereview-cb211f02/opacity-escape.log` reports DOM 8 / pinned Selenium visible 7 / old predicate visible 8 / Playwright visible 8 for transparent Clapping. Before changing `cb211f02`'s assertion modules, added and ran two served CSS variants over those unchanged checks. Both escaped on both applications:

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_interactions --case 'opens message actions from context menu and keyboard, and cancels a moving long press' --mutant transparent-clapping
python3 rust/reference-tools/messaging/behavior-check.py message_interactions --case 'opens message actions from context menu and keyboard, and cancels a moving long press' --mutant transparent-reaction-ancestor
```

```text
WS8bm review escape: message_interactions: opens message actions from context menu and keyboard, and cancels a moving long press: transparent-clapping: Rails ACCEPTED
WS8bm review escape: message_interactions: opens message actions from context menu and keyboard, and cancels a moving long press: transparent-clapping: Rust ACCEPTED
WS8bm review escape check: 1 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
WS8bm review escape: message_interactions: opens message actions from context menu and keyboard, and cancels a moving long press: transparent-reaction-ancestor: Rails ACCEPTED
WS8bm review escape: message_interactions: opens message actions from context menu and keyboard, and cancels a moving long press: transparent-reaction-ancestor: Rust ACCEPTED
WS8bm review escape check: 1 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
```

The Clapping variant injects `.message__quick-reaction[title="Clapping"] { opacity: 0 !important; }`. The ancestor variant injects `.message__quick-reactions { opacity: 0 !important; }`. Neither removes any reaction DOM element. The full corrected discrimination run includes these plus all thirty original mutants and all five earlier review variants. Valid startup, an applied served mutation, no network failure and an assertion rejection on both apps are still required.

## Current verification

Fresh source clone: `.scratch/ws8bm-opacity-fresh`, at `3296f5779938351a51dcc890f872556ccc2e8c50` for the requested thirty checks and full discrimination set. It started without seeds, node_modules or a target directory. The harness generated its own seeds, installed the declared Node/browser dependencies, verified the pinned oracle image and atom, and built its own app binary. Shared-helper changes are commits `7e804405a` and `3296f5779`; the additional typing-selector change is recorded below. Rust workspace tests and clippy were not rerun for this reference-tools-only correction, as the latest request permits. The configured machine-wide rustc wrapper remains in use; no concurrency or native timing thresholds changed. No queue-observation race was repaired.

Ran from that fresh source root, with `CARGO_TARGET_DIR` set to its own `rust/target` for the paired commands:

```sh
node --test rust/reference-tools/messaging/behavior-visibility.test.mjs
python3 rust/reference-tools/messaging/behavior-check.py message_toolbar message_actions_mobile message_interactions code_highlighting --keep-going
python3 rust/reference-tools/messaging/behavior-check.py message_toolbar message_actions_mobile message_interactions code_highlighting --negative --keep-going
```

```text
ℹ tests 2
ℹ suites 0
ℹ pass 2
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1117.637684
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 13s
WS8bm visibility atom: pinned Selenium SHA256 ae26018c01cd27448b250f8e55a094cbfcd2e2cbbe171c78aaa906e1b5c3ed7c verified
WS8bm behaviour check: 30 named cases passed on Rails and Rust; 0 failed; no pixel checks
WS8bm delayed-write probe: Rails: 10037 ms observed; actual write completed: false
WS8bm delayed-write probe: Rust: 10021 ms observed; actual write completed: false
WS8bm discrimination: message_interactions: opens message actions from context menu and keyboard, and cancels a moving long press: transparent-clapping: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: opens message actions from context menu and keyboard, and cancels a moving long press: transparent-clapping: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: opens message actions from context menu and keyboard, and cancels a moving long press: transparent-reaction-ancestor: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: opens message actions from context menu and keyboard, and cancels a moving long press: transparent-reaction-ancestor: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination check: 37 served mutants rejected on Rails and Rust across 30 named checks; 0 invalid or escaped
```

The additional legacy typing selector update in `9d963ed80eaacc06c470ff160769beebb73a6b97` uses the same visibility helper and the original ten-second waits. The fresh clone was advanced to that commit, and its affected case was rerun:

```sh
python3 rust/reference-tools/messaging/behavior-check.py composer --case 'two typers with the same name do not merge'
```

```text
WS8bm browser flow: composer: two typers with the same name do not merge: Rails PASS; Rust PASS
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

Independent inventory verification re-read all pinned declaration names and source hashes and required evidence for every credited pass. Deferred diagnostics remain diagnostics, not declaration credit. Earlier full workspace results and all received attribution are retained in the [received report at cb211f022](https://github.com/Smart-Data-Ohio/smartfire/blob/cb211f022eae9eece7c944920701fe14eb19446b/rust/plans/ws8bm-report.md); those historical commands are not presented as rerun results here.

```text
WS8bm pinned system inventory: 102 passed / 27 remaining / 6 owner-blocked; 135 exact names and source hashes verified
WS8bm cleanup: zero scratch Cargo targets remain; assigned 52020/52021/52022 listeners and parity-owner containers absent
```

## Stable integration contract

The shell seam in [ws8bm-integration.md](ws8bm-integration.md) is unchanged: `Presenter::messages(&records)`, `messages::Index { ctx, messages }`, and `Presenter::room_message_list(&records, divider.message_id, divider.count)`. The shell passes verified request origin/viewer/assets/signer/CSRF context, selected records and divider facts. The composer receives room kind/name/ID, ordered commands, Drive flow, thread scope, the real scheduled-message child and request-owned PendingTemplate. Mount list/template boundaries verbatim. Real WS15g GitHub rendering and merged WS14e/M2 children remain in use. There is no new cross-owner production touch.

## Exact remaining work

### Remaining WS8bm behaviour (27)

`test/system/message_interactions_test.rb`

- a release click landing on the just-opened menu does not activate it — two earlier reference failures; passed this fresh run with the pinned atom and original assertion/deadlines; received deferral retained pending reference reliability sign-off, with no new declaration credit

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
