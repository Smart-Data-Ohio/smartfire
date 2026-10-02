# WS8bm PR #189 hidden-scope and runner corrections — partial port

Worker: GPT-6.1 Sol. Branch: `rust/ws8bm-messages-http`.
Reference: Rails `d7c7de9264c63015be398001d7a1094e7695a6db`, plus the approved drift in wave4/_common.md. Behaviour and byte-response parity only; no pixel checks.

Addressed the two over-scoped hidden assertions and the negative-job scheduler. The live-log attributes now use an attached-only CSS selector at the original ten-second wait, before any unrelated visible-message setup. The More button's expanded attribute uses an attached-only CSS selector at Capybara's two-second default. The sweep also restored the toolbar's DOM-only lookup before hover and the thread-menu attribute's DOM-only wait after its independent visible-row find. Three served CSS acceptance proofs cover both Astra variants and a fully hidden log; they are excluded from the negative mutation registry.

`behavior_mutation_jobs.py` plans only registered negative variants, preserving read-only batching where a default exists and independent fixtures for other variants. An explicit nonexistent variant raises rather than reporting an empty green run. Six unit tests cover named-only, mixed default/named batching, explicit variant selection, absent explicit variants, positive/allowed-hidden scopes and complete negative scope sets. The actual named-only CLI regression also runs through both live applications.

The URL-card port differed from the pinned original: generic `send` waited for body text before the Loading post assertion, and then required that fleeting state on a second viewer. Rails :74–78 clicks Send and immediately asserts Loading post on the author only. The port now follows that sequence, retaining the fifteen-second loading requirement; :73's visible context label uses ten seconds, :83's reloaded card uses ten. The pinned gem reports default wait 2 seconds and retry interval 0.01 seconds. Astra's 63–74 ms observation explains the old port timing risk; it does not prove the original Ruby case itself flakes. The earlier report's blanket Rails-side-flake attribution is superseded.

Changed reference tools: `behavior-message-list.mjs`, `behavior-toolbar.mjs`, `behavior.mjs`, `behavior-search-forward.mjs`, `behavior-mutations.mjs`, `behavior-check.py`, and the new planner module/test. Audit/report and pass-evidence attribution are updated. No Rust product, Rails application, golden, mask, atom, concurrency or job-observation change. No wait was widened; the repaired scopes use the pinned two-/ten-second waits. The pinned Selenium 4.35.0 atom remains byte-identical (SHA256 `ae26018c01cd27448b250f8e55a094cbfcd2e2cbbe171c78aaa906e1b5c3ed7c`).

Inventory remains **102 passed / 27 deferred / 6 WS12-blocked** out of 135 named system declarations, and **146/156 controller declarations**, with ten WS12 deferrals. This review correction earns no declaration credit. The release-click declaration stays deferred despite its successful fresh attempt. Full system sign-off remains partial.

**Duplicate-delivery limitation:** the case injects Turbo markup into the browser and checks its mounted-row/render-queue guard. It does **not verify server-originated redelivery**; the pinned original calls `message.broadcast_create`.

## Explicit hidden-scope sweep

All lines refer to `test/system/<file>_test.rb` at the pin; `system_test_helper` refers to `test/test_helpers/system_test_helper.rb`. The [complete assertion audit](ws8bm-visibility-audit.md) retains the thirty-case inventory and earlier discriminating visibility probes. Explicit hidden queries below never use the Selenium atom or a role/text locator that excludes hidden matches. A separate original visible `find`/action stays visible.

| Pinned Rails assertion | Port scope and outcome |
| --- | --- |
| message_list_a11y:307 (`visible: false`) | Raw row `aria-expanded` read; includes hidden nodes, unchanged. |
| message_list_a11y:313 (`visible: false`, wait 10) | **Changed:** CSS selector for live-log attributes, attached-only wait 10; returns before unrelated visible-message setup. Accepts a hidden log with visible children and a log with all messages hidden. |
| message_list_a11y:506 (`visible: false`) | Raw CSS viewport-meta attribute read; unchanged. |
| message_toolbar:49 (`visible: false`) | **Changed:** CSS More-button `aria-expanded=true` selector, attached-only default wait 2; accepts a toolbar hidden after the menu opens. The visible open-owner assertion at :48 remains separate. |
| message_toolbar:219 (`visible: false` find) | **Changed:** attached-only toolbar lookup at default wait 2 before hover. The body hover and subsequent visible toolbar assertion at :220–221 remain separate. |
| message_interactions:83,136,198 (`visible: false`) | Hidden composer contexts use attached-only CSS selectors; unchanged, including the deferred release-click case. |
| code_highlighting:41,80 (`visible: :all`) | Raw CSS counts for forbidden `img`/`script` nodes; inspect hidden nodes as well, unchanged. |
| workspace_markdown:138 (`visible: :all`) | Raw CSS count for forbidden `script`, `img[onerror]`, `javascript:` links; unchanged. |
| workspace_markdown:159 (`visible: :all` find) | `setInputFiles` on the raw CSS file-input locator; no visible lookup, unchanged. The attachment declaration stays deferred. |
| workspace_markdown:169 (`visible: false`) | Attached-only hidden composer context; unchanged, in the deferred attachment declaration. |
| workspace_markdown:265 (`visible: :all`) | Theme/mobile-navigation declaration remains deferred; no ported assertion or parity credit. |
| workspace_markdown:301 (`visible: :all`) | Compact-composer helper in that deferred declaration; no ported assertion or parity credit. |
| threads:93 (`visible: false`) | Attached-only hidden thread indicator, unchanged. |
| threads:450 (`visible: false`, wait 10) | Anchored-unread declaration remains deferred; existing thread setup also uses attached-only cable-source CSS selectors. No anchored-case parity credit. |
| threads:583 (`visible: false`, wait 10) | **Changed:** attached-only `aria-haspopup` attribute selector at wait 10; preserves the independent visible message find at :581. |
| system_test_helper:73–74 (`visible: false`, wait 0) | Shared browser startup uses an attached-only cable-source selector; no visibility filter. This is readiness, not credit for the helper's all-source count contract. |


## Before-fix evidence

Before changing eb062e20's assertions, added only the registered served acceptance probes. Both reported hidden-scope checks failed on Rails; this positive harness stops each named case at the first failing application, so it did not claim fresh Rust-before evidence. Astra's read-only `rereview-eb062e20/reverse-visibility.log` independently reproduces both applications. All three after-fix CSS proofs below are explicitly accepted on both apps, with valid startup, applied serving changes and no network failures; they earn no parity credit.

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_list_a11y message_toolbar --mutant-set hidden-scopes --keep-going
python3 rust/reference-tools/messaging/behavior-check.py sending_messages --case 'sending messages between two users' --negative --keep-going
python3 -m unittest discover -s rust/reference-tools/messaging -p '*_test.py'
```

The first invocation ran the two initial probes before the fully-hidden-log variant was added. The second reproduces the native scheduling bug: its actual mutant rejects on both apps, but the unregistered default adds an invalid result. Unit tests on the extracted unchanged planner fail three of five checks; the subsequent absent-explicit-variant test fails before its guard, then all six pass.

Raw summaries (`before-hidden-scopes.log`, `before-named-only.log`, `planner-before.log`, `planner-explicit-before.log`):

```text
WS8bm review escape check: 0 served mutants accepted on Rails and Rust; 2 failed probes; no parity credit
WS8bm discrimination check: 1 served mutants rejected on Rails and Rust across 1 named checks; 1 invalid or escaped
Ran 5 tests in 0.006s
FAILED (failures=3)
Ran 6 tests in 0.001s
FAILED (failures=1)
```

## Fresh-clone verification

Created `.scratch/ws8bm-hidden-scope-fresh` with `git clone --no-hardlinks --no-checkout .`, checked out the tool commits and generated fresh seeds, dependencies and its own Cargo target. The clone was advanced to e703cad5 before the thirty-case and complete discrimination runs. The visibility helpers are byte-identical across the follow-ups; the ordinary named-only scheduling path is unaffected. The final six planner tests were rerun there. Every browser invocation rebuilds/validates the pinned seed and performs `cargo build --locked -j2`; no case depends on a pre-existing scratch fixture or target. Rust code, vectors and parity inputs compare unchanged to eb062e20.

The full PR #189 discrimination set is **89 unique registered variants across 44 names**: the previous 88 plus the real live-log missing-attribute mutant. Three permitted hidden-scope variants are acceptance proofs, excluded from that count and from negative scheduling. Isolated named-only coverage duplicates one registered variant. The scope and search batches overlap five variants; the evidence verifier requires both-app markers for every unique expected tuple, with none missing or extra.

Run these from the fresh clone (the driver sets `CARGO_TARGET_DIR` to that clone's `rust/target`):

```sh
node --test rust/reference-tools/messaging/behavior-visibility.test.mjs
python3 -m unittest discover -s rust/reference-tools/messaging -p '*_test.py'
python3 rust/reference-tools/messaging/behavior-check.py sending_messages --case 'sending messages between two users' --negative --keep-going
python3 rust/reference-tools/messaging/behavior-check.py message_list_a11y message_toolbar --mutant-set hidden-scopes --keep-going
python3 rust/reference-tools/messaging/behavior-check.py message_toolbar message_actions_mobile message_interactions code_highlighting --keep-going
python3 rust/reference-tools/messaging/behavior-check.py search_forward_edit --keep-going
python3 rust/reference-tools/messaging/behavior-check.py search_forward_edit --case 'editing to add a URL renders its card live and the edited marker on load' --keep-going
python3 rust/reference-tools/messaging/behavior-check.py message_list_a11y --case 'the main message list is a live log' --keep-going
python3 rust/reference-tools/messaging/behavior-check.py message_list_a11y --case 'the menu closes before Turbo caches the page' --keep-going
python3 rust/reference-tools/messaging/behavior-check.py message_list_a11y --case 'the viewport allows pinch zoom' --keep-going
python3 rust/reference-tools/messaging/behavior-check.py threads --case 'creates a thread from a channel message and keeps the channel draft separate' --keep-going
python3 rust/reference-tools/messaging/behavior-check.py threads --case 'the thread root counts its replies live and hides the count when none remain' --keep-going
python3 rust/reference-tools/messaging/behavior-check.py message_toolbar message_actions_mobile message_interactions code_highlighting --negative --keep-going
python3 rust/reference-tools/messaging/behavior-check.py search_forward_edit --negative --keep-going
python3 rust/reference-tools/messaging/behavior-check.py workspace_markdown sending_messages threads composer boosting_messages search_forward_edit --mutant-set element-scopes --negative --keep-going
python3 rust/reference-tools/messaging/behavior-check.py message_list_a11y --case 'the main message list is a live log' --negative --keep-going
```

The URL isolation command ran three separate times with fresh fixtures after the full search batch. It passed all four paired observations at the original fifteen-second loading wait; no Rails-original flake is claimed. All thirty requested checks passed in this fresh attempt. Their existing release-click reliability deferral is retained. The five extra hidden-scope-related positive cases also passed, with database checks.

One first scope-batch attempt failed during Rails startup (`window.Stimulus=false`, before the named assertions, at the unchanged 30-second bootstrap wait). The guard rejected that run as invalid: it is retained in `fresh-scope-mutants.log` and never counted as a mutation rejection. Reran only the missing `transparent-cleared-thread-draft` variant in isolation with identical inputs and deadlines; both apps then reached the case and rejected it. No startup, network, queue or timing fix was made.

```sh
python3 rust/reference-tools/messaging/behavior-check.py composer --case 'thread drafts persist per thread without touching the channel draft' --mutant transparent-cleared-thread-draft --negative --keep-going
```

Raw summaries in run order (individual files under `.scratch/ws8bm-hidden-scope-review/`):

`fresh-helper-tests.log`

```text
ℹ tests 4
ℹ pass 4
ℹ fail 0
ℹ skipped 0
```

`fresh-planner-tests.log`

```text
Ran 6 tests in 0.000s
OK
```

`fresh-named-only.log`

```text
WS8bm discrimination check: 1 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
```

`fresh-hidden-scopes.log`

```text
WS8bm review escape check: 3 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
```

`fresh-reviewed-paired.log`

```text
WS8bm behaviour check: 30 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-search-paired.log`

```text
WS8bm behaviour check: 3 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-url-isolated-1.log`

```text
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-url-isolated-2.log`

```text
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-url-isolated-3.log`

```text
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-live-log.log`

```text
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-cache-menu.log`

```text
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-viewport.log`

```text
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-thread-menu.log`

```text
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-thread-indicator.log`

```text
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-reviewed-mutants.log`

```text
WS8bm discrimination check: 56 served mutants rejected on Rails and Rust across 30 named checks; 0 invalid or escaped
```

`fresh-search-mutants.log`

```text
WS8bm discrimination check: 8 served mutants rejected on Rails and Rust across 3 named checks; 0 invalid or escaped
```

`fresh-scope-mutants.log`

```text
WS8bm discrimination check: 28 served mutants rejected on Rails and Rust across 12 named checks; 1 invalid or escaped
```

`fresh-live-log-mutant.log`

```text
WS8bm discrimination check: 1 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
```

`fresh-recheck-thread-cleared.log`

```text
WS8bm discrimination check: 1 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
```

First fresh binary build (from `fresh-named-only.log`):

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 29s
```

The requested suite was reference tools only. Rust workspace tests and strict clippy were not rerun; no Rust product files changed. The prior tools-only workspace waiver remains in effect.

## Evidence, inventory and cleanup

```sh
python3 .scratch/ws8bm-hidden-scope-review/verify-evidence.py
python3 rust/reference-tools/messaging/deferred-system-inventory.py
git diff --check
git diff --exit-code eb062e20 -- rust/crates rust/vectors rust/parity
```

The evidence verifier is an artifact check generated in this turn, not an application test dependency. Every cited native case can regenerate its own fixtures from tracked inputs. Diff checks exited zero with empty output. Refreshed 37 existing pass-evidence records and one uncredited successful release-click diagnostic; all statuses, names and pinned source hashes are unchanged.

```text
WS8bm full discrimination evidence: 89/89 expected unique served variants rejected on Rails and Rust across 44 named checks; 0 unresolved
WS8bm hidden-scope evidence: 3/3 permitted served CSS variants accepted on Rails and Rust; excluded from negative scheduling; 0 missing
WS8bm reviewed positive evidence: 30/30 paired passes; 0 failed attempts retained; 0 missing declarations
WS8bm system inventory: 135 named declarations; 102 mapped behaviour passes; 27 deferred; 6 WS12 blocked; no pixel checks
WS8bm attribution: 37 existing pass records refreshed; 1 deferred successful attempt uncredited; 0 status/name/hash changes
```

Removed only the owned fresh clone's Cargo target after all gates finished; preserved source clones and raw logs. No owned application processes, assigned-port listeners or containers remain. No scratch Cargo target remains.

```sh
CARGO_TARGET_DIR="$PWD/.scratch/ws8bm-hidden-scope-fresh/rust/target" mise exec rust@1.98.1 -- cargo clean --manifest-path .scratch/ws8bm-hidden-scope-fresh/rust/Cargo.toml
```

```text
     Removed 6852 files, 5.2GiB total
WS8bm cleanup: zero scratch Cargo targets remain; assigned 52020/52021/52022 listeners and parity-owner containers absent
```

The requested review corrections are complete; stopping after the push as requested. The port remains partial, with exact remaining declarations below.

## Stable integration contract

The shell seam in [ws8bm-integration.md](ws8bm-integration.md) is unchanged: `Presenter::messages(&records)`, `messages::Index { ctx, messages }`, and `Presenter::room_message_list(&records, divider.message_id, divider.count)`. The shell passes verified request origin/viewer/assets/signer/CSRF context, selected records and divider facts. The composer receives room kind/name/ID, ordered commands, Drive flow, thread scope, the real scheduled-message child and request-owned PendingTemplate. Mount list/template boundaries verbatim. Real WS15g GitHub rendering and merged WS14e/M2 children remain in use. There is no new cross-owner production touch.

## Exact remaining work

### Remaining WS8bm behaviour (27)

`test/system/message_interactions_test.rb`

- a release click landing on the just-opened menu does not activate it — known reference failures; this fresh attempt passes both apps, but the original assertion/deadlines and reliability deferral remain, with no new declaration credit

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
