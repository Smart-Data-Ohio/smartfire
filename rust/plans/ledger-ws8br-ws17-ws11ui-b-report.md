# Controller assertion receipts — 120-declaration checkpoint

Stacked on #240, `b2389a94465fb61b65003b3e791891664fbf0e0f`. Rails stays pinned to
`d7c7de92`, with the already approved layout/status drift.

## First pushed batch (historical checkpoint)

The first batch closes **57 profile declarations**. The receipt manifest lists
individual original assertion sites, the precise enabled Rust test identities,
the observed values and the pinned Rails oracle cases. The in-place ledger keeps
the previous pending disposition under history. Original Rails file-level run
counts are retained as historical evidence; they are not native execution counts.

Missing clauses now go through actual GET/PUT/PATCH requests: original notification
booleans and invalid inputs, duplicate login errors, smaller text size, default
call values, password autocomplete, nil/IANA/legacy selected zone options, exact
Google form/feature counts and redirect/email preservation. Separate complete
fragment byte oracles are retained. No production implementation changes.

The worktree controller baseline executed:

```text
     Summary [  30.489s] 81 tests run: 81 passed, 2777 skipped
```

This checkpoint is followed by the second controller batch and fresh-clone
workspace/clippy/release-input verification before the next PR. Remaining at that batch:
271 broad declarations, 8 sidebar declarations, 14 overlapping browser criteria,
muted-room sequence, Calendar sequence and the overlapping aggregate mappings.
The earlier #240 report remains historical and unchanged in its body.

## Second controller batch

The next 63 declarations bring this checkpoint to **120 declarations / 399
individual assertion sites**, across profiles (57), people (20), public pages
(18), cards (7), tours (5), PWA (5), zones (4), and account settings (4).
The 70 credited native identities are enabled in the workspace gate;
the ordinary CI gate includes them. The original Node service-worker harness is
an explicitly external receipt, run on bytes captured from the real Rust HTTP
response. Its assertions are unchanged from `d7c7de92`.

Cards and agent profiles now compare the complete real-route fragment DOM with
the pinned byte oracle, retaining every tag, attribute, text and cardinality.
Only the random CSRF input **value** is excluded from this additional HTTP DOM
comparison; none of these original clauses assert it. The independent complete
fragment byte comparisons remain unchanged. The two-room bot fixtures restore
exactly the original `and 1 more` / `and 2 more` cases. All eleven previous agent
profile vector rows regenerate unchanged; two original cases are added.

Join requests now prove the issued cookie authenticates the newly created user
through the real 2FA page. Account settings checks the exact administrator/member
divider and ordering through the actual frame. Tours recheck both the success
status and Help control after completion. Offline responses check cookie absence.

Five actual producer mutations are rejected: profile writer no-op, false default
notification reader, bot call controls, public operator escaping, and missing
tour timestamp writes. Every control compiles, executes its exact test and fails
at the intended assertion; compile/transport failures do not count. Sources are
restored in `finally`.

```text
Controller receipt control profile-save: intended assertion rejected actual producer defect
Controller receipt control profile-default: intended assertion rejected actual producer defect
Controller receipt control card-call: intended assertion rejected actual producer defect
Controller receipt control public-escaping: intended assertion rejected actual producer defect
Controller receipt control tour-save: intended assertion rejected actual producer defect
Controller receipt discrimination: 5 actual producer defects rejected; 0 invalid controls
```

No production behavior was missing in the credited clauses. This changes tests,
oracles, receipt validation and ledgers. It does not change production Rust or
vendored JS/CSS. Production read growth and byte output are unchanged.

## Exact remainder at this checkpoint

The [remaining manifest](ledger-ws8br-ws17-ws11ui-remaining.json) now lists **208**
broad declarations, **8** sidebar declarations, **14** overlapping interaction
criteria, the **muted-room** and **Calendar** browser sequences, and overlapping
phone/header/member/pins aggregate mappings. These overlap and are not additive
unique-behavior counts. The APIs are on main; their remaining reason is missing
complete per-assertion acceptance evidence. No absent-owner claim is made.
The three geometry-only exclusions and the 81 WS11 API comparisons remain with
their established dispositions/owners. None is newly credited here.

Full fresh-clone verification follows below before this branch is opened for review.

Second batch raw receipts:

```text
     Summary [  27.711s] 70 tests run: 70 passed, 2788 skipped
     Summary [   1.611s] 1 test run: 1 passed, 2857 skipped
Controller per-assertion receipts: 120 original declarations; 399 assertion sites; 70 enabled native test identities passed; 0 missing receipts
Rails original profile receipts: 3 GETs; 5 PUTs; 0 failures
service worker harness: all checks passed
```

The one-test account replay verifies the final routed frame/strong-name ordering
addition. The 70-test baseline already includes the other credited account tests.
The five producer controls and two original agent profile cases do not replace
any earlier vector row or expected response.


## Parent review fixes and assertion audit

Merged the newer #240 review head `e635996564383c24e8f6386c2be62a786d227bbf`
with a merge commit. Its complete Recorder checks and ignored browser registrations
are preserved; this branch does not push to the parent. The combined ledger checker
keeps both that gate evidence and this branch's dynamic remaining-count checks.

The final audit connects all 399 original assertion sites to their actual native
assertions and helper scopes, rather than to a nearby status assertion. It also
checks original selector identity, ancestry and cardinality: the tour shell and
Help control, Edge instructions image, DND checkbox, directory selection inputs,
accessible Message buttons, timezone select/meta and duplicate-login paragraph.
Signed-in public bytes equal anonymous bytes, and the public private-data checks
include Action Cable, prefetch and browser-upgrade output. These are test changes;
production paths, read counts, output and assets are unchanged.

All eight affected corpora regenerated unchanged from the fresh clone. The first
three use the pinned Rails image with the current schema overlay; the other five
use the verified ten-file approved `2e20b24c` status image. An initial people run
stopped at its source-hash guard because the plain pinned image lacks the approved
card partial. Using the approved image resolved the harness setup; no expected
bytes or source hashes were changed. The original eleven agent profile rows
remain unchanged, with the two original two-room rows added.

```text
Rails oracle profile_original_receipts: regenerated byte-identically; 0 differences
Rails oracle agent_profiles: regenerated byte-identically; 0 differences
Rails oracle profiles: regenerated byte-identically; 0 differences
Rails oracle people: regenerated byte-identically; 0 differences
Rails oracle profile_sections: regenerated byte-identically; 0 differences
Rails oracle preferences: regenerated byte-identically; 0 differences
Rails oracle account_views: regenerated byte-identically; 0 differences
Rails oracle public: regenerated byte-identically; 0 differences
WS8br2 status asset source verification: both Rust asset inputs match the approved Rails bytes
WS8br2 post-pin verification: all 10 approved source files match 2e20b24c byte for byte
WS8br2 worker harness provenance: pinned Rails assertions unchanged; only stdin source loading differs
```

This is the user-requested 120-item checkpoint. The 208 remaining broad declarations
are 78 controller declarations and 130 browser declarations. The named inventory
remains in `ledger-ws8br-ws17-ws11ui-remaining.json`; none is silently converted into
an owner blocker. Its eight sidebar declarations, fourteen overlapping criteria,
muted-room and Calendar sequences, and aggregate phone/header/member/pins mappings
still need their complete real-path acceptance evidence. Geometry-only exclusions
and the 81 WS11 API comparisons keep their previous dispositions/owners.


## Final fresh-clone verification

Validated source head `2963f36aafb82b178ec080f4b2fdfec50b1b16d8` in an independent `--no-hardlinks`
clone, with only `default`, `first_run` and `agents_ui` rebuilt there. `CI=1`
prevents absent seeds from silently skipping tests. The existing target was reused;
`CAMPFIRE_REFERENCE` explicitly points at the clone, and the application/storage
media groups run with the pinned media runner. `CARGO_BUILD_JOBS=2` and the
machine-wide rustc throttle stay unchanged. Native execution uses four test threads.

The native assertions are enabled for the ordinary Rust CI gate. These fresh-clone
results are local executions, not a claim that the stronger/new assertions ran in
the historical main CI run. While this PR targets its parent, the ordinary workflow
runs on retargeting to main; the parent's three ignored browser registrations keep
their separately documented `rust/ci-full-gate` entry point.

Intermediate setup runs were stopped when incorporating the parent and tightening
selectors. One compile-only test error (a missing fixture-name dereference) was
fixed. A reused cache initially referenced a deleted clone; final commands use the
explicit clean-clone path. No timing failure appeared in the completed run, and no
retry, deadline or concurrency relaxation was used.

Commands below ran from the fresh clone root with the environment described above:

```sh
cargo nextest run --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever -j 4 --profile ci --no-fail-fast
cargo clippy --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --all-targets -- -D warnings
bash rust/ci/with-release-inputs.sh cargo build --locked --workspace --bins
cargo test --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --doc -- --test-threads=4
cargo nextest list --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --message-format json
```

Raw summary lines (clippy and release-input build both exited zero):

```text
Summary [2198.109s] 4961 tests run: 4961 passed (15 slow), 20 skipped
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 46s
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 38s
Doctest summary: 0 passed; 0 failed; 2 ignored; 11 crate suites
```

The two ignored doctests and twenty ignored native/browser/instrumentation tests
are unchanged by this slice. No missing-seed skip contributes to those counts.
Both validators were rerun against the final fresh test listing and completed log:

```sh
python3 rust/reference-tools/check-controller-assertion-receipts.py --nextest-list FRESH_LIST_JSON --native-log FRESH_NEXTEST_LOG
python3 rust/reference-tools/check-cutover-ledgers.py --nextest-list FRESH_LIST_JSON --native-log FRESH_NEXTEST_LOG
```

```text
Controller per-assertion receipts: 120 original declarations; 399 assertion sites; 70 enabled native test identities passed; 0 missing receipts
Cutover current branch: 100 credited test identities passed in the fresh-clone workspace run
Cutover ledger receipts: 25 historical CI test identities still enabled; 14 WS17 closures; 3 ignored browser registrations for rust/ci-full-gate; 129 broad WS8 closures; 1 approved queue supersession; 0 inconsistent records
Cutover ledger remains partial: 208 broad receipts; 8 sidebar receipts; 14 overlapping criteria; 1 muted browser; 1 Calendar browser; 3 geometry-only exclusions
```

The service-worker HTTP capture was rerun on the final fresh source. Its real
response bytes were fed to the unchanged, provenance-checked Rails Node harness:

```sh
WS17_SERVICE_WORKER_OUTPUT=WORKER_HTTP_RESPONSE cargo nextest run --manifest-path rust/Cargo.toml --locked -p campfire -j 4 -E 'test(=controllers::pwa::tests::ws17_service_worker_is_served_byte_identical_to_rails)'
node rust/reference-tools/users/service_worker_harness.mjs < WORKER_HTTP_RESPONSE
```

```text
Summary [   0.754s] 1 test run: 1 passed, 2857 skipped
service worker harness: all checks passed
```

For the eight corpora, each generator ran with `reference runner --seed default
--time 2026-03-02T16:00:00Z --freeze`, followed by a raw `cmp` against its tracked
vector. The individual generator paths are `rust/reference-tools/users/` plus
`profile_original_receipts.rb`, `agent_profiles.rb`, `profiles.rb`, `people.rb`,
`profile_sections.rb`, `preferences.rb`, `account_views.rb`, and `public.rb`.
No response masks or normalization were applied.

```text
Fresh Rails corpus comparison: 8 complete corpora; 0 byte differences
```

The original expressions in the receipt manifest identify the pinned assertions;
the native equivalents use their documented oracle inputs. Some pre-existing
vectors use hostile names/addresses or another unprivileged viewer to exercise
the same field/permission clauses. This is per-assertion native evidence, not a
claim that the original Rails Minitest methods ran as Rust tests. The original
membership-count scenarios and default/save/error settings have dedicated rows.


The producer-control script was repeated in the final fresh clone. All five
compiled actual producer mutations failed at their intended native checks;
no compilation or transport failure counted as a rejected mutation. `finally`
restored every source byte, and `git diff --exit-code -- rust/crates` confirmed it.

```sh
python3 rust/reference-tools/check-controller-receipt-discrimination.py
```

```text
Controller receipt control profile-save: intended assertion rejected actual producer defect
Controller receipt control profile-default: intended assertion rejected actual producer defect
Controller receipt control card-call: intended assertion rejected actual producer defect
Controller receipt control public-escaping: intended assertion rejected actual producer defect
Controller receipt control tour-save: intended assertion rejected actual producer defect
Controller receipt discrimination: 5 actual producer defects rejected; 0 invalid controls
```


After restoring the five producers, the same five native identities were rerun
at four threads and passed. This is a targeted restoration check, not a replacement
for the completed workspace run.

```text
Summary [   2.877s] 5 tests run: 5 passed, 2853 skipped
```

The private validation clone, tool download and temporary control-output directories
are removed after recording these summaries. Small raw logs remain in the worktree's
untracked `.scratch/ledger-cutover-b/`; no test consumes them.
