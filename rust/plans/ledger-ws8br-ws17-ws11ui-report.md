> Current continuation: [controller assertion receipts](ledger-ws8br-ws17-ws11ui-b-report.md)
> and [exact remaining inventory](ledger-ws8br-ws17-ws11ui-remaining.json).
> The complete #240 checkpoint below is preserved as history; its remaining counts
> are historical, not the continuation branch's active counts.

# WS8br / WS17 / WS11-UI cutover reconciliation — partial

Base: `78b9b1546bdab4c6c1c9b8ddb94512f661289112`, pinned Rails `d7c7de92`
with the already-approved controller/layout drift. Branch:
`rust/ledger-ws8br-ws17-ws11ui`. The read-only cutover inventory is retained unchanged.
This is a coherent partial PR, **not a code-complete cutover receipt**.

## Closed records

The in-place ledgers preserve their previous record fields under `history` and
explicitly separate historical execution counts from current reconciliation.
[receipts](ledger-ws8br-ws17-ws11ui-receipts.json) records every credited test identity,
its actual main-CI pass line and the original declaration it supersedes. Main CI
[37200618245](https://github.com/Smart-Data-Ohio/smartfire/actions/runs/37200618245)
executed **4,948 passes, 0 failures**; those are historical counts, not this branch's run.

- **WS17: 14 of the 15 stale records resolved, 1 exact browser receipt remains.**
  Twelve declarations map to enabled current tests: validated unique MeetingCache
  creation; all four invitation-job scenarios; all seven Recorder work/preference/
  grouping/caller-authorization clauses. The two mixed DND/group huddle declarations
  now have new enabled regressions, pinned complete Rails payload/recipient/state
  vectors, actual grant issuance, real invitation jobs and overdue resolution.
  The source-issuance mutation is rejected by both new tests. No WS13 domain logic
  is copied. The current selected total is 346/347; the earlier 332/347 checkpoint
  remains historical in `ws17-wave4-report.md`.
- **WS11-UI: all 3 original browser flags pass on Rails and Rust.** Both inbox
  declarations and the work-assignment declaration run their original interacting
  sequences on independent seed copies. Each real writer mutation is rejected at
  its intended assertion. No browser retry, deadline change or original assertion
  weakening is used. These paired sequences and their writer controls are now registered as ignored
  Rust browser tests in `controllers::ws12_browser_remaining_tests`.
  `rust/parity/system/ws12` executes them; CI execution comes via the ignored
  browser-test job supplied on `rust/ci-full-gate`. The combined-branch CI receipt
  remains pending. The native inbox/page tests cited alongside them passed
  in main CI, and are explicitly labelled supporting evidence.
- **WS8br: 9 broad original receipts superseded by current CI tests.** The four
  first-run declarations, two welcome redirects, original QR/cache response, and
  uploaded/unresizable avatar paths have explicit source/assertion and main-CI
  receipts. The avatar fallback uses the same original BMP but a different viewer
  name; that substitution is disclosed rather than called a literal Kevin replay.
- **WS8br: the queue-failure record is superseded by the approved transaction
  contract.** `controllers::rooms::queue_recovery_tests::queue_decision_keeps_atomic_http_failure_and_recovers_a_rails_tombstone`
  proves atomic HTTP rollback on durable-queue failure, recovery of an existing
  Rails-compatible tombstone, exactly one enqueue and actual deletion. Lead decision
  2 explicitly changes the Redis after-commit failure boundary. This is recorded as
  an approved difference, not exact Rails HTTP parity. Queue inspection now uses
  `TestApp::without_job_runner()`.
- **WS11 API stale prose corrected.** The public Recorder supplies the persisted
  budget reader; REST events return real authorized JSON; Drive polling tests are
  enabled; the standalone owning GitHub thread page is mounted and byte-tested.
  Current source and enabled CI test citations replace the absent-owner claims in
  `ws11api-remaining-scope.md`. Historical prose is retained as a labelled checkpoint.

The three browser flags were acceptance gaps in implemented code. The work replay's
first startup failure was caused by a seed missing the latest schema migration;
rebuilding the same CI seed set from the pinned Rails runtime plus current schema
resolves startup before any assertion. The new inbox filter replay initially
compared raw JSON against `false`; Rails stores the form value `"0"` and its typed
inbox reader returns false. The replay now uses Rails' exact false-value set, then
checks the original effective-false assertion. A producer restoring the default
preference is still rejected. Neither issue warrants a production timing patch.

New Rust test names (initial reconciliation):

- `tests::huddle_cutover_test::huddle_push_honors_dnd_with_a_starred_caller_exception`
- `tests::huddle_cutover_test::group_huddle_push_skips_dnd_and_quiet_hours_but_records_all_missed_calls`

These two tests are enabled in the ordinary workspace nextest gate; they were not
in historical main CI 37200618245. The ignored tools-only HTTP host was already
present; its optional test-only followup producer now records an actual activity
through the writer rather than inventing a response. No production endpoint or
asset changes are introduced.

## Exact remaining scope

[ledger-ws8br-ws17-ws11ui-remaining.json](ledger-ws8br-ws17-ws11ui-remaining.json)
enumerates every original name, source line and cutover P identifier. Counts below
overlap; narrower criteria/browser entries must not be added to the broad ledger
as unique behaviours. No stale “owner API unmerged” explanation remains active.

| Ledger group | Still open | Actual reason |
| --- | ---: | --- |
| WS8br broad original declarations | 328 | Complete file-level/named receipt reconciliation not completed in this slice; existing HTTP/component passes do not automatically credit every original clause. |
| WS8br sidebar controller declarations | 8 | Real Huddle source/header/sidebar APIs are merged; the exact original live/quiet/Board/cache/query assertions still need fresh named receipts. |
| WS8br2 original interaction criteria | 14 | Missing complete member-card/huddle, star-menu/phone, brand-icon/two-theme, and icon-upload/delete browser receipts. Owner domain APIs alone do not prove these interactions. |
| WS8br muted-room browser | 1 | Original flowing-delivery control, muted noise and mention-unread sequence not executed here. Static row/push tests alone are insufficient. |
| WS17 meeting-status browser | 1 | Real refresh/interval/broadcast implementation passes; original toggle, stubbed busy interval, injected-clock advance and cleared browser badge need an exact replay. |
| Aggregate WS8br phone/header/member/pins mapping | Partial | Overlaps the broad originals above; interaction probes still do not establish every original declaration. |

Three original mobile-layout declarations are geometry/style-only and already
excluded by the no-pixel phase (P0340–P0342). They remain in historical totals;
no additional behaviour is excluded. The fourteen original WS8br2 criteria are
still emitted by `deferred_inventory.py`, which now clarifies that the old owner
names describe historical divisions rather than absent APIs. The remaining items
are **not all owner-blocked**: their primary blocker is incomplete named evidence.
This PR does not claim that unknown remaining behaviours are supported.

## Reproduction and discrimination

With the pinned Rails/current-schema image and the default, first_run and agents_ui
seeds (the only three seeds built by CI), run from the repository root:

```sh
PARITY_NAMESPACE=ws11ui-cutover PARITY_OWNER=ws11ui \
PARITY_IMAGE=ws11ui-cutover-reference:current-schema \
python3 rust/reference-tools/views/agents_ui/check_cutover_browser.py \
  --binary rust/target/debug/campfire --test-host "$CAMPFIRE_TEST_HOST"
python3 rust/reference-tools/check-ws17-huddle-cutover.py
PARITY_NAMESPACE=ws11ui-cutover PARITY_OWNER=ws11ui \
PARITY_IMAGE=ws11ui-cutover-reference:current-schema \
rust/parity/bin/reference runner --seed default \
  rust/reference-tools/ws17_huddle_cutover.rb > huddle-regenerated.json
cmp rust/vectors/ws17_huddle_cutover.json huddle-regenerated.json
```

`CAMPFIRE_TEST_HOST` is the current-source campfire test executable obtained from
Cargo's JSON build output, not an untracked fixture dependency. Build jobs remain
2 and nextest workers remain 4; the machine-wide rustc throttle is unchanged.
The media wrapper uses the pinned libvips/ffmpeg runtime with four libtest threads.

Raw executed summaries:

```text
Cutover inbox: Rails 1 passed; Rust 1 passed; 0 failures
Cutover inbox: intended writer defect rejected; Rails 1 passed; Rust 1 deliberate failure; waits unchanged
Cutover inbox-filter: Rails 1 passed; Rust 1 passed; 0 failures
Cutover inbox-filter: intended writer defect rejected; Rails 1 passed; Rust 1 deliberate failure; waits unchanged
Cutover work: Rails 1 passed; Rust 1 passed; 0 failures
Cutover work: intended writer defect rejected; Rails 1 passed; Rust 1 deliberate failure; waits unchanged
Cutover browser discrimination: 3 paired sequences passed; 3 writer defects rejected; 0 invalid controls
WS17 huddle cutover baseline: 2 passed; 0 failures
WS17 huddle cutover discrimination: 2 missing-source failures rejected; 0 invalid controls
Rails huddle cutover: 2 source scenarios; 0 failures
```

Vector regeneration and `cmp` exited 0; no expected response/payload normalization
is applied. Controls restore the production source in `finally`; the actual
HuddleGrant issuance producer is unchanged in the submitted patch.

## Fresh-clone verification

The independently cloned source was `e2fe3ccd4`, fast-forwarded through
`9f035162d` for the two media-runner corrections. Native Rust sources and vectors
are identical throughout those commits. Only the three CI seeds were independently
rebuilt. All 27 credited test identities (25 historical CI identities and the two
new huddle tests) passed in the completed fresh-clone workspace run.

Environment, from the fresh clone root (the existing worktree target is reused):

```sh
export CI=1 RUST_TEST_THREADS=4 CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11ui/rust/target
export TMPDIR="$PWD/.scratch/runtime"
export CABLE_TEST_PORT_RANGE=52750-52799 INTEGRATION_TEST_PORT_RANGE=52750-52799
export MAIL_TEST_PORT_RANGE=52750-52799 GITHUB_TEST_PORT_RANGE=52750-52799
export PARITY_OWNER=ws11ui PARITY_IMAGE=ws11ui-cutover-reference:current-schema
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="python3 $PWD/rust/reference-tools/agents/pinned-media-runner.py"
cargo nextest run --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever -j 4 --profile ci --no-fail-fast
cargo nextest run --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever -j 4 --profile ci -E 'test(attachment_processing_rows_html_and_broadcast_bytes_match_fresh_rails)'
cargo test --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --doc -- --test-threads=4
cargo clippy --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --all-targets -- -D warnings
bash rust/ci/with-release-inputs.sh cargo build --locked --workspace --bins
```

The first enumeration attempt stopped before running tests: the existing media
wrapper printed a diagnostic on stdout during `--list`. It now prints to stderr,
preserving nextest's unmodified libtest listing. The first complete run found one
codec mismatch in main's newer attachment-processing test at
`controllers/messages/attachment_processing_tests.rs:597`: the original MOV's
hash was identical, but the host produced different JPEG/WebP preview bytes. That
new test was missing from the pinned-media list. Adding it sends the unchanged test
through the same pinned codec runtime already used by the six older application
media-byte groups. The corrected test passes. No hash, byte count, expected HTML,
assertion or timing threshold is changed; this is a runtime-routing correction,
not a timing retry. The complete run's initial failure and the separate corrected
replay are both retained below rather than replacing the initial receipt.

Raw summary lines, in the command order above (the doctest lines represent the
two distinct summaries across eleven targets; aggregate 0 runnable examples,
0 failures, 2 existing ignored examples):

```text
     Summary [1746.931s] 4950 tests run: 4949 passed (7 slow), 1 failed, 20 skipped
     Summary [   1.657s] 1 test run: 1 passed, 4969 skipped
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 49s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 58s
```

Thus all **4,950 unique enabled workspace tests** have passing receipts across
the complete run and corrected pinned replay; the complete run itself records the
initial codec failure. The 20 skipped tests are the pre-existing ignored tools/
external cases. Filtered tests in the one-case replay are not new ignores. Strict
clippy and release-input build both exited 0. The normal release-input guard had
no access to vectors, parity files or reference tools.

Fresh-clone browser replay and vector regeneration also exited 0:

```text
Cutover inbox: Rails 1 passed; Rust 1 passed; 0 failures
Cutover inbox: intended writer defect rejected; Rails 1 passed; Rust 1 deliberate failure; waits unchanged
Cutover inbox-filter: Rails 1 passed; Rust 1 passed; 0 failures
Cutover inbox-filter: intended writer defect rejected; Rails 1 passed; Rust 1 deliberate failure; waits unchanged
Cutover work: Rails 1 passed; Rust 1 passed; 0 failures
Cutover work: intended writer defect rejected; Rails 1 passed; Rust 1 deliberate failure; waits unchanged
Cutover browser discrimination: 3 paired sequences passed; 3 writer defects rejected; 0 invalid controls
Rails huddle cutover: 2 source scenarios; 0 failures
```

The ledger checker consumes the current nextest listing and the completed native
log; it verifies every credited function is enabled and actually passed:

```text
Cutover current branch: 27 credited test identities passed in the fresh-clone workspace run
Cutover ledger receipts: 25 historical CI test identities still enabled; 14 WS17 closures; 3 external browser closures; 9 broad WS8 supersessions; 1 approved queue supersession; 0 inconsistent records
Cutover ledger remains partial: 328 broad receipts; 8 sidebar receipts; 14 overlapping criteria; 1 muted browser; 1 Calendar browser; 3 geometry-only exclusions
```

No production JS/CSS, vendored asset, digest golden or CodeQL configuration changed.
At that historical checkpoint, no new target directory was created. The fresh clone, downloaded nextest binary/
archive and copied Docker schema context are regenerable large scratch outputs and
are deleted after recording these results; the pre-existing worktree target is
retained. Small raw logs remain in the owned cutover scratch directory.

## PR #240 review corrections

Three newly enabled `work_mutations_test` regressions replace incomplete historical
Recorder credits in both WS17 and WS12:

- `work_room_notifications_off_then_mentions_restores_updates` executes tracking
  and assignment with notifications off, then proves a mentions recipient gets
  the newest unread status update.
- `work_status_update_retains_assignment_for_same_recipient` proves David retains
  exactly `[work_update, work_assignment]`; only the update source changes and the
  complete assignment row remains unchanged.
- `agent_work_opt_out_allows_agent_updates_and_human_reassignment` proves agent
  assignment suppression, the positive agent status-update exception, handling
  that update, and the positive human reassignment exception for the former owner.

These execute real work-event writers and their after-commit Recorder. Their new
assertions were absent from historical main CI 37200618245, which is now retained
only as supporting evidence for those three closures. Exact-head CI is pending.

The group huddle Rails producer now enables DND and quiet hours before creating
the room and issuing the grant, matching `push_gating_test.rb:165`. Regenerated
setup SQL gives Rust the same pre-issuance policies; the existing DND/starred
caller scenario still enables DND after issuance. A new policy-conditional
missing-invitation control must fail the group scenario while the later-DND
scenario still passes.

The existing `ws12_browser_c221/c222/c223_original_named_system_assertions`
ignored tests retain their WS12 assertions and additionally invoke, respectively,
the paired inbox, inbox-filter and work Rails/Rust sequence and writer control.
These exact names are already selected by the CI worker's ignored-test manifest;
no new ignored identity needs a manifest addition. Separate namespaces and
scenario-specific port triples support the CI job's four nextest workers. They run via
the existing `parity/system/ws12` ignored-browser entry point, which also builds
the current normal application binary; the test executable supplies the inbox
HTTP host. Direct nextest execution in the CI job builds the current normal
work host if the shell entry point has not supplied it. The CI job is
`Rust correctness (browsers)` from `rust/ci-full-gate`, owned by the other worker.
No workflow is edited here, and no combined-branch CI pass is claimed.

Local review-fix validation (Rust 1.98.1, pinned toolchain/media image, nextest
`-j 4`, no retries):

- **1,398 native tests passed, 0 failed**: all 1,384 enabled database tests and
  the 14 application tests credited by the reconciliation. This is a selected
  database/application run, not a new full-workspace execution. All 30 currently
  credited native identities have PASS lines.
- **3 enhanced ignored browser tests passed, 0 failed**, concurrently under four
  nextest workers. Their existing WS12 two-roster-size assertions remain intact;
  the three paired Rails/Rust sequences and three real writer defect controls
  pass inside the same already-registered tests. The work test exercises the
  direct-nextest normal-host build fallback.
- Huddle controls: **2 baseline passes**, **2 missing-issuance failures rejected**,
  **1 policy-conditional missing-invitation failure rejected**, **0 invalid controls**.
- Strict workspace/all-target Clippy with `-D warnings`: **passed**. Database
  doctests: **0 runnable, 0 failures**. The ledger checker passes with the current
  native listing/log and completed ignored-browser log.
- Private copies of existing `default`, `first_run`, and `agents_ui` seeds were
  independently checked in Rails: **29 + 4 + 40 = 73 passed, 0 failed**. No seed
  was rebuilt here. The corrected Rails huddle producer emits **2 scenarios,
  0 failures**; only group setup SQL differs from the previous vector.

No Rust behavior divergence was revealed. The submitted production Rust, Rails,
assets, workflow files and remaining manifest are unchanged. The corrected
closures stay closed with complete assertions, while exact-head/combined-branch
CI receipts remain explicitly pending.
