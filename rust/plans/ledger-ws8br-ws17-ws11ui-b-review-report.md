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

## Third re-review: complete HTTP branch/setup sweep

Review base: `1b83dda5a39c28b2fb7765184437fdb19ee0eca9`; source and runtime pin: `78b9b1546bdab4c6c1c9b8ddb94512f661289112`. This round changes enabled tests, Rails oracle tools/vectors and assertion receipts. Production behavior already matches these original probes and is unchanged. Only B/#244 is updated; C/#248 remains at `f50d735d181be0f88032d07d37eb01548919ca4b` with no tracked modifications.

The three review gaps are now directly discriminated:

- P0100: cookie-free HEAD on /about, /privacy and /terms checks 200 plus zero body bytes; authenticated HEAD coverage remains.
- P0155: meeting status on, no GoogleAccount, no cache and calendar OOO off; the real GET checks status and the exact original Reconnect below anchor href/text/count.
- P0173: the original rejected openid/email/Calendar/Drive grant is persisted before the real GET; full section bytes and form ancestry/method/data-turbo plus exactly one hidden Drive feature are checked.

Every one of the 120 pinned declaration bodies and its helper/setup branches was read. The full request/source inventory and all 401 assertion mappings are in `ledger-ws8br-ws17-ws11ui-b-branch-sweep.{json,md}` and the existing per-assertion map. Additional sweep findings:

- P0113: own-user public GET, previously only peer and bot GETs.
- P0128: role omitted on the original signup, in addition to the retained escalation-input negative case.
- P0148/P0149, P0176, P0184/P0186, P0187–P0191, P0193/P0195/P0197/P0199: original PUTs now execute in addition to PATCH. The original John Doe/Acrobat, foreign John Doe, push-to-talk/CapsLock and dark/Pacific form pairs are retained. Existing notification/text-size original tests already used PUT and were not counted as new gaps.
- P0153/P0165/P0167: original legacy-account email david@gmail.test; P0168: original full identity+Calendar+Drive grant; P0169: original retired metadata grant; P0166/P0174: rejected legacy grant with NULL scopes; P0172: partial openid/email/Drive grant. Capability-equivalent prior rows remain additional coverage.
- P0163: manual-only OOO with notifications retained, rather than also enabling calendar OOO.
- P0175: absent → legacy Calendar → full identity/Calendar/Drive layout states.
- P0202: disabling quiet hours retains the stored UTC zone.
- Previously reopened exact-value gaps now execute: P0102 Acme Widgets/privacy@example.com; P0154 the actual UNREACHABLE_MESSAGE; P0165 david@gmail.test; P0176 spaced David-GH; P0186 empty-string clearing after david-gh; P0190 david@smartdata.net; P0191 Dave/case-only original email; P0276 original Node harness exit and all-checks stdout on the actual HTTP worker.

All eight formerly reopened B declarations are closed with their old insufficient evidence preserved in history. The independent remainder is 208 broad declarations, eight sidebar declarations, fourteen overlapping criteria, one muted-room sequence, one Calendar sequence and aggregate mappings. This round makes no closure claim for those other records.

Mutation results are listed individually in the sweep table and `reference-tools/users/review244_branch_mutations.json`. Against the reviewed old assertions, 21 controls survive and one (service-worker cache gate) is already killed by byte equality. All 22 current controls reject actual producer defects at assertion failures. A repeated run using the committed runner also rejects all 22. Pure presenter fixtures that remain green under a loader mutation are not credited as discrimination of that loader. Invalid setup/compile attempts were corrected and excluded from the recorded results. Every producer is restored byte-for-byte; public/PWA production prefixes are unchanged.

Rails generation used the pinned Docker image `review236-reference:78b9b1546`, isolated namespace/owner ws244r3, copied validated CI seeds and frozen 2026-03-02T16:00:00Z. Default/first_run/agents_ui seed readback reports 29/29, 4/4 and 40/40 checks. The full original Google helper scope strings are extracted with their current-pin source hash; the original Node harness is an exact copy of the pinned test script. The new structural checker verifies both copies. It is an integrity check, not a substitute for the executed differentials.

Rails oracle commands (executed from the B worktree; log redirection omitted):

```sh
export PARITY_IMAGE=review236-reference:78b9b1546 PARITY_OWNER=ws244r3 PARITY_NAMESPACE=ws244r3
for tool in profile_sections public layout_preferences profiles profile_reconnect_receipts; do
  bash rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor "/work/reference-tools/users/$tool.rb"
done
bash rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- env PROFILE_RECEIPT_VERB=put bin/rails runner --skip-executor /work/reference-tools/users/profiles.rb
bash rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- env WS9_PROFILE_SECURITY_VECTORS=/work/.scratch/review244-r3/profile_security.json bin/rails runner --skip-executor /work/reference-tools/auth/profile_security.rb
bash rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- env PROFILE_RECEIPT_VERB=put WS9_PROFILE_SECURITY_VECTORS=/work/.scratch/review244-r3/profile_security_put.json bin/rails runner --skip-executor /work/reference-tools/auth/profile_security.rb
bash rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/users/service_worker_original_probe.rb
node rust/.scratch/review244-r3/rails-worker-source/test/scripts/service_worker_harness.mjs
```

The last Node command runs in the canonical correctness image (Node 26.10.0), on the actual Rails HTTP body staged by the preceding command; the Rails runtime does not supply Node. Corpus verification compares all newly recorded vectors plus unchanged prior rows, including untouched security failure/transfer/window corpora.

Local verification uses the private normal correctness Docker runner with Rust 1.98.1, CI=true, nextest -j 4 and the unchanged shared four-slot compiler throttle. No local full workspace test suite or release-input build was run this round. Commands inside that runner (or host Python where shown):

```sh
cargo nextest list --manifest-path rust/Cargo.toml --locked -p campfire -p campfire_db --message-format json > rust/.scratch/review244-r3/list.json
python3 rust/reference-tools/users/review244_targeted_filter.py rust/.scratch/review244-r3/list.json > rust/.scratch/review244-r3/targeted.filter
cargo nextest run --manifest-path rust/Cargo.toml --locked -p campfire -p campfire_db -j 4 --no-fail-fast -E "$(cat rust/.scratch/review244-r3/targeted.filter)"
python3 rust/reference-tools/users/check_review244_branch_mutations.py --exec-wrapper rust/.scratch/review244-r3/exec-throttled.sh --output rust/.scratch/review244-r3/final-controls
cargo clippy --manifest-path rust/Cargo.toml --locked -p campfire --all-targets -- -D warnings
python3 rust/reference-tools/check-controller-assertion-receipts.py --nextest-list rust/.scratch/review244-r3/list.json --native-log rust/.scratch/review244-r3/targeted.log
python3 rust/reference-tools/check-cutover-ledgers.py --nextest-list rust/.scratch/review244-r3/list.json --native-log rust/.scratch/review244-r3/targeted.log
python3 rust/reference-tools/check-controller-branch-sweep.py
python3 rust/reference-tools/users/verify_review244_corpora.py rust/.scratch/review244-r3
```

The 118 selected tests include all 104 ledger-credited identities plus other tests in the touched modules. The per-assertion checker credits 74 enabled identities across the 120 declarations. Unselected nextest counts are not skips of selected tests. The initial affected-module run is 60/60; the final restored-producer run selects the 19 distinct identities used by the 22 controls.

Raw positive and checker summary lines:

```text
     Summary [  28.867s] 60 tests run: 60 passed, 2921 skipped
     Summary [  41.923s] 118 tests run: 118 passed, 4308 skipped
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 09s
Controller per-assertion receipts: 120 audited declarations (120 closed, 0 reopened); 401 assertion sites; 74 enabled native test identities passed; 0 explicit reopened gaps; 0 unaccounted assertions
Cutover current branch: 104 credited test identities passed in the supplied current workspace run
Cutover ledger receipts: 25 historical CI test identities still enabled; 14 WS17 closures; 3 ignored browser registrations for rust/ci-full-gate; 129 broad WS8 closures; 1 approved queue supersession; 0 inconsistent records
Cutover ledger remains partial: 208 broad receipts; 8 sidebar receipts; 14 overlapping criteria; 1 muted browser; 1 Calendar browser; 3 geometry-only exclusions
Controller branch sweep: 120 current-pin declarations; 401 exact assertion mappings; all original HTTP request lines recorded; 0 source/setup drift
Original helper scopes and Node harness: byte-identical to current Rails pin
Controller branch controls: 21 prior survivors; 1 already rejected; 22/22 current assertion rejections; 0 invalid results credited
users_profile_sections: 10 prior rows unchanged; 16 current Rails rows identical
users_public: 5 prior rows unchanged; 6 current Rails rows identical
users_layout_preferences: 34 prior rows unchanged; 38 current Rails rows identical
users_profile_settings: 31 prior rows unchanged; 37 current Rails rows identical
profile_security: 15 prior rows unchanged; 20 current Rails rows identical
Rails PATCH/PUT: 37 settings + 20 security cases; 0 response/state mismatches
Rails reconnect original setups: 2/2 identical; 0 mismatches
```

Raw repeated mutation summary lines (intentional failures):

```text
anonymous-head: Summary [   0.644s] 1 test run: 0 passed, 1 failed, 2980 skipped
missing-meeting: Summary [   0.757s] 1 test run: 0 passed, 1 failed, 2980 skipped
rejected-full-drive: Summary [   1.462s] 2 tests run: 1 passed, 1 failed, 2979 skipped
profile-put: Summary [   0.746s] 3 tests run: 0 passed, 3 failed, 2978 skipped
active-full-drive: Summary [   1.286s] 2 tests run: 1 passed, 1 failed, 2979 skipped
partial-drive-identity: Summary [   1.349s] 2 tests run: 1 passed, 1 failed, 2979 skipped
retired-full-grant: Summary [   1.514s] 2 tests run: 1 passed, 1 failed, 2979 skipped
rejected-legacy: Summary [   1.403s] 2 tests run: 1 passed, 1 failed, 2979 skipped
original-calendar-email: Summary [   1.255s] 1 test run: 0 passed, 1 failed, 2980 skipped
own-show: Summary [   0.670s] 1 test run: 0 passed, 1 failed, 2980 skipped
signup-default: Summary [   1.389s] 1 test run: 0 passed, 1 failed, 2980 skipped
quiet-off-utc: Summary [   0.919s] 1 test run: 0 passed, 1 failed, 2980 skipped
manual-only-ooo: Summary [   0.787s] 1 test run: 0 passed, 1 failed, 2980 skipped
layout-full-drive: Summary [   1.033s] 1 test run: 0 passed, 1 failed, 2980 skipped
appearance-combined: Summary [   0.749s] 1 test run: 0 passed, 1 failed, 2980 skipped
configured-original: Summary [   0.973s] 1 test run: 0 passed, 1 failed, 2980 skipped
original-fetch-error: Summary [   0.750s] 1 test run: 0 passed, 1 failed, 2980 skipped
clear-existing-login: Summary [   0.755s] 1 test run: 0 passed, 1 failed, 2980 skipped
original-github-link: Summary [   0.758s] 1 test run: 0 passed, 1 failed, 2980 skipped
original-correct-email: Summary [   3.402s] 1 test run: 0 passed, 1 failed, 2980 skipped
original-case-only: Summary [   4.178s] 1 test run: 0 passed, 1 failed, 2980 skipped
worker-cache: Summary [   0.679s] 1 test run: 0 passed, 1 failed, 2980 skipped
```

Final restored-producer command and raw summary:

```sh
cargo nextest run --manifest-path rust/Cargo.toml --locked -p campfire -j 4 --no-fail-fast -E "$(cat rust/.scratch/review244-r3/restored.filter)"
```

`restored.filter` is the sorted union of `after_tests` in the committed mutation manifest (19 distinct identities).

```text
     Summary [  28.938s] 19 tests run: 19 passed, 2962 skipped
```
