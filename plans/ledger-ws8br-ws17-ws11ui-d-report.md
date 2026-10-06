# WS8br / WS17 / WS11-UI remainder — D

Reference: `78b9b1546bdab4c6c1c9b8ddb94512f661289112` (`rust/parity/reference.sha`).
Branch: `rust/ledger-ws8br-ws17-ws11ui-d`, based on merged #248 at
`be5cace8ee08e0435a4ed82286a5c134b7dfdfeb`.

Integration status: both final four-wrapper runs and their receipt checks passed.
Canonical reconciliation closes all active gaps; the three pre-existing geometry
exclusions retain their original dispositions.

## Closure inventory

The 104 broad declarations have exact per-assertion browser maps. The six
WS8br2 criteria overlap six of these declarations and receive no extra execution
credit. Muted delivery and Calendar add two distinct lifecycle declarations.
One supplementary live-unread declaration at the current Rails pin is executed
without increasing the 104 legacy count. Each round therefore executes 107
cases against Rails and Rust: 214 real browser executions.

| Broad category (`test/system/*_test.rb`) | Closed declarations | Remaining active |
|---|---:|---:|
| keyboard_shortcuts | 14 | 0 |
| member_select_mode | 18 | 0 |
| sidebar_room_menu | 16 | 0 |
| channel_navigation | 7 | 0 |
| room_header | 9 | 0 |
| motion | 9 | 0 |
| content_security_policy | 5 | 0 |
| unread_divider | 5 | 0 |
| channel_members | 4 | 0 |
| icons | 4 | 0 |
| audit_log | 2 | 0 |
| mobile_layout | 2 | 0 |
| service_worker | 2 | 0 |
| timezone_detection | 2 | 0 |
| unread_rooms | 2 | 0 |
| people_group_dms | 1 | 0 |
| workspace_icons | 1 | 0 |
| browser_launch_profile | 1 | 0 |
| **Broad total** | **104** | **0** |
| WS8br2 original criteria (overlapping) | 6 | 0 |
| Muted noise → positive delivery control → mention delivery | 1 | 0 |
| Calendar opt-in → busy → clock advance → cleared badge | 1 | 0 |
| Partial aggregate mapping record (overlapping) | 1 | 0 |

## Assertion and fixture fidelity

The four `ledger-ws8br-ws17-ws11ui-d-*-receipts.json` manifests map all 607
direct assertion sites, 339 helper/setup assertion expansions, 382 synchronized
interaction premises, and 113 additional fixture/environment/authentication
premises. Each citation names the pinned Ruby line, its exact text, and the
executable browser assertion file:line and anchor. The checker rejects missing
original assertions, stale anchors, missing repeated helper invocations,
unregistered wrappers, changed implementation digests, and incomplete paired
execution markers.

The shared visibility implementation runs the pinned Selenium displayed atom.
Visible text runs the ChromeDriver GET_TEXT atom, verified against the installed
native driver; explicit `visible: :all` uses pinned Capybara all-text
normalization. Thirteen native text proofs pass, including six examples where
`innerText` gives a different result. Finder predicates also preserve disabled
fields and the original frame/element scope. Geometry reads the original
DOM-first selector and exact boundary/size/scroll predicate. Original outer
window dimensions are measured through native WebDriver for the current run;
fullscreen uses the native screen dimensions. The profile declaration inspects
native ChromeDriver's managed launch profile rather than a Playwright substitute.

Every wrapper uses the existing isolated original-browser byte transport and
port-lease harness. Both actual renderers run in the Rails test environment.
Each case restores all original database tables, FTS tables and SQLite sequences,
then clears the actual renderer cache, pubsub/test queues and current Kit rate
limits. The seed only loads the original fixtures. Authentication executes the
original credential GET `/test_session`, creating a real verified session and
cookie; no imported fixture cookie supplies authentication.

Fixture writes, verbs, class/declaration sign-ins, focus, media preferences and
configuration changes occur at their original points. Late Huddle environment
changes retain the live browser contexts and server DB/Cable/job dependencies,
rebind the real Rust configuration, and restore the previous values in `finally`.
Original forgery-protection policy is applied per declaration. CSP checks inspect
actual violation records, secrets in rendered text, and actual response headers.
Status checks are additional transport guards; they do not replace mapped DOM
or body assertions.

Muted delivery uses real model creation and broadcasts, checks the loud room as
the positive control, then verifies mention delivery in the muted room. Calendar
uses the actual profile form and persisted opt-in, drains exactly the UI-enqueued
refresh job through the actual recorded Google transport, dispatches the real
meeting windows, advances the shared clock six minutes, and visits the rendered
profile again to check the cleared badge. The Rails bridge is tools-only; Rails
production files and application JS/CSS assets are unchanged.

One bounded driver timing adaptation is explicit: the landscape menu waits one
rendering frame after the original visibility/text/boundary predicates before
the original page-level End key. It assigns no focus and mutates no DOM. The
original two-second focus deadline remains unchanged; no guaranteed Selenium
animation-frame contract is claimed.

## Confirmed producer repair

`controllers/presenters.rs` previously classified only Unicode emoji as
emoji-only content. The original `icons_test.rb:38` failed for `:openai:` because
the rendered message lacked `.message--emoji`. The shared message/boost
classifier now recognizes registered built-in and workspace shortcodes, rejects
unknown names and mixed prose, and consults the current workspace catalog so
upload/deletion changes classification. Two focused tests cover these predicates
and the browser regression passes against both Rails and Rust.

The private Rust browser host also mirrors Rails test-mode null fragment caching.
An earlier harness configuration incorrectly enabled production fragment caching
and reused a frozen membership version. That was a test-context defect; no
production cache key or direct-room fixture was changed. Earlier construction
receipts and controls are uncredited history.

## Producer mutation evidence

The combined `ledger-ws8br-ws17-ws11ui-d-mutations.json` binds the four source
receipts and includes their literal failure/restoration output, hashes, commands,
and mapped coordinates. It credits **44 valid producer defects across 38 distinct
closures; zero invalid controls**. Each defect fails the cited original assertion,
then restores the producer. Final controls use pure fixtures, real credential
authentication, original environment/forgery policy, null test cache and the
current Kit counter reset.

| Source receipt | Valid controls | Distinct closures |
|---|---:|---:|
| `d-navigation-mutations.json` | 16 | 14 |
| `ledger-ws8br-ws17-ws11ui-d-members-mutations.json` | 15 | 11 |
| `d-surfaces-mutations.json` | 11 | 11 |
| `ledger-ws8br-ws17-ws11ui-d-lifecycle-mutations.json` | 2 | 2 |
| **Total** | **44** | **38** |

Scope/geometry controls include a foreign sidebar menu and member selection
count outside the original scope, hidden help/checkbox/inline buttons,
zero-width header and inline actions, and out-of-frame headers/inline actions/
sticky member controls. The exception coordinate itself must match the map;
merely printing a successful marker elsewhere cannot receive mutation credit.

## Required local verification

All commands run from the repository root unless specified. No full workspace
test suite was run locally. Task output is retained under `rust/target/ledger-d/`.

The two full browser commands differ only by the second run's CI profile:

```bash
PARITY_IMAGE=review236-reference:78b9b1546 \
PATH="$PWD/rust/target/tools:$PATH" \
CARGO_TARGET_DIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-main-ci/rust/target \
WS11UI_BROWSER_BINARY=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-main-ci/rust/target/debug/campfire \
cargo nextest run --manifest-path rust/Cargo.toml --locked -p campfire -j 4 \
  -E 'test(controllers::ledger_browser_tests) & not test(original_ledger_injected_clock_host)' \
  --run-ignored only --success-output final --no-fail-fast --no-tests fail

PARITY_IMAGE=review236-reference:78b9b1546 \
PATH="$PWD/rust/target/tools:$PATH" \
CARGO_TARGET_DIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-main-ci/rust/target \
WS11UI_BROWSER_BINARY=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-main-ci/rust/target/debug/campfire \
cargo nextest run --manifest-path rust/Cargo.toml --locked -p campfire --profile ci -j 4 \
  -E 'test(controllers::ledger_browser_tests) & not test(original_ledger_injected_clock_host)' \
  --run-ignored only --success-output final --no-fail-fast --no-tests fail
```

First run: **4 passed, 0 failed, 3059 skipped**; run ID
`c95a5c36-6637-419e-b130-fb88f47c175d`; 214 paired browser executions.
Second run: **4 passed, 0 failed, 3059 skipped**; run ID
`684ed862-efc4-4770-8a48-93e6ac108291`; 214 paired browser executions.
Total: **8 wrapper passes, 0 failures; 428 actual browser executions**.

Focused native tests:

```bash
PATH="$PWD/rust/target/tools:$PATH" \
CARGO_TARGET_DIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-main-ci/rust/target \
cargo nextest run --manifest-path rust/Cargo.toml --locked \
  -p campfire -p campfire_views -p campfire_kit -j 4 \
  -E 'test(all_emoji) | test(disabled_forgery_omits_tokens_and_preserves_the_real_csp_nonce) | test(fixture_rebind_preserves_live_counters_clock_and_cookies)' \
  --success-output final
```

**4 passed, 0 failed, 3410 skipped**. Strict clippy from `rust/`:

```bash
CARGO_BUILD_JOBS=4 \
CARGO_TARGET_DIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-main-ci/rust/target \
cargo clippy -p campfire -p campfire_views -p campfire_kit --all-targets -- -D warnings
```

**Passed: all three touched crates, zero warnings.** Supporting harness tests:

```bash
python -m unittest discover -s rust/reference-tools/users -p ledger_fixture_state_test.py
python -m unittest discover -s rust/reference-tools/messaging -p browser_host_test.py
python rust/ci/ignored_tests.py
```

**25 Python tests passed** after merging main (1 fixture reset, 24 generated-host cases,
including this branch's include-input case on main's #251 include audit). The ignored
registry had **25 correctness tests and 7 compiled workspace utilities, zero unowned
ignores** on this branch (78 correctness tests after merging #252). The source guard
prints 8 utilities because it also counts `ws8bm_browser_host_without_jobs`, which only
the generated messaging host compiles. Merging #251 and #252 changed `app.rs`,
`concerns/sudo.rs` and `presenters.rs` (main's additions only), so the receipts'
`implementation_sha256` for those three files were refreshed and the four wrappers
re-ran on the merged head in the dispatched correctness matrix. A fresh
generated build tree carries all **934 literal crate include sites** (933 inside
`rust/`, referencing 651 distinct inputs, plus the root `public/500.html` fixture),
including plan/receipt inputs outside the old whitelist; a filtered generated host compile
also passed during construction. The generated host uses a separate target
directory so it cannot replace a running nextest executable.

Ledger/receipt commands (the receipt command is run against each final log):

```bash
python rust/reference-tools/check-ledger-d-receipts.py \
  --nextest-list rust/target/ledger-d/ledger-current-list.json \
  --browser-log rust/target/ledger-d/wrappers-final-2.log \
  --mutations rust/plans/ledger-ws8br-ws17-ws11ui-d-mutations.json \
  --manifest rust/plans/ledger-ws8br-ws17-ws11ui-d-navigation-receipts.json \
  --manifest rust/plans/ledger-ws8br-ws17-ws11ui-d-members-receipts.json \
  --manifest rust/plans/ledger-ws8br-ws17-ws11ui-d-surfaces-receipts.json \
  --manifest rust/plans/ledger-ws8br-ws17-ws11ui-d-lifecycle-receipts.json

python rust/reference-tools/reconcile-ledger-d.py \
  --nextest-list rust/target/ledger-d/ledger-current-list.json \
  --browser-log rust/target/ledger-d/wrappers-final-2.log \
  --mutations rust/plans/ledger-ws8br-ws17-ws11ui-d-mutations.json

python rust/reference-tools/check-cutover-ledgers.py \
  --nextest-list rust/target/ledger-d/ledger-current-list.json \
  --browser-log rust/target/ledger-d/wrappers-final-2.log --browser-scope d
python rust/reference-tools/users/deferred_inventory.py
git diff --check HEAD
```

The receipt checker passed independently on both final logs: 106 legacy
declarations plus one current-pin addition, 214 paired executions per round,
zero unaccounted assertions. Canonical reconciliation, the cutover checker and
deferred inventory pass with zero active remainder. All 337 broad WS8 receipts,
all 14 WS8br2 criteria and all 15 reopened WS17 records are now closed.

## Preserved exclusions and remote gate

The three existing declarations remain `outside-current-gate`, with their
original geometry/style-only no-pixel-phase evidence unchanged:

| ID | Pinned declaration | Reason retained |
|---|---|---|
| P0340 | `mobile_layout_test.rb:47` — profile fits phone widths | Existing geometry/style-only phase exclusion; no new closure receipt claimed. |
| P0341 | `mobile_layout_test.rb:60` — outside headers stay opaque | Existing geometry/style-only phase exclusion; no new closure receipt claimed. |
| P0342 | `mobile_layout_test.rb:84` — outside headers never cover page/scrollbar | Existing geometry/style-only phase exclusion; no new closure receipt claimed. |

The seven-job correctness matrix is dispatched after the single branch push:

```bash
gh workflow run rust.yml --ref rust/ledger-ws8br-ws17-ws11ui-d \
  -f ref=rust/ledger-ws8br-ws17-ws11ui-d
```

Its run URL, exact head and completed job conclusions are reported on the PR and
in the final handoff; PR CI alone does not establish this correctness gate.
