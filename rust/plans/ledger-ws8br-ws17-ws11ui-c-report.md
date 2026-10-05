# Original assertion reconciliation — C checkpoint

Stacked on PR #244's assertion-audit head `f05c3f2a5146a586a80149840f79f2ea52c6d87c`.
Its 120-declaration audit preserves 112 closures and eight reopened records, rather
than claiming missing assertions passed. This checkpoint closes all eight reopened declarations, including the Node subprocess exit/output assertion through the served service-worker action.

The first C batch closes 83 broad original declarations and all eight sidebar
original declarations: 91 declarations, 269 direct assertion sites and 15 private
PNG-helper assertion expansions. The JSON manifest and human table name every
original assertion and its actual discriminating assertion file:line, including
multi-step requests, exact reloaded audit fields/count deltas, media headers/bytes,
permission statuses, redirect destinations, CSV fields, and complete scoped DOM.
Historical file-level Rails runs remain historical; new enabled Rust test identities
are checked against actual current nextest output and the current test list.

Two missing behaviours are fixed: configured sidebar DM rows now use Rails'
membership/participant collection cache, preserving a renamed peer until the
participant key changes; icon creation/deletion audits now store `:shortcode:`
labels as Rails does. The original sidebar cache sequence and Acme upload/list/
delete test failed before these fixes. No assets or Rails production code changed.

Actual producer controls rejected 40 distinct tests in the first batch, 29 in the
second and five in the third. The header test overlaps these groups; totals are
reported by batch, not as unique counts. Compilation/transport/setup failures are
invalid controls. A partial-only Google fragment mutation survived because another
real status partial correctly rendered the same email; that control was rejected
as insufficient, then replaced with a mutation of the common email reader. All five
profile/header defects now fail the intended original assertions. Source restoration
is guaranteed in each control's `finally` block.

The mixed quiet-room request stays flat: Rust 23 → 23 SELECTs, Rails 22 → 22;
HuddleGrant source reads stay at one. These are the original total-read-growth
and single-grant-query clauses; the one fixed additional Rust read is explicit.

Remaining after the 120-declaration checkpoint: 104 broad browser declarations, no sidebar receipts, six overlapping criteria,
the muted-room and Calendar sequences, and aggregate mappings. All are individually
listed in the remaining manifest. The 81 API comparisons remain with WS11-API;
the three geometry-only exclusions retain their existing dispositions. No missing
acceptance evidence is relabelled an absent domain API.

Completed commands and fresh-clone verification follow at the final checkpoint.

First batch restored baseline and discrimination receipts:

```text
     Summary [  43.923s] 108 tests run: 108 passed, 4958 skipped
Controller per-assertion receipts: 91 audited declarations (91 closed, 0 reopened); 269 assertion sites; 80 enabled native test identities passed; 0 explicit reopened gaps; 0 unaccounted assertions
C receipt discrimination: 19 producer mutations; 40 distinct tests rejected; 0 invalid controls
C audit/media discrimination: 17 producer mutations; 29 distinct tests rejected; 0 invalid controls
C profile discrimination: 5 producer mutations; 5 tests rejected; 0 invalid controls
```

Commands: the three committed `check-controller-c-*-controls.py` scripts; restored
`cargo nextest run --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever -j 4 --no-fail-fast -E 'test(original_) or test(controllers::users::ban_lifecycle_tests) or test(controllers::accounts::icons::tests) or test(controllers::accounts::logos::tests) or test(ws15e_rails_composer) or test(controllers::public_pages::tests::public_page_bodies) or test(controllers::users::profile_settings_tests)'`;
`cargo nextest list --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --message-format json`;
`python3 rust/reference-tools/check-controller-assertion-receipts.py --manifest rust/plans/ledger-ws8br-ws17-ws11ui-c-receipts.json --nextest-list <list.json> --native-log <restored.log>`.

Second batch adds the two original per-test CSV export-cap overrides and 27
browser declarations: 18 people/group/picker/mobile interactions, four tours,
four starred-people interactions, and the original Node event-harness receipt.
The total is **120 declarations** (112 broad and eight sidebar), **420 direct
Rails assertion sites**, **54 private-helper expansions**, and four setup
assertions. The controller manifest accounts for 93 declarations/279 sites and
82 distinct enabled native identities. The browser manifest accounts for 27
original declarations/141 sites through seven explicitly enabled ignored tests
through `rust/parity/system/ws12`. Future CI ownership is PR #237
(`rust/ci-full-gate`); its exact ignored-test registry must add these seven names.
Current main toolchain CI skips them, so this report credits the executed paired
local receipt and does not claim a completed CI browser run. Both targets execute all 27 cases (26 Chromium scenarios and one served-worker
Node event harness per target);
stateful DM recipients/canonical destinations, group departure and tour stamps
are checked on each actual server database after the browser action.

The new browser driver uses the existing network-none container and unchanged
HTTP/WebSocket byte-forwarder. This causally avoids Chromium's host-namespace
`ERR_NETWORK_CHANGED` when other workers start Docker containers. An unrelated
seed rich-text preview pointing to an external Twitter image is made inert on
both targets; these original cases do not exercise that preview. The original
0.7-second touch hold and finite member-panel animation condition are preserved.
No retries or widened deadlines were added. Visible label versus accessible-name
selector mistakes were corrected in the harness, with the original assertions
unchanged. The star controls mutate the actual polled members payload, preserving
all other fields and the real HTTP producer, so periodic refresh cannot hide the
defect. Invalid/surviving controls (including an initially selected hidden picker
controller, a nonexistent refresh method and an unconfigured mutant queue) are
excluded and were replaced by controls that reach the intended assertions.

The aggregate mappings retain their history and explicitly credit the eight
sidebar controller originals while keeping the phone/header/member/pins browser
aggregates partial. None are closed by a file-level controller receipt. The six
overlapping criteria still needing acceptance evidence are group-huddle ringing,
colon brand-icon autocomplete in both themes, room-icon sidebar/header rendering,
icon-room search-arrow suppression, LobeHub icons in both themes, and workspace
icon upload/post/delete fallback in both themes. Real owner APIs are present;
these are missing named acceptance receipts, not absent-owner API claims.
The 104 exact broad declarations are listed by name in the remaining JSON.
Muted-room noise -> mention delivery and Calendar injected-clock busy -> clear
remain separate missing sequences. The 81 WS11-API comparisons stay with their
owner; the three geometry-only exclusions retain their prior dispositions.

Fresh-clone test, clippy, release-input and new browser/Node receipt summaries follow
below after verification. The only seeds built for that clone are `default`,
`first_run` and `agents_ui`, matching `.github/workflows/rust.yml`.

Additional restored-baseline and actual producer-control summaries:

```text
     Summary [   1.358s] 2 tests run: 2 passed, 2950 skipped
Original audit cap discrimination: 3 producer mutations; 2 distinct tests rejected; 0 invalid controls
C lifecycle discrimination: 6 producer mutations; 6 tests rejected; 0 invalid controls
Original browser people: 10 producer defects rejected; 0 invalid controls
Original browser pickers: 5 producer defects rejected; 0 invalid controls
Original browser members: 2 producer defects rejected; 0 invalid controls
Original browser group: 1 producer defects rejected; 0 invalid controls
Original browser tours: 4 producer defects rejected; 0 invalid controls
Original browser stars: 4 producer defects rejected; 0 invalid controls
Original browser worker: 1 producer defects rejected; 0 invalid controls
```

The four native control groups reject all 80 first-batch native identities; the
cap controls add both new identities. Exact assertions are preserved after
restoring producer bytes. CSV limit parameterization is a pure extraction; its
per-request override is compiled only in tests and follows the normal router,
CSRF, permissions, sudo, selection and rendering path.

The first full fresh-clone run found a real cache-boundary regression in
`controllers::rooms::row_broadcast_tests::composed_sidebar_broadcast_rows_match_sixteen_complete_rails_renders`:
the second standalone DM row incorrectly reused the first row's unread/muted
classes. Rails caches the direct membership **collection** in
`app/views/users/sidebars/show.html.erb:163`; its standalone broadcast/favorite/
category partial renders are uncached. The production fix places cache lookup
only in `Shell::direct_rows`, leaving `Row::render`/`render_fragment` fresh. The
existing sixteen-complete-response oracle is unchanged. That failing-first run
was stopped after confirming the defect; the fresh workspace is rerun in full
after the fix, with the same four workers and unchanged assertions/deadlines.

Future CI handoff is explicit in `ledger-ws8br-ws17-ws11ui-c-ci-handoff.json`: PR #237 must append the seven exact browser-wrapper records to its ignored-test registry and build the normal binary before running them. Current main's Rust toolchain job skips these wrappers. The existing shell entry point builds that binary and executes them; the local paired receipt is credited here. Each wrapper now has an explicit function name recognized by the future ignored-test lexer, rather than a macro placeholder. Mode-specific three-port leases keep separate fixture/server namespaces.

Final checkpoint verification

Merged main `5f5c18635` with merge commit `5aa58cecb`, keeping both test registrations in the only conflict (`controllers.rs`). Production sources were verified in the independent no-hardlink fresh clone with only CI's `default`, `first_run`, and `agents_ui` seeds and the pinned media runner. Subsequent changes are confined to explicit ignored-test functions, driver port leases/diagnostics, and receipt metadata; all seven wrappers were then rerun at four workers and strict clippy rerun. No production asset, Rails application, CodeQL configuration, deadline, or assertion was changed.

Commands run from that clone's `rust/` directory:

```sh
cargo nextest run --locked --workspace --exclude html5ever -j 4 --no-fail-fast
cargo test --locked --workspace --exclude html5ever --doc -- --test-threads=4
cargo clippy --locked --workspace --exclude html5ever --all-targets -- -D warnings
./ci/with-release-inputs.sh cargo build --locked -p campfire --bin campfire
cargo build --locked -p campfire --bin campfire
cargo test --locked -p campfire controllers::ws11ui_original_browser_tests -- --ignored --show-output --test-threads=1
cargo nextest run --locked -p campfire -j 4 --no-fail-fast --run-ignored only --success-output final -E 'test(controllers::ws11ui_original_browser_tests)'
cargo nextest run --locked -p campfire -j 4 -E 'test(original_mixed_quiet_sidebar_read_counts)' --success-output immediate
cargo nextest list --locked --workspace --exclude html5ever --message-format json
for mode in people pickers members group tours stars worker; do
  python3 reference-tools/users/run_original_browser_assertions.py "$mode" --controls
done
```

`CAMPFIRE_REFERENCE` points at that fresh clone; `CARGO_TARGET_DIR` uses the pre-existing shared target; `WS11UI_BROWSER_BINARY` points at the normal binary built above. Rustc retains its configured slot throttle. The browser/Node runs use the pinned Rails image `review236-reference:78b9b1546` and pinned Playwright image.

Raw summaries:

```text
     Summary [1329.556s] 5074 tests run: 5074 passed (1 slow), 27 skipped
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.31s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 16s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 2959 filtered out; finished in 270.47s
     Summary [  80.249s] 7 tests run: 7 passed (1 slow), 2959 skipped
Original sidebar reads: Rust 23 -> 23; Rails 22 -> 22; huddle_grants 1
     Summary [   0.879s] 1 test run: 1 passed, 2965 skipped
```

The two `Finished` lines correspond to final strict clippy (zero warnings/errors) and the release-input production build. Doctests have 0 passed/0 failed/2 ignored across eleven crate suites; the raw nonzero-ignored crate line is:

```text
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Failing-first and non-passing history is retained:

```text
     Summary [1430.227s] 2542/5060 tests run: 2537 passed (6 slow), 5 failed, 27 skipped
     Summary [1609.118s] 5060 tests run: 5060 passed (5 slow), 27 skipped
     Summary [  86.132s] 7 tests run: 6 passed (1 slow), 1 failed, 2959 skipped
```

The first workspace line contains one genuine standalone-DM cache regression and four cancellations when that run was stopped to fix it. The corrected pre-main run passed all 5,060; the merged-main run then passed all 5,074. Strict clippy initially rejected a contains-key/insert test setup; the entry API fixes that style issue without moving any assertion anchors, and the affected fifteen original room-audit tests pass. The first four-worker browser probe had one picker host startup exit, before any browser assertion; it is not credited as a mutation rejection or a passing receipt. Its log was lost during cleanup. Startup errors now retain the server log contents. The diagnostic run passed all seven at the unchanged four-worker limit, so the original exit's cause remains unconfirmed and is flagged for the CI handoff. There are no automatic retries or widened waits; the successful serial receipt and the diagnostic run do not erase that failed probe.

Nine pinned Rails corpora were regenerated in the fresh clone and compared byte-for-byte: account originals, audit originals, audit-cap originals, avatar originals, icon originals, profile gaps, sidebar originals, human profile originals, and room audit originals. Each regenerated unchanged.

The current controller and browser validators check all source hashes, declaration assertion sets, exact assertion anchors, enabled/ignored registrations, successful current-run identities, per-target original assertion traces, actual persisted DB observations, and all twenty-seven intended producer rejections. Raw summaries:

```text
Controller per-assertion receipts: 120 audited declarations (112 closed, 8 reopened); 399 assertion sites; 70 enabled native test identities passed; 11 explicit reopened gaps; 0 unaccounted assertions
Controller per-assertion receipts: 93 audited declarations (93 closed, 0 reopened); 279 assertion sites; 82 enabled native test identities passed; 0 explicit reopened gaps; 0 unaccounted assertions
Original browser per-assertion receipts: 27 declarations; 141 direct sites; 39 private-helper expansions; 4 setup assertions; 54 paired executions passed; 27 producer defects rejected; 0 unaccounted assertions
Cutover current branch: 181 credited test identities passed in the supplied current workspace run
Cutover ledger receipts: 25 historical CI test identities still enabled; 14 WS17 closures; 3 ignored browser registrations for rust/ci-full-gate; 233 broad WS8 closures; 1 approved queue supersession; 0 inconsistent records
Cutover ledger remains partial: 104 broad receipts; 0 sidebar receipts; 6 overlapping criteria; 1 muted browser; 1 Calendar browser; 3 geometry-only exclusions
```

The first controller summary deliberately preserves #244's historical eight reopenings; the C mappings close those eight with new evidence. Browser receipt validation credits current local execution; future CI registration is separately pending as described above. This is the requested 120-declaration checkpoint, not a claim that the remaining ledger is complete.

## PR #248 review follow-up

The preceding summaries are historical. The current pinned assertion mappings,
causal port-lease fix, stronger selector/visibility predicates, failing-first
producer controls and fresh verification are recorded in
[ledger-ws8br-ws17-ws11ui-c-review-report.md](ledger-ws8br-ws17-ws11ui-c-review-report.md).
The current 120-declaration mapping contains 426 direct assertion sites against
the runtime Rails pin, including six predicates absent from the historical
420-site mapping. The per-declaration selector scan is recorded separately.

Exact final validator commands, run from the worktree root:

```sh
python3 rust/reference-tools/check-controller-assertion-receipts.py --manifest rust/plans/ledger-ws8br-ws17-ws11ui-b-receipts.json --nextest-list .scratch/ledger-audit/c-merged-final-list.json --native-log .scratch/ledger-audit/c-merged-workspace.log
python3 rust/reference-tools/check-controller-assertion-receipts.py --manifest rust/plans/ledger-ws8br-ws17-ws11ui-c-receipts.json --nextest-list .scratch/ledger-audit/c-merged-final-list.json --native-log .scratch/ledger-audit/c-merged-workspace.log
python3 rust/reference-tools/check-original-browser-receipts.py --nextest-list .scratch/ledger-audit/c-merged-final-list.json --browser-log .scratch/ledger-audit/c-browser-startup-diagnostics.log --controls-dir .scratch/ledger-audit
python3 rust/reference-tools/check-cutover-ledgers.py --nextest-list .scratch/ledger-audit/c-merged-final-list.json --native-log .scratch/ledger-audit/c-merged-workspace.log
```
