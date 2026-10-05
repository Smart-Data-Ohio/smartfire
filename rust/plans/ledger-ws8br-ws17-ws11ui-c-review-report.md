# PR #248 review follow-up

Review base: `dae59026d33979ce42fafc0be957954612426711`.
Rails oracle: `78b9b1546bdab4c6c1c9b8ddb94512f661289112`.
Disk-upload review items 3 and 4 are intentionally left to their separate PR.

## Fixes and causal evidence

The runner reserves three ports from **24000–24191**, outside this host's
**32768–60999** outbound ephemeral range. Individual Linux abstract-socket names
coordinate overlapping allocations across processes/worktrees in the host network
namespace. They remain held through Rails boot, Rust boot, browser execution and
cleanup; process exit releases them. The bounded allocation scan rejects occupied
leases/listeners before boot. Unsafe range overrides fail before startup. There
are no server-startup retries, wider deadlines or weaker assertions.

The network forwarder's Unix socket and temporary runs now live under `rust/`.
A relative Unix address avoids Linux's 108-byte address limit in deep worktrees.
The unchanged HTTP/WebSocket forwarder still forwards actual server bytes.

Four real-kernel Python regressions cover outbound ephemeral connections, separate
processes/overlapping ranges, release/reallocation, and unsafe overrides. The new
enabled ordinary-CI Rust test invokes them:
`controllers::ws11ui_original_browser_tests::original_browser_port_leases_are_host_coordinated_and_ephemeral_safe`.
Removing the coordinator fails the cross-process test; restoring the old unsafe
range without validation fails the outbound-safety test. Restored: **4/4 pass**.

The tour tests read the actual HTTP server's database after its completion PATCH,
before restart/navigation. They preserve the pinned `assert_tour_completed` helper
and all three `data-tour-auto-start-value="false"` assertions. Group rename checks
the visible success notice; leave checks departure from both edit and show URLs,
with Rails' explicit ten-second wait, then verifies actual persisted membership.
Member-panel tests wait for at least three visible rows, including the related
member-huddle case. Dismissal checks rendered menus, independent of `[hidden]`.
The explicit Rails `visible: :all` intermediate menu/card/grouping assertions keep
their attached-node semantics.

The audit observer now projects **`tbody td code`**, preserving both required
ancestors. The actual template mutant replaces the action cell with
`<th><code>...</code></th><td>...</td>`: the old P0245 test passes, the repaired test
fails on the missing action codes, and the restored producer passes.

| Actual producer defect | Before strengthening | After strengthening |
| --- | --- | --- |
| Skip acknowledges completion without saving | Survives | Actual DB stamp assertion rejects |
| Completed page has incorrect auto-start attribute | Survives | False-attribute assertion rejects |
| Real rename response loses its notice | Survives | Visible notice assertion rejects |
| Leave saves membership removal but suppresses navigation | Survives | Departure from edit URL rejects |
| Only two member rows are visible; others remain attached | Survives | Three-visible-row wait rejects |
| Dismissed menu remains rendered despite `[hidden]` | Survives | No-rendered-menu assertion rejects |
| Member identities are hidden with valid geometry | Survives | Visible identity assertion rejects |
| Initial picker has only two visible rows, keeping named filter matches usable | Survives | Original full-visible-list assertion rejects |
| Audit action code is under `th`, preserving plain `td` text | Survives | Required `td` ancestry rejects |

Controls that failed setup or an earlier unrelated interaction were not credited.
The initial member CSS retained three rows across different groups, the initial
identity control blocked a later click, and an all-row picker hide failed an
already-existing text predicate. The corrected controls retain the earlier
interactions and demonstrate the eight browser survivors above. Compilation,
startup, transport and mutation-setup errors are invalid controls.

The sidebar Rails oracle resets the cache-test override before the unconfigured
declaration. Its `direct_stacks` changes **4 → 0**; every other recorded field,
including the cache sequence and query counts, regenerates unchanged. This fixes
fixture contamination without changing production behaviour or the original
unconfigured Rust test's zero-stack predicates.

## Complete 120-declaration scan

[`ledger-ws8br-ws17-ws11ui-c-review-selector-scan.md`](ledger-ws8br-ws17-ws11ui-c-review-selector-scan.md)
lists each declaration's selector/cardinality disposition. The native and browser
assertion tables map every current pinned Rails assertion to its exact Rust
assertion file:line. The current reference is also enforced by both validators,
so an older historical source cannot silently omit newer runtime assertions.

The result is **93 native declarations / 279 direct sites**, plus **27 browser
declarations / 147 direct sites**: **120 declarations / 426 direct sites**.
There are 15 native helper expansions, 42 browser helper expansions and four
browser setup assertions. Legacy input-vector/source labels retain their original d7c7de92 provenance; the
current assertion sets and source hashes explicitly pin 78b9b1546.
The historical 420-site receipt is identified in JSON
history and retained in the preceding checkpoint report. No declaration is newly
closed by a broad file-level receipt.

## Wrapper startup receipts

All seven wrappers were run at four workers three times. Each run executes the
27 original cases on both Rails and Rust, including actual server database checks.
All **21 wrappers passed**, **162 paired case executions passed**, and there were
**0 startup failures**, including zero `EADDRINUSE`/server-exit/startup-timeout
errors. Raw nextest summaries:

```text
     Summary [ 192.541s] 7 tests run: 7 passed (7 slow), 2959 skipped
     Summary [ 141.178s] 7 tests run: 7 passed (4 slow), 2959 skipped
     Summary [ 107.912s] 7 tests run: 7 passed (4 slow), 2959 skipped
```

The historical "picker host startup exit" has no retained server log. Astra's
new collision trace proves the old allocation's failure class, and this fix
prevents that class; it does **not** prove the historical picker's exact cause.
That old failure remains recorded rather than being relabelled a pass/rejection.

## Executed commands and raw summaries

Commands run from the worktree root. `PARITY_IMAGE=review236-reference:78b9b1546`;
the pinned Playwright image is `ws12-playwright:2091a0eabd5d`. `cargo-nextest` was
installed only in `rust/.scratch/rv248-fix/bin`, then added to this command's PATH.
Rustc's machine-wide throttle is unchanged.
The final workspace uses the existing target runner
`CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="python3 $PWD/rust/.scratch/rv248-fix/fresh/rust/reference-tools/agents/pinned-media-runner.py"`
and `PINNED_MEDIA_SCRATCH="$PWD/rust/.scratch/rv248-fix/pinned-media"`.

```sh
python3 -B -m unittest discover -s rust/reference-tools/users -p test_browser_port_leases.py -v
python3 rust/reference-tools/check-original-browser-review-controls.py
python3 rust/reference-tools/users/run_original_browser_assertions.py people --controls
python3 rust/reference-tools/users/run_original_browser_assertions.py pickers --controls
python3 rust/reference-tools/users/run_original_browser_assertions.py members --controls
python3 rust/reference-tools/users/run_original_browser_assertions.py group --controls
python3 rust/reference-tools/users/run_original_browser_assertions.py tours --controls
python3 rust/reference-tools/users/run_original_browser_assertions.py stars --controls
python3 rust/reference-tools/users/run_original_browser_assertions.py worker --controls
cargo nextest run --manifest-path rust/Cargo.toml --locked -p campfire -j 4 --no-fail-fast --run-ignored only --success-output final -E 'test(controllers::ws11ui_original_browser_tests)'
```

The three wrapper invocations are distinct successful runs, rather than retries
of a failed startup. The ordinary control modes were executed with a four-worker
pool; their scripts restore or isolate every producer control.

```text
Browser review before/after: 8 survivors -> 8 intended rejections; 0 invalid controls
Original browser review discrimination: 8 producer defects rejected; 0 invalid controls
Original browser people: 10 producer defects rejected; 0 invalid controls
Original browser pickers: 5 producer defects rejected; 0 invalid controls
Original browser members: 2 producer defects rejected; 0 invalid controls
Original browser group: 1 producer defects rejected; 0 invalid controls
Original browser tours: 4 producer defects rejected; 0 invalid controls
Original browser stars: 4 producer defects rejected; 0 invalid controls
Original browser worker: 1 producer defects rejected; 0 invalid controls
Port control ephemeral-range : rejected
Port control coordination : rejected
```

P0245 with the malformed real template, before/after the selector fix, then the
restored actual producer:

```text
     Summary [   0.908s] 1 test run: 1 passed, 2965 skipped
     Summary [   0.921s] 1 test run: 0 passed, 1 failed, 2965 skipped
     Summary [   0.862s] 1 test run: 1 passed, 2965 skipped
```

The full workspace and strict clippy use a no-hardlink fresh clone under
`rust/.scratch/rv248-fix/fresh`, with current Rust changes copied into it. Only
CI's `default`, `first_run`, and `agents_ui` seeds were freshly built there.
`CAMPFIRE_REFERENCE` points to that clone; `CARGO_TARGET_DIR` points to the new
worktree `rust/target`. The temporary clone, downloaded tool and target are
removed after final verification.

```sh
rust/.scratch/rv248-fix/fresh/rust/parity/bin/seed build default first_run agents_ui
cargo nextest run --manifest-path rust/.scratch/rv248-fix/fresh/rust/Cargo.toml --locked --workspace --exclude html5ever -j 4 --no-fail-fast --failure-output immediate
cargo clippy --manifest-path rust/.scratch/rv248-fix/fresh/rust/Cargo.toml --locked --workspace --exclude html5ever --all-targets -- -D warnings
python3 rust/reference-tools/check-controller-assertion-receipts.py --manifest rust/plans/ledger-ws8br-ws17-ws11ui-c-receipts.json --nextest-list rust/.scratch/rv248-fix/current-list.json --native-log rust/.scratch/rv248-fix/workspace.log
python3 rust/reference-tools/check-original-browser-receipts.py --nextest-list rust/.scratch/rv248-fix/current-list.json --browser-log rust/.scratch/rv248-fix/wrappers-3.log --controls-dir rust/.scratch/rv248-fix
```

The initial workspace command omitted the existing pinned media runner. Six
media tests failed under the host runtime (PNG bytes and video preview/variant
headers); four in-flight tests were cancelled when that run was stopped. All
seven pinned application-media tests then passed unchanged with the existing
runner. This is an environment correction, not a changed byte expectation or a
timing/startup retry. The interrupted run is retained:

```text
     Summary [ 556.516s] 855/5075 tests run: 845 passed (1 slow), 10 failed, 27 skipped
     Summary [   7.203s] 7 tests run: 7 passed, 2960 skipped
```

The corrected full-workspace and strict-clippy results:

```text
     Summary [1313.698s] 5075 tests run: 5075 passed (2 slow), 27 skipped
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 14s
Original browser per-assertion receipts: 27 declarations; 147 direct sites; 42 private-helper expansions; 4 setup assertions; 54 paired executions passed; 27 producer defects rejected; 0 unaccounted assertions
```

The final gate used the same fresh-clone inputs:

```sh
cargo nextest run --manifest-path rust/.scratch/rv248-fix/fresh/rust/Cargo.toml --locked -p campfire -j 4 --no-fail-fast --run-ignored only --success-output final -E 'test(controllers::ws11ui_original_browser_tests)'
python3 rust/reference-tools/check-original-browser-receipts.py --nextest-list rust/.scratch/rv248-fix/current-list.json --browser-log rust/.scratch/rv248-fix/wrappers-4.log --controls-dir rust/.scratch/rv248-fix
python3 rust/reference-tools/check-cutover-ledgers.py --nextest-list rust/.scratch/rv248-fix/current-list.json --native-log rust/.scratch/rv248-fix/workspace.log
```

The final fresh-clone browser gate also passes after keeping the ordinary member
minimum within its single pinned ten-second wait (the first-row wait is confined
to mutation setup). All final compiled Rust inputs match the worktree bytes.
This fourth successful run raises the total to **28/28 wrappers**, **216 paired
case executions**, **0 startup failures**.

```text
     Summary [  90.247s] 7 tests run: 7 passed (2 slow), 2960 skipped
Controller per-assertion receipts: 93 audited declarations (93 closed, 0 reopened); 279 assertion sites; 82 enabled native test identities passed; 0 explicit reopened gaps; 0 unaccounted assertions
Final fresh-clone wrapper startup failures: 0
```

The existing cutover validator remains honest about the larger ledger:

```text
Cutover current branch: 181 credited test identities passed in the supplied current workspace run
Cutover ledger receipts: 25 historical CI test identities still enabled; 14 WS17 closures; 3 ignored browser registrations for rust/ci-full-gate; 233 broad WS8 closures; 1 approved queue supersession; 0 inconsistent records
Cutover ledger remains partial: 104 broad receipts; 0 sidebar receipts; 6 overlapping criteria; 1 muted browser; 1 Calendar browser; 3 geometry-only exclusions
```

All requested non-upload review fixes are complete. The historical picker exit
remains causally unconfirmed because its log was lost; no new startup failure
remains. The broader ledger and owner-held scope are unchanged.

## Merge of PR #244 review fixes

Merged `rust/ledger-ws8br-ws17-ws11ui-b` at
`471ab2c5214b57483ebd0c9660d7acff8fceb79f` with a merge commit.
The populated zone-clear and exact DOM assertions are retained, including C's
existing original GitHub profile cases and all later acceptance closures. B's
401 current-pin assertions replace its historical map; source coordinates are
refreshed for this merged tree. C's 93 native and 27 browser mappings keep their
existing credit and historical evidence. Browser producer/harness inputs are unchanged.

The first build reused a Kit artifact from the other worktree and incorrectly
reported `Ctx::take_body_file` missing, though that method exists in C's source.
The shared target was invalidated for every workspace package and both checks
were rebuilt; no production source change was needed.

Verification used `CI=1`, the same three CI seeds and pinned media runtime,
`nextest -j 4` and the configured machine-wide rustc throttle. The affected
page/receipt expression ran 158 tests; the 30 supplemental historical identities
complete the combined ledger's 181 distinct credited identities.

Commands (from this worktree; expressions are the exact named identity sets
from the B/C manifests and the parent ledger plus every affected page test):

```sh
cargo nextest run --manifest-path rust/Cargo.toml --locked -p campfire -j 4 --no-fail-fast -E AFFECTED_EXPRESSION
cargo nextest run --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever -j 4 --no-fail-fast -E SUPPLEMENTAL_EXPRESSION
cargo clippy --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --all-targets -- -D warnings
python3 rust/reference-tools/check-controller-assertion-receipts.py --nextest-list LIST_JSON --native-log COMBINED_LOG
python3 rust/reference-tools/check-controller-assertion-receipts.py --manifest rust/plans/ledger-ws8br-ws17-ws11ui-c-receipts.json --nextest-list LIST_JSON --native-log COMBINED_LOG
python3 rust/reference-tools/check-cutover-ledgers.py --nextest-list LIST_JSON --native-log COMBINED_LOG
```

Raw successful summaries:

```text
     Summary [  41.282s] 158 tests run: 158 passed, 2809 skipped
     Summary [  33.944s] 30 tests run: 30 passed, 5072 skipped
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 06s
Controller per-assertion receipts: 120 audited declarations (112 closed, 8 reopened); 401 assertion sites; 70 enabled native test identities passed; 11 explicit reopened gaps; 0 unaccounted assertions
Controller per-assertion receipts: 93 audited declarations (93 closed, 0 reopened); 279 assertion sites; 82 enabled native test identities passed; 0 explicit reopened gaps; 0 unaccounted assertions
Cutover current branch: 181 credited test identities passed in the supplied current workspace run
Cutover ledger receipts: 25 historical CI test identities still enabled; 14 WS17 closures; 3 ignored browser registrations for rust/ci-full-gate; 233 broad WS8 closures; 1 approved queue supersession; 0 inconsistent records
Cutover ledger remains partial: 104 broad receipts; 0 sidebar receipts; 6 overlapping criteria; 1 muted browser; 1 Calendar browser; 3 geometry-only exclusions
```

The temporary B worktree, downloaded nextest executable and shared build target
are removed after pushing. Small raw logs remain under `rust/.scratch/`. No stash
was used and no source outside `rust/` was edited.


## Rereview of f50d735d: retain browser scopes

Reference remains `78b9b1546bdab4c6c1c9b8ddb94512f661289112`. Merge commit
`b273b6880` incorporates B `7263ad8af7d7fbd2944966dd01abd91317798584`.
The conflict resolution retains B's PUT/PATCH profile replay loop and C's
selected, named profile cases. Both ledger histories remain, and the eight
independently closed overlapping records are counted once. Live B/C source
coordinates and the B branch-sweep coordinates were refreshed after resolution.
No disk-upload code or expectations were changed; review findings 3/4 remain
with PR #249.

The scope sweep found these three shared helper defects:

| Family | Rails scope | Fix and discriminating mutation |
| --- | --- | --- |
| Picker filters, visible rows/text/counts and empty states | `people_group_dms_test.rb:188`, `:221`, `:249`, `:296`: within `#direct_rooms_control` | `browser_scopes.mjs:2`–`:6`, used by both original and legacy picker scripts. Move the real `.directs--new` controller element outside the frame: 0 rows inside, 8 outside. The old original case passes; the fixed case rejects at the original `:192` predicate. |
| Tour keyboard interaction | `first_run_tour_test.rb:101`: `.tour__card [data-tour-target='next']` | `browser_scopes.mjs:7`, used by original and legacy tour scripts. Move the real next button outside the card, retaining its real click/keydown actions. The old case still finishes and persists; the fixed helper rejects the missing card descendant. |
| Status-popup field interactions | `status_popup_test.rb:19` and `:65`: within `#user_card` | `browser_scopes.mjs:8`, used by legacy status script for presence, emoji, text and expiry fields. Move the actual text field outside the card, retaining explicit form association. The old case still saves and reads back the value; the fixed helper cannot find that outside field. |

The original three system files and all browser assertion helper families under
`reference-tools/users/` were inspected. Card/name, directory/bar, member/row,
starred-group/menu and message-author helpers already retain their ancestors.
Document-wide predicates stay document-wide where Rails does: checked-field
IDs, picker-row interactions, phone picker geometry, phone status-field presence,
tour progress/title/anchor observations, controller readiness and animation
settling. Audit table assertions retain `tbody td code`. Timezone metadata,
service-worker events and navigation helpers have no lost enclosing `within`.
The WS12 thread helpers accept already-qualified selectors and preserve their
board/form/thread scope. No other lost ancestor was found.

The persisted-tour receipt additionally now names the actual current-pin
`assert_not_nil` at line 96, instead of the closing `end` at 95. The receipt guard
requires helper/setup rows to cite an assertion. Its new explicit `--modes`
option limits current execution validation to touched wrappers while checking
all 27 declarations, 147 direct assertions, 42 helper expansions and four setup
assertions against the pin and live source. Omitting `--modes` still requires
all seven wrappers and all 54 paired executions. No test predicate, retry,
wait/deadline or expected Rails response was weakened.

Mutation details and exact call sites are in
`ledger-ws8br-ws17-ws11ui-c-scope-review.json`. Before receipts use f50d735d's
helpers (unchanged by the B merge); after receipts use the scoped helpers. All
three are actual rendered-element/producer relocations, with real saves where
applicable, and ordinary transport/setup failures are excluded. The first
experimental tour relocation removed the keyboard event ancestry as well and
failed even with the old helper; it was excluded and replaced with the functional
producer control described above. A parallel invocation of the older developer
runner also collided on its PID-derived seed name before any tour assertions;
that invalid setup attempt is excluded. Those older scripts were then invoked
serially. This does not affect the registered CI wrappers, whose kernel leases
and private seeds remain unchanged.

Only targeted tests were run. All builds use the configured machine-wide four
rustc slots; Docker invocations join the same lock pool via a private copy of the
existing wrapper. The global throttle/configuration is unchanged. Test inputs
are CI's default, first_run and agents_ui seeds, independently validated by the
pinned Rails image (29 + 4 + 40 checks, zero failures). The Rust normal binary
was built before browser replays. No application JS/CSS assets changed.

Executed commands (inside the Rust 1.98.1 / Node 26.10.0 correctness image,
`PARITY_IMAGE=review236-reference:78b9b1546`, CI=true):

```sh
cargo build --manifest-path rust/Cargo.toml --locked -p campfire --bin campfire
cargo nextest list --manifest-path rust/Cargo.toml --locked -p campfire --message-format json
cargo nextest run --manifest-path rust/Cargo.toml --locked -p campfire -j 4 --no-fail-fast -E "$(python3 rust/reference-tools/users/review248_targeted_filter.py LIST.json)"
cargo nextest run --manifest-path rust/Cargo.toml --locked -p campfire -j 4 --no-fail-fast --run-ignored only --success-output final -E 'test(=controllers::ws11ui_original_browser_tests::original_picker_assertions) or test(=controllers::ws11ui_original_browser_tests::original_tour_assertions)'
python3 rust/reference-tools/users/run_original_browser_assertions.py pickers --controls
python3 rust/reference-tools/users/run_original_browser_assertions.py tours --controls
python3 rust/reference-tools/users/run_original_browser_assertions.py pickers --mutation picker-scope
python3 rust/reference-tools/users/run_original_browser_assertions.py tours --mutation tour-key-scope
env WS8BR2_BROWSER_SCOPE_CONTROL=status-field python3 rust/reference-tools/users/browser_people.py --status
python3 rust/reference-tools/users/browser_people.py --picker
python3 rust/reference-tools/users/browser_people.py --tour
python3 rust/reference-tools/users/browser_people.py --status
cargo clippy --manifest-path rust/Cargo.toml --locked -p campfire --all-targets -- -D warnings
cargo clippy --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --all-targets -- -D warnings
cargo nextest list --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --run-ignored only --ignore-default-filter --message-format json
python3 rust/ci/compiler_ignore_mutations.py
```

The native filter generated from the tracked helper exactly equals the executed
177-identity filter (all merged/touched modules, B/C credited tests and the lease
regression). A workspace **listing**, not a workspace test run, supplies the
compiled ignored-test inventory.

Executed guards, using the current compiled listings and captured logs:

```sh
python3 rust/reference-tools/users/check_scope_mutation_receipts.py --evidence rust/.scratch/review248-r2
python3 rust/reference-tools/check-controller-branch-sweep.py
python3 rust/reference-tools/check-controller-assertion-receipts.py --nextest-list rust/.scratch/review248-r2/nextest-list.json --native-log rust/.scratch/review248-r2/native.log
python3 rust/reference-tools/check-controller-assertion-receipts.py --manifest rust/plans/ledger-ws8br-ws17-ws11ui-c-receipts.json --nextest-list rust/.scratch/review248-r2/nextest-list.json --native-log rust/.scratch/review248-r2/native.log
python3 rust/reference-tools/check-original-browser-receipts.py --nextest-list rust/.scratch/review248-r2/ignored-list.json --browser-log rust/.scratch/review248-r2/browser-final.log --controls-dir rust/.scratch/review248-r2 --modes pickers tours
python3 rust/reference-tools/check-cutover-ledgers.py --nextest-list rust/.scratch/review248-r2/ignored-list.json
python3 rust/ci/ignored_tests.py
python3 rust/ci/ignored_tests.py --nextest-list rust/.scratch/review248-r2/ignored-list.json
python3 -m unittest discover -s rust/ci -p test_ignored_tests.py -v
```

Raw summary lines:

```text
     Summary [  47.415s] 177 tests run: 177 passed, 2877 skipped
     Summary [  48.559s] 2 tests run: 2 passed, 3052 skipped
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 09s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 16.51s
Scope control picker: survived before; rejected after at required ancestor; 0 invalid controls
Scope control tour: survived before; rejected after at required ancestor; 0 invalid controls
Scope control status: survived before; rejected after at required ancestor; 0 invalid controls
Scope mutation receipts: 3 surviving before; 3 rejected after; 0 invalid controls
Controller per-assertion receipts: 120 audited declarations (120 closed, 0 reopened); 401 assertion sites; 74 enabled native test identities passed; 0 explicit reopened gaps; 0 unaccounted assertions
Controller per-assertion receipts: 93 audited declarations (93 closed, 0 reopened); 279 assertion sites; 82 enabled native test identities passed; 0 explicit reopened gaps; 0 unaccounted assertions
Original browser per-assertion receipts: 27 declarations; 147 direct sites; 42 private-helper expansions; 4 setup assertions; 18 paired executions passed; 9 producer defects rejected; 0 unaccounted assertions
Ignored-test guard: 21 CI correctness tests, 7 utilities; 0 unowned
Compiler ignored-test guard: 21 CI correctness tests, 6 utilities; 0 unclassified
Ran 1 test in 0.110s
OK
Compiler ignored-test guard: 8 CI correctness tests, 0 utilities; 0 unclassified
Ran 6 tests in 1.083s
OK
Compiler ignored-test guard: 1 CI correctness tests, 0 utilities; 0 unclassified
Cutover ledger receipts: 25 historical CI test identities still enabled; 14 WS17 closures; 3 ignored browser registrations for rust/ci-full-gate; 233 broad WS8 closures; 1 approved queue supersession; 0 inconsistent records
Cutover ledger remains partial: 104 broad receipts; 0 sidebar receipts; 6 overlapping criteria; 1 muted browser; 1 Calendar browser; 3 geometry-only exclusions
Original browser pickers: 5 producer defects rejected; 0 invalid controls
Original browser tours: 4 producer defects rejected; 0 invalid controls
WS8br2 browser picker: 5 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF
WS8br2 browser picker: 5 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF
WS8br2 browser tour: 4 passed; 0 failed; Chromium 153.0.8010.12; keyboard, persistence and help-menu behavior; real signed sessions and CSRF
WS8br2 browser tour: 4 passed; 0 failed; Chromium 153.0.8010.12; keyboard, persistence and help-menu behavior; real signed sessions and CSRF
WS8br2 browser status: 5 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF; member-panel integration deferred
WS8br2 browser status: 5 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF; member-panel integration deferred
```

The branch-sweep guard additionally verifies B's retained 22 mutation receipts;
those are historical B evidence, not newly executed controls in this round.
Local scope controls are 3/3 rejections and local standard controls are 9/9.
Legacy external receipts total 28 paired cases (10 picker + 8 tour + 10 status);
the two registered wrappers total 18 paired cases. No new ledger declaration is
closed: the exact remainder stays 104 broad receipts, six overlapping criteria,
one muted-room sequence and one Calendar sequence, with the existing three
geometry-only exclusions. Current CI results belong to the newly pushed head;
historical job receipts remain historical.

All targets created by this round and its six older-runner scratch directories
are removed after pushing; small raw logs remain in rust/.scratch/review248-r2.
No stash was used, and only C is pushed in this round.
