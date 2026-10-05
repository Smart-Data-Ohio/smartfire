# PR #244 review fixes

Review base: `f05c3f2a5146a586a80149840f79f2ea52c6d87c`. Rails reference:
`78b9b1546bdab4c6c1c9b8ddb94512f661289112` from `rust/parity/reference.sha`.
Work is confined to the requested separate `rust-ws11ui-b` worktree and `rust/` inputs.

P0197's `not_set` vector now starts with America/New_York already saved and the
explicit marker true. The Rails generator produces the same redirect and saved
state from that populated setup. The cited vector test detects an omitted clear;
the original choose/detect/clear sequence additionally reads `(None, true)` directly.

P0106 and the analogous P0105 check parse the actual sign-in GET response. They
require the original nav ancestry and real anchors with exact href/target/rel;
a commented anchor or a section masquerading as the nav cannot satisfy them.
The synthetic complete-byte comparison remains additional evidence.

P0207's peer status-link absence and P0210's own status href without
`data-turbo-frame` are explicitly asserted through the actual card route and mapped.
The complete HTTP DOM oracle is retained. Every one of the 120 source declarations,
401 assertion literals/coordinates, file hashes and current Rust anchors is checked
against the current pin by `check-controller-assertion-receipts.py`; it now rejects
a historical reference, stale declaration coordinates, duplicate lines and conflicting
Rust source pointers. Historical maps remain in Git/JSON history.

The 120-record setup/selector scan is in
`ledger-ws8br-ws17-ws11ui-b-review-selector-scan.md`. Additional fixes cover exact
Ban Kevin text, directory row minimum/presence/badge selectors, Calendar connect
and reconnect anchors, image-label absence and the light color-scheme meta.
A global `JZ` source-substring check is replaced by Rails' directory-row text
selector. An old-check mutation invocation hit that unrelated global assertion;
it is not counted as discrimination of the injected defect. The earlier isolated
original-test directory run is the valid before receipt.

No production Rust behavior, assets, expected response bytes, deadlines or worker
counts changed. The media runner accepts an explicit scratch path inside `rust/`
so verification does not write outside the authorized tree.

## Mutation receipts

`ledger-ws8br-ws17-ws11ui-b-review-mutations.json` records each named identity and
raw before/after summary. Fourteen isolated producer mutations cover ten real-path
tests: eleven survived the original checks and all fourteen fail at the intended
checks after strengthening. The image-label and two card controls already failed
before the change. Compilation/setup/transport failures do not count as kills.

The reproducible control runner is
`rust/reference-tools/check-controller-review244-controls.py`; its `finally` restores
writers, renderers, and (for `before`) the exact reviewed test/vector bytes.

```sh
python3 rust/reference-tools/check-controller-review244-controls.py after
```

Raw validated comparison:

```text
PR244 mutation comparison: 11 survived before; 14 rejected after; 10 distinct tests
PR244 review controls: 14 actual producer defects rejected by 10 distinct tests; 0 invalid controls
Current-pin source refresh: 120/120 declarations; 401 assertion sites; 2 added card assertions; 0 unresolved coordinates
PR244 setup/selector scan: 120 records; 10 strengthened mapping/setup/selector records; 8 existing reopened records retained
```

## Rails receipts

The pinned `review236-reference:78b9b1546` image rebuilt exactly the three CI seeds:
`default`, `first_run`, `agents_ui`. No test reads private local seed state;
`CI=1` makes a missing seed fail.

```sh
rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/users/profiles.rb
rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/users/sign_in_google.rb
rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/users/people.rb
```

Raw generator summaries:

```text
Rails profile settings oracle: 31 PATCH cases; reference 78b9b1546bdab4c6c1c9b8ddb94512f661289112
Rails Google sign-in display oracle: 8 complete sign-in bodies and configuration cases; reference 78b9b1546bdab4c6c1c9b8ddb94512f661289112
Rails people oracle: 14 cards, 2 directories; reference 78b9b1546bdab4c6c1c9b8ddb94512f661289112
```

Sign-in and people corpora regenerate byte-identically. The profile corpus changes
only `not_set.before`; all 31 status/saved-state expectations are unchanged. Commands
use `PARITY_RUNTIME=docker`, the pinned image, and a private `ws11ui-rv244` namespace.

## Verification

Commands run from the new worktree with `CI=1`, `RUST_TEST_THREADS=4`, the configured
machine-wide rustc throttle, no extra build jobs, and the pinned application/storage
media runner. `PINNED_MEDIA_SCRATCH` stays inside `rust/`. The shared target is removed
when both branches' verification finishes.

```sh
cargo nextest run --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever -j 4 --no-fail-fast
cargo clippy --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --all-targets -- -D warnings
cargo nextest list --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --message-format json
python3 rust/reference-tools/check-controller-assertion-receipts.py --nextest-list LIST_JSON --native-log WORKSPACE_LOG
python3 rust/reference-tools/check-cutover-ledgers.py --nextest-list LIST_JSON --native-log WORKSPACE_LOG
```

The 112 retained closures and eight reopened records are unchanged. The reopened
IDs are P0102, P0154, P0165, P0176, P0186, P0190, P0191, P0276 (eleven precise gaps).
They do not acquire credit from selector fixes or a repinned coordinate.

The source guard negative controls also ran against the actual checker:

```text
Controller source-pin guard: old d7c7de92 manifest rejected before reading receipts
Controller source-coordinate guard: one-line declaration drift rejected
```

Raw completed gate summaries (all commands exited zero):

```text
     Summary [1306.418s] 4983 tests run: 4983 passed (2 slow), 20 skipped
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 08s
Controller per-assertion receipts: 120 audited declarations (112 closed, 8 reopened); 401 assertion sites; 70 enabled native test identities passed; 11 explicit reopened gaps; 0 unaccounted assertions
Cutover current branch: 100 credited test identities passed in the supplied current workspace run
Cutover ledger receipts: 25 historical CI test identities still enabled; 14 WS17 closures; 3 ignored browser registrations for rust/ci-full-gate; 121 broad WS8 closures; 1 approved queue supersession; 0 inconsistent records
Cutover ledger remains partial: 216 broad receipts; 8 sidebar receipts; 14 overlapping criteria; 1 muted browser; 1 Calendar browser; 3 geometry-only exclusions
```

## Second re-review: P0130 and P0171

This round starts at b169b8b49 and changes test evidence, with no production
behavior changes. The reference remains 78b9b1546. P0130 now counts every input
bearing inactive JZ's `data-user-id`, without requiring a multi-select attribute
(`people_tests.rs:398`, matching Rails `users_controller_test.rb:200`).
P0171 now uses a persisted `david@gmail.test` account with exactly `openid email`
scopes. Its real profile GET checks the four original Rails assertions at
`profile_sections_tests.rs:330`, `:331`, `:332`, and `:345`, including all form
selector attributes on the same element. Unrelated connected parity-seed panels
are cleared to match the original controller-test fixtures. The new test also
compares the complete basic-identity fragment against the pinned Rails oracle.
The nine existing fragments regenerate byte-identically; the tenth is new.

Both surviving producer mutations were reproduced against the old assertions:

- An actual directory template emits an inactive-user checkbox without the
  multi-select attribute: the old test passes; the new input count fails 1 vs 0.
- The actual profile presenter considers an account connected only with Calendar
  or Drive: both old credited identities pass. With the new basic-identity state,
  the HTTP matrix fails its Disconnect check and the dedicated original-setup
  test fails the Calendar-permission warning check. The pure Drive fixture remains
  green and is not credited as discrimination of the original basic-identity case.

All three temporarily instrumented producer files were restored byte-for-byte
before verification. No production mutation hook is committed. Both receipt
maps retain the former insufficient evidence as history. Structural verification
checks all 120 declarations, all 401 Rails assertions and all 390 current Rust
assertion anchors; eleven previously reopened gaps remain unchanged. The focused
receipt checker credits only these two closures' current executed identities.

Only the two affected test modules were executed locally. The private Docker
runner uses Rust 1.98.1, the existing shared compiler-slot pool, CI=true, the
validated default seed (29 checks, zero failures), and nextest -j 4. Commands inside
that runner:

```sh
cargo nextest run --manifest-path rust/Cargo.toml --locked -p campfire -j 4 --no-fail-fast -E 'test(controllers::users::people_tests::) or test(controllers::users::profile_sections_tests::)'
cargo clippy --manifest-path rust/Cargo.toml --locked -p campfire --all-targets -- -D warnings
python3 rust/reference-tools/check-controller-assertion-receipts.py --manifest rust/.scratch/rereview244/focused-receipts.json --nextest-list rust/.scratch/rereview244/list.json --native-log rust/.scratch/rereview244/modules.log
```

Raw summary lines (negative-control failures are intentional):

```text
Before, inactive-input mutation:
     Summary [   0.743s] 1 test run: 1 passed, 2968 skipped
Before, connection-needs-scope mutation:
     Summary [   0.950s] 2 tests run: 2 passed, 2967 skipped
After, inactive-input mutation:
     Summary [   0.722s] 1 test run: 0 passed, 1 failed, 2969 skipped
After, connection-needs-scope mutation:
     Summary [   1.057s] 3 tests run: 1 passed, 2 failed, 2967 skipped
Restored producers, both affected modules:
     Summary [   4.811s] 26 tests run: 26 passed, 2944 skipped
Controller per-assertion receipts: 2 audited declarations (2 closed, 0 reopened); 13 assertion sites; 2 enabled native test identities passed; 0 explicit reopened gaps; 0 unaccounted assertions
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 10s
Rails profile sections oracle: 10 complete Google Calendar fragments; reference 78b9b1546bdab4c6c1c9b8ddb94512f661289112
```

Skipped counts above are unselected nextest cases, not skipped selected tests.
The final reply supplies the pushed heads and the two ensuing CI runs; this is
not a new local full-workspace receipt.
