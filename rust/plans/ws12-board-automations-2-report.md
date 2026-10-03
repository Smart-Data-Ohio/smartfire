# WS12 board automations, second round

Branch: `rust/ws12-board-automations-2`. This checkpoint completes the automation production
surface; WS12 as a whole remains partial. The complete owned-file audit and remaining functional
boundaries are in `rust/plans/ws12-board-automations-2-unported.md`. The generic recorder API is
still WS12-owned continuation work, so this is not an owner-blocked-only stop.

## Re-review at `262fdfce`

This section supersedes the earlier review's response-difference count and verification
receipts. The three new findings are fixed within PR #206's scope:

| Finding | Regression and resulting behavior |
| --- | --- |
| P2 numeric-key SLA hashes | `review_pr206_round2_sla_shapes` replays 128 unique Rails shapes across all four statuses: 66 redirects, 35 validation responses, 27 exceptions. Filtering follows the pinned Strong Parameters numeric nested-attribute, array-of-hash and scalar permits. The lead's mixed-key repro clears and audits the rule. Every Rails-accepted response matches full bytes, headers and rule/tag/audit facts. Every rejected shape preserves the existing rule and writes no audit. |
| P2 digest preload failure | Two full-app socket regressions cover 34 due boards, including a late note in the second rendering batch. Normal venue destruction through a second database handle between event and venue loading still publishes all 34 frames, omitting the missing optional venue. A SQLite interruption during the late note's event preload publishes 33 healthy frames, leaves the failed board's claim unattached, and publishes nothing on repeat, matching Rails' explicit broadcast-failure differential. Published frame bytes match Rails exactly. Failed batch preloads fall back per note; successful rendering/broadcast IDs determine claim attachment. |
| P3 boolean tag casts | `review_pr206_round2_scalar_tags` compares 16 complete Rails responses and assignment/audit facts for booleans, null, signed/unsigned integers, floats, negative zero, exponent notation and a string control. Active Model String casting stores `t`/`f`; other scalars retain Ruby `to_s` semantics before tag normalization. |

The approved 400 difference now covers every SLA shape for which Rails raises 500,
including absent/scalar roots, retained root arrays and directly retained status arrays.
Rust returns its existing empty-body bad-request response, with no redirect, writes or
audit. This is explicit in the differential, without masking Rails' expected facts.
Accepted numeric-key collections and filtered empty arrays retain Rails' actual 302 or
422 behavior. The 128-shape corpus has exactly the reviewer's 66/35/27 distribution;
all 62 rejected Rails cases are checked to leave the original facts unchanged.

Failing-first receipts are in `rust/.scratch/pr206-round2/logs/`. Before their fixes,
`before-settings.log` fails both new settings regressions: shape-0 persists 61/241
instead of removing the rule, and the two boolean tags persist `true`/`false` instead
of `t`/`f`. `before-digests-valid.log` fails both corrected digest regressions: missing
venue publishes zero instead of 34 frames, and the failed note's claim is attached.
An earlier query-hook timeout was corrected and is not counted as regression proof.

The original six fixes were also temporarily removed and restored on this checkout.
Independent named settings regressions fail for malformed arrays, missing/underscored
assignees and Unicode blankness; the 48-case cast test reports its four differences;
the normal room-destruction race panics; the full-app performance regression fails.
Without batching, measured SLA reads are 323/3203 and digest reads are 283/2803 at
10/100 boards (the previously fixed reference callbacks remain installed). The receipts
are `without-original-settings.log`, `without-original-db.log`,
`without-original-performance.log` and `without-original-summary.log`. All mutations
were restored before the passing runs; no stash or other worktree was used.

New Rails producers are `review/round2-settings.rb` and `review/round2-digests.rb`.
They use the same pinned image, source checks and seeds as the first review. Producers
ran sequentially. The digest oracle was recaptured with identical JSON after fixing
client IDs at creation (not masking rendered bytes). The source and oracle replay
instructions are in `reference-tools/board_automations/review/README.md`.

| Full-app path | Rust 10 | Rails 10 | Rust 100 | Rails 100 |
| --- | ---: | ---: | ---: | ---: |
| SLA new | 163 | 268 | 1603 | 2668 |
| SLA repeat | 2 | 62 | 2 | 602 |
| Digest new | 64 | 61 | 487 | 601 |
| Digest repeat | 1 | 41 | 1 | 401 |
| Settings | 14 | 14 | 14 | 14 |

The probe includes the production rich-text callbacks, installed app broadcast adapter,
all readers and the writer. Successful batched broadcasts retain the individual renderer's
bytes. The 100-board and 64-quoted-source regressions retain SQLite's 64-variable limit.
Final gate receipts and cleanup are recorded in the companion verification file.

## PR #206 review corrections

This section supersedes the earlier database-only performance measurements below. The review
was reproduced from `ae99f082b` in the dedicated `ws12-fix206` worktree. The fix did not edit the
stacked branch or agent-controller/agent-work source. The reviewer scratch directory was read-only;
the producers and regressions use copies under this worktree.

After the fix commit, the required final merge check found a conflict with `origin/main` in
the calendar message-reference loader: both sides had independently replaced an IN-bind list
with one JSON bind. Per the task's conflict exception, `origin/main` at
`12b812796c53316480f9c861a71ae3144236b8d4` was merged; main's equivalent loader was retained.
Upstream agent-owned changes were imported by that merge without manual edits to those files.
All three verification gates were rerun on the merged tree. The GitHub discussion association
loader also uses one JSON bind, with batched/individual equality checked for 96 IDs at a
64-variable limit.

| Finding | Result and regression |
| --- | --- |
| P2 malformed SLA arrays delete rules | Fixed. Reject retained top-level hashes-in-arrays and nested status arrays before rule reads or mutations. Rails-accepted empty/scalar arrays still remove rules and audit exactly. The complete-response settings differential verifies unchanged rules and zero audits on malformed arrays. |
| P2 absent assignee becomes user 0 | Fixed. Carry an optional form foreign key through validation; missing/null/blank values cannot resolve an eligible ID-0 user. Explicit ID 0 remains valid. Strings use Rails integer casting, including underscored IDs. The settings differential checks 422 form bytes/no writes for absent values and successful underscored/explicit-zero assignments. |
| P2 destruction aborts sweeps | Fixed. Skip missing preloaded rooms; retain owned non-null status-entry timestamps and remove row-dependent indexing/expectations. Actual room destruction at three read boundaries leaves the healthy digest/SLA board running, with Rails destruction differentials. Concurrent two-handle sweeps still claim exactly once. The only remaining sweep `expect`s protect the in-memory deduplication mutex, not database rows. |
| P2 integrated reads exceed Rails growth | Fixed. Batch SLA recipient users and reuse the fresh transactional association check for claims/recording. Batch digest partial rendering without fragment-cache reads; reuse the persisted new-note body and reconcile reference removals with set-based deletes. Full-app growth assertions include every reader, writer, production callback and broadcast rendering. |
| P2 Unicode whitespace cannot clear timers | Fixed. Reuse the existing Active Support `String#blank?` Unicode-whitespace helper. NBSP, em-space, ideographic and mixed whitespace clear/audit/redirect exactly like Rails; half-blank invalid forms preserve complete Rails errors and facts. |
| P3 decimal-prefix casts | Fixed. Handle `0d`/`0D` after an optional sign. All 48 review cast cases now match casts, validity, complete validation errors and messages; decimal-prefix HTTP cases also match the invalid form value and response bytes. |

### Approved differences

The lead-approved malformed-array response difference is explicit in the HTTP regression:
Rails raises and returns 500; Rust returns its existing empty-body 400 bad-request response
(`text/html; charset=UTF-8`, no redirect). Both retain all existing rules and assignments and
write no automation audit. Four corpus responses exercise this difference. The Rust check
does not mask the Rails response or silently replace the persisted expected facts.
Accepted `sla_rules: []` and scalar arrays preserve Rails' 302 removal/audit behavior.
The existing source/inbox/job atomicity and approved Rails source-drift pins documented below
are unchanged.

### Failing-first evidence and Rails oracles

Before implementation, the added regressions produced these receipts in
`rust/.scratch/pr206-fixes/logs/before-review.log` and `before-settings-all.log`:

```text
APP_DISPATCH sla-mixed-10 run=0: fields_differ=[]; Rust_reads=322; Rails_reads=268; bound=32766
APP_DISPATCH sla-mixed-100 run=0: fields_differ=[]; Rust_reads=3202; Rails_reads=2668; bound=64
APP_DISPATCH digest-mixed-10 run=0: fields_differ=[]; Rust_reads=363; Rails_reads=61; bound=32766
APP_DISPATCH digest-mixed-100 run=0: fields_differ=[]; Rust_reads=3603; Rails_reads=601; bound=64
REVIEW decimal casts: 48 inputs; 4 differences
PR206 independent digest delete race: normal RoomDestroyJob completed; dispatcher_panicked=true; healthy_board_claims=0
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 2576 filtered out; finished in 4.77s
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 1295 filtered out; finished in 0.10s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2577 filtered out; finished in 17.70s
```

The aggregate settings failure independently reproduced deletion/audits for nested SLA arrays,
inverted empty-array handling, erroneous missing/null/blank ID-0 assignments, rejected
underscored IDs and rejected Unicode clears. The model failure reproduced exactly `0d10`,
`0d42`, `+0D42` and `-0d42`. These failures preceded the corresponding implementation changes.

Checked-in producers, inputs and replay instructions:
`reference-tools/board_automations/review/README.md`. The pinned image was
`ws12-reference:boards-b908ebc2`. Settings, dispatch and deletion producers check the existing
Rails source hashes; the cast producer checks dispatch/model hashes too. Rails was recaptured
for all original review vectors; JSON equality with the copied original outcomes was verified.
The additional quoted-title dispatch case was independently captured twice with identical
JSON, and Rails asserts that it creates 64 distinct quote references. Default, first_run and
agents_ui seeds were validated against the pinned Rails app (29/4/40 passed, zero failures).

### Full-app SELECT executions

The probe encloses the dispatch call through the production broadcast sink, with all database
readers and the writer traced. Assertions and diagnostic rendering comparisons run after the
probe stops. New/repeat cases use two due posts with both SLA stages, mixed owners and creators,
and real persisted job/reference callbacks. Settings is the authenticated full GET/render path
with 10/100 assignment choices. These are the reviewer's app-path measurements, not DB-only
counts. The 100-board sweeps cap every connection at 64 bound variables.

| Path | Rust 10 | Rails 10 | Rust 100 | Rails 100 |
| --- | ---: | ---: | ---: | ---: |
| SLA new | 163 | 268 | 1603 | 2668 |
| SLA repeat | 2 | 62 | 2 | 602 |
| Digest new | 64 | 61 | 487 | 601 |
| Digest repeat | 1 | 41 | 1 | 401 |
| Settings | 14 | 14 | 14 | 14 |

From 10 to 100 boards, SLA new grows by 1440 SELECTs versus Rails' 2400; digest new grows by
423 versus Rails' 540. Repeat sweeps and settings have zero growth. Digest's small fixed batch
overhead explains its three additional reads at 10 boards; its per-board growth meets the
requested Rails ceiling. Every digest's batched message partial matches the former shared
renderer byte-for-byte and in order, including 32 notes plus 64 quoted sources under the
64-variable limit. Association loaders bind one JSON list so referenced messages and users
cannot overflow the limit even when they exceed the root-note batch size.

The shared reference reconciliation changes preserve reference IDs/timestamps, creations,
validation, fetch jobs and callback ordering. They remove only existing-list/body SELECTs
charged to note creation; no callback domain is skipped. Ordinary message creation retains
its association validations. The broadcast still precedes claim attachment; post/attachment
failures retain the same committed claims/notes and allow healthy boards to finish.

### Review verification and cleanup

All three required gates passed with Rust 1.98.1, `CI=1`, `CARGO_BUILD_JOBS=4`, and the existing
`~/.cargo/config.toml` rustc throttle unchanged. The merged-tree rebuild used
`CARGO_INCREMENTAL=0` and line-table debug information for dev/test profiles. Full workspace tests include `html5ever` and
use eight test threads. Pinned libvips/ffmpeg from the reference image were supplied through
task-local `LD_LIBRARY_PATH`/`PATH`; `CI=1` requires media version equality. Tests ran in a
bubblewrap network namespace, isolating their localhost services from other workers and the
model server. Clippy and release-input checks used the same owned target directory.

The task-local Cargo helper sets `CARGO_TARGET_DIR=rust/.scratch/pr206-fixes/target` and separate
mail/cable/GitHub test port ranges, then executes `mise exec rust@1.98.1 -- cargo "$@"` from
`rust/`. The executed Cargo commands were:

```bash
cargo test --locked --workspace --no-fail-fast -- --test-threads=8
cargo test --locked -p campfire -p campfire_db review_pr206 -- --test-threads=4 --nocapture
cargo clippy --locked --workspace --all-targets -- -D warnings
ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo check --locked -p campfire
```

Raw principal summary lines, with the complete 61 workspace summaries and full-app read
receipts preserved in [review-verification.txt](ws12-board-automations-2-review-verification.txt):

```text
test result: ok. 2610 passed; 0 failed; 7 ignored; 0 measured; 0 filtered out; finished in 754.75s
test result: ok. 1306 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 125.93s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2615 filtered out; finished in 23.61s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1307 filtered out; finished in 0.28s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 11m 45s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6m 31s
```

The merged workspace aggregate is **4654 passed, 0 failed, 16 existing ignores**; this is an aggregate
of Cargo's raw summaries, not a Cargo-generated line. No new ignore was introduced. Full
workspace, focused review replay, strict clippy and release-input checks each exited zero.
Final logs use the `*-merged.log` names under `rust/.scratch/pr206-fixes/logs/`; the earlier
pre-merge receipts remain preserved as historical evidence. The four test-export JSON files
generated under `rust/target` were preserved in task-local `test-exports` directories before cleanup.

Owned target cleanup removed exactly `rust/.scratch/pr206-fixes/target` (5.1G) and `rust/target`
(40K), after checking that no process used either. Zero owned target directories remain;
no target in another worktree was touched.

## Completed

- The #200 ID-0 review correction excludes an existing claim ID only when one is supplied.
  A stored claim with ID 0 now returns Rails' validation error for a new duplicate; validating
  that stored row itself with `Some(0)` remains valid.
- BoardSlaRule model validations, uniqueness, dirty-column timer writes and stale-instance
  preservation; complete settings HTML and tag/SLA add/change/remove/no-op/error behaviour.
  Membership is checked before creator/admin authority, including for administrators.
  All statuses validate before any settings mutation, and omitted/blank statuses remove rules.
  Audits follow each successful primary write, as in Rails.
- SLA sweeps batch eligibility across boards, retain per-crossing/stage claims, resolve human
  and agent-owner recipients, dedupe pushes per recipient per sweep, isolate failures, and
  avoid writer transactions on repeat sweeps. Posting grants do not gate the SLA recipient.
- Daily stale digests: quiet root system notes, ruled unfinished posts, threshold boundaries,
  stalest-first order, first 20 entries plus remainder count, literal escaped titles/names,
  age formatting, per-board/day dedupe and repeat/next-day behaviour. Claims persist if note
  creation fails; the already-posted note and broadcast persist if associating the claim fails.
- Actual recurring task registration matches `app/services/periodic/runner.rb`: `board sla
  nudges` every 300 seconds and `board stale digests` every 3600 seconds. FrozenClock tests
  exercise the real closures and scheduler at 299/300 seconds, 3599/3600 seconds and UTC midnight.
- The real merged `BoardSlaNudge::claim_and_notify` persists the source, inbox item and typed
  `BoardNudgeJob { nudge_id }` in one transaction. Queue, inbox and source insert failures each
  roll back that source and allow subsequent threads to dispatch. Queue-inspection tests use
  `TestApp::without_job_runner()`.
- Dispatcher and settings associations are batched. A 64-variable-limit regression dispatches
  100 distinct boards and creators successfully; loaders use single-parameter `json_each`.
- Room destruction retains the shared WS8 API. Two actual Rails cleanup outcomes include
  rules, tag assignments, nudges, digests, messages, inbox sources, scheduled messages and polls.
- Additional complete-response/model probes caught inherited tag-rule ID-0 handling: creating a duplicate reports Rails' validation error, validating the existing ID-0 rule excludes itself, valid deletion removes it, and a malformed deletion ID retains it with Rails' redirect/alert and no audit.
- Additional differential probes caught arbitrary-size SLA integer and error-form differences.
  Decimal casts now preserve Ruby's bound/comparison errors, underscore handling, hexadecimal
  numericality distinctions and Unicode blank handling without machine-integer overflow.

The older digest description involving recipient preferences, local delivery hours, weekends,
timezones and quiet hours was incorrect and is superseded. Rails posts one quiet board note per
day. There is no `config/recurring.yml` in the pinned app; the recurring source is its runner.

## Changed files and shared surfaces

Paths below are relative to `rust/`.

| Files | Change |
| --- | --- |
| `crates/db/src/models/{board_sla_rule,board_stale_digest,board_automations}.rs`, `models.rs` | New validated rule/claim APIs and injected-time batched dispatchers. |
| `crates/db/src/models/{board_sla_nudge,board_tag_assignment}.rs` | Optional-ID uniqueness exclusions; ID-0 validation parity. |
| `crates/db/src/models/activity_item/recorder.rs` | Thin nudge entry point using main's reviewed recipient-aware recorder. |
| `crates/db/src/models/message.rs` | Narrow internal quiet-note helper reuses shared message callbacks with preloaded room/creator associations. Ordinary callers retain association validation. |
| `crates/campfire/src/controllers/rooms/board_automations.rs`, `board_automations/sla.rs`, `controllers.rs`, `rooms.rs` | Scoped settings routes, permission checks, tag writes, all-status validation and Rails form/error behaviour. |
| `crates/views/src/rooms/board_automations.rs`, `rooms.rs`, `templates/rooms/boards/automations.html` | Typed settings presentation and complete Rails-matching HTML. |
| `crates/campfire/src/jobs/periodic.rs`, `jobs/tests.rs`, `integrations/action_claims/tests.rs` | Real dispatcher task registration and both existing scheduler-roster expectations, preserving prior interval assertions. |
| `crates/db/src/tests/{board_automations_test,board_sla_nudge_test,board_tag_assignment_test}.rs`, `tests.rs`, `Cargo.toml`; `crates/campfire/src/controllers/rooms/board_automation_tests.rs` | Model/HTTP/clock/failure/cleanup/read-growth/parameter-limit regressions; SQL limits enabled for tests. |
| `reference-tools/board_automations/*`, `reference-tools/agents/tag_assignment_contract.rb`, `vectors/{board_automations,board_automation_settings,board_sla_nudge,board_tag_assignments_contract}.json` | Hash-pinned actual Rails producers and complete outcome/response vectors. |
| `reference-tools/users/ws12_inventory.py`, `plans/ws12-rails-cases.json`, `plans/ws12-board-automations-2-unported.md` | Explicit assertion mapping and the complete remaining owned-file audit. |

No duplicated WS11 owner services, WS17 push implementation, WS8 room-destruction path or #201 recorder batching was introduced.

## Source and integration

The reference remains d7c7de92 plus approved board drift. Board/work/activity source files that
drifted use origin/main's Rails versions. The canonical approved-drift image is
`ws12-reference:boards-b908ebc2`; the committed producers verify seven dispatch and nine settings
source hashes, all identical to the Rails files on the merged main. No Rails files were edited.
Production additions are inside crates; vectors and reference tools are test inputs.

The saved WIP was reviewed and renamed into a domain/contracts commit, then completed through
separate domain, controller/scheduler, inventory and numericality commits. The ID-0 commit
25e45a80 is retained. Main was merged with merge commits at 34b3cd40c (#192/#202), 15c9426be
(#199), b573dd24c (#203), and 692f998dd (#201/#204). The last merge retained main's complete ActivityItem model and recorder batching/grouping/callback implementation, adding only the BoardSlaNudge entry point that passes its already-loaded recipient into the reviewed recorder. Main's icon and message callback fixes were retained. The #201 and original #200 review branches were not changed.

Existing BoardSlaNudge reader shape, retained from merged #200:

```rust
BoardSlaNudge::find_by_id(&Connection, i64) -> Result<Option<BoardSlaNudge>>
BoardSlaNudge::find(&Connection, i64) -> Result<BoardSlaNudge>
BoardSlaNudge::for_ids(&Connection, &[i64]) -> Result<Vec<BoardSlaNudge>>
nudge.waited_minutes(Timestamp) -> i64
nudge.activity_recipient_ids() -> [i64; 1]
BoardSlaNudge::claim_and_notify(&mut Tx, NewBoardSlaNudge, bool) -> Result<BoardSlaNudge>
```

`for_ids` is bounded through `json_each`; its rows expose the room/thread/recipient/status/stage
and crossing timestamps. The merged inbox presenter uses this API rather than a BoardSlaNudge
SQL adapter. New async domain functions are `board_automations::dispatch_sla(&Database, Timestamp)` and
`board_automations::dispatch_digests(&Database, Timestamp)`, returning `Result<DispatchStats>`; BoardSlaRule exposes
create/update/destroy/find_by_id/for_room/validate and decimal `cast_threshold` facts. BoardStaleDigest
exposes claim/find_by_id/for_room/attach_message. Shared message creation, recorder and WS17 push paths
remain the actual production implementations.

## Failing-first evidence

Receipts are preserved under `.scratch/board-automations-2/logs/`. Actual defects and deliberately
injected negative controls are distinguished here; a negative control is not claimed as an
unmodified-main failure. Every temporary control was restored before final checks.

| Regression | Before/control receipt | What it demonstrates |
| --- | --- | --- |
| Stored claim ID 0 | `id-zero-before.log` | Original predicate returns no duplicate validation error. |
| Unimplemented settings | `settings-before-runtime.log` | Complete page response regression fails before HTTP handlers exist. |
| Digest read growth | `growth-before-current.log` | Saved WIP uses 91/901 SELECTs, exceeding Rails' 61/601 slope. |
| Digest association failure | `digest-attach-before.log` | Saved WIP loses the posted note; Rails retains it. |
| Model/SLA/digest eligibility | `domain-discrimination-red.log` | Restoring the ID-0 predicate and disabling validation/eligibility fails six tests. |
| Settings batching/scheduler | `settings-cadence-discrimination-red.log` | Per-assignee reloads and missing actual task registration fail two tests. |
| SQLite parameter bound | `limit-discrimination-red.log` | A deliberately unbounded loader fails at the lowered 64-variable limit. |
| Large SLA integer errors | `numeric-model-before.log`, `numeric-settings-before.log` | Tests-only commit 9c544a11d retains 2a43e84cb's production files; both model errors and four complete 422 responses fail. |
| Float/hexadecimal numericality | `numeric-model-extra-before.log` | The unchanged pre-fix validator reports the wrong error for a space-prefixed hexadecimal input. |
| Stored tag rule ID 0 | `tag-zero-model-before.log`, `tag-zero-settings-before.log` | Tests-only commit da3985ed0 fails with a database uniqueness error/500 response instead of Rails' validation/422 response; the corrected model and complete settings response now pass. |

## Read-growth receipts

All counts include SELECTs from pooled readers and writer transactions. Setup and post-request
assertion reads are outside the measured interval. Rails measurements are regenerated by the
committed producers, without masks.

| Probe (10/100 rows or boards) | Rust before/control | Rust after | Rails |
| --- | --- | --- | --- |
| SLA new claims | 82/802 (saved WIP) | 82/802 | 82/802 |
| SLA repeat | 2/2 (saved WIP) | 2/2 | 32/302 |
| Digest new claims | 91/901 (saved WIP) | 53/503 | 61/601 |
| Digest repeat | 1/1 (saved WIP) | 1/1 | 41/401 |
| Settings, distinct assignees | 25/115 (injected reload control) | 14/14 | 14/14 |

The settings 15/15 intermediate measurement became 14/14 after merging #199's page cache work.
The fixed-limit case uses 100 distinct boards and creators and emits 100 notes with every pooled
reader and the writer limited to 64 SQL variables. Its control fails with `too many SQL variables`.

## Remaining WS12-owned Rails behaviour

| Rails source | Remaining behaviour | Reason |
| --- | --- | --- |
| `app/services/activity_items/recorder.rb` | Shared generic record API for SavedItem, Event, HuddleGrant, AgentApproval, AgentBudgetNotice, ScheduledMessage, Session, TwoFactorCredential and arbitrary persisted polymorphic sources under `skip_source_check`. | WS12 continuation: add typed source/authorization and recipient/idempotency contracts. Existing owning-domain writers already record their inbox rows; preserve their callbacks. |
| `app/helpers/activity_items_helper.rb`, `app/controllers/activity_items_controller.rb`, corresponding inbox views | Replace three AgentBudgetNotice read-only presenter seams (HTML preload/card and JSON facts). | WS11 must export a typed budget-notice reader, then WS12/WS11-UI integrate it. |

The complete file-by-file audit is in `ws12-board-automations-2-unported.md`. The 488-declaration
ledger remains partial: 249 mapped ports, eight existing peer tests and 231 exact named assertion
sets awaiting reconciliation. Each deferred name and owner remains in `ws12-rails-cases.json`.
This is validation debt, not a claim that the corresponding already-merged production path is
absent. No unmapped name is silently credited by this checkpoint. #201 is now merged; its reviewed
JSON batching, parameter-limit and brand fixes are retained without changing its review branch.

## Final verification

The clean source clone is `.scratch/board-automations-2-clean/source`, created locally with
`git clone --local --no-hardlinks --single-branch --branch rust/ws12-board-automations-2 . .scratch/board-automations-2-clean/source`.
After each merged source commit it was advanced using its local origin, with no tracked changes;
the final tested source is recorded in `logs/final-source-sha.txt`. Cached compilation output is
separate from this clean clone, in the owned `.scratch/target`. All three parity seed databases
were copied into the clone and independently revalidated: default 29/0, first_run 4/0, agents_ui
40/0. Tests were run, rather than silently skipping for absent seeds.

Toolchain: the installed direct `cargo`/`rustc` 1.98.0. The configured global rustc throttle was
retained. Focused tests use two test threads; the final full workspace uses eight.

Final gate commands, from the worktree root, share:

```sh
export CI=1 CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR="$PWD/.scratch/target"
export TMPDIR="$PWD/.scratch/board-automations-2-clean/source/tmp"
export LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs"
export PATH="$PWD/.scratch/rails-media/usr/bin:$PATH"
export CABLE_TEST_PORT_RANGE=53420-53449
export MAIL_TEST_PORT_RANGE=53400-53419
export GITHUB_TEST_PORT_RANGE=53450-53499
cargo metadata --manifest-path .scratch/board-automations-2-clean/source/rust/Cargo.toml --locked --format-version 1
cargo test --manifest-path .scratch/board-automations-2-clean/source/rust/Cargo.toml --locked -p campfire_db board_ -- --test-threads=2 --nocapture
cargo test --manifest-path .scratch/board-automations-2-clean/source/rust/Cargo.toml --locked -p campfire board_automation -- --test-threads=2 --nocapture
cargo test --manifest-path .scratch/board-automations-2-clean/source/rust/Cargo.toml --locked --workspace --no-fail-fast -- --test-threads=8
cargo clippy --manifest-path .scratch/board-automations-2-clean/source/rust/Cargo.toml --locked --workspace --all-targets -- -D warnings
.scratch/board-automations-2-clean/source/rust/ci/with-release-inputs.sh cargo check --locked -p campfire
```

Outputs are redirected to the named receipts under `.scratch/board-automations-2/logs/`; the
sequence is saved in `.scratch/board-automations-2/final-gates.sh`. The metadata command returned
0 and produced valid locked workspace metadata. The final compiler/test summary lines follow
below.

The initial full run on the pre-tag-fix snapshot found one owned scheduler-roster expectation
failure: the GitHub recovery test still expected the old task list. The expectation now includes
both board tasks, retaining that test's original 29/30-second boundary assertions. That failed
receipt is `workspace-before-tag-fix.log`; it is not claimed as a passing final run. Earlier
interrupted runs are also retained separately. No test was disabled to obtain the final result.

Rails oracle regeneration commands used this fixed prefix:

```sh
export PARITY_RUNTIME=docker
export PARITY_IMAGE=ws12-reference:boards-b908ebc2
export PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_CPUS=1
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/board_automations/dispatch.rb
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/board_automations/settings.rb
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/board_automations/nudge.rb
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/agents/tag_assignment_contract.rb
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/work/human_http.rb
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/work/link_model.rb
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/boards/write_views.rb
python3 .scratch/board-automations-2/verify_vectors.py
python3 rust/reference-tools/users/ws12_inventory.py
```

For each of default, first_run and agents_ui, the seed check was:

```sh
rust/parity/bin/reference runner --seed NAME --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb NAME
```

The raw producer lines, equality/hash checks, focused counts, complete workspace summaries,
strict clippy and release-input results are appended below. The full 488-entry assertion ledger
remains partial as described above; final workspace ignores are explicitly listed with their
existing reasons. Two are documentation examples; the others require external harnesses or are
peer-owned opt-in checks/measurements. The two Drive-polling ignores still carry a pre-merge
WS11-API reason; they remain peer validation debt for WS14g/WS11-API to reconcile, not an
automation feature claimed absent or a new ignore introduced here.


Tested source SHA: `20ec36ae3258193febc6ba40f549290d25c9edca`.

### Regenerated Rails and seed receipts

```text
Rails board automation oracle: 40 rule cases; 22 SLA cases; 19 digest cases; 2 failure cases; 2 cleanup cases; 2 cadences; 8 query probes; 0 masks
Rails board automation settings oracle: 41 complete responses and rule/tag/audit facts; 0 masks
Rails BoardSlaNudge oracle: 25 model cases; 6 recorder cases; 5 complete JSON responses plus HTML list fragments; 0 masks
Rails board tag assignment oracle: 19 validation cases
Rails human work HTTP oracle: 90 complete responses; committed handoffs; 0 masks
Rails work link model oracle: 29 validation/persistence cases; 0 masks
Rails board write oracle: 96 complete HTTP responses; no masks
seed-default.log
  "passed": 29,
  "failed": 0
seed-first_run.log
  "passed": 4,
  "failed": 0
seed-agents_ui.log
  "passed": 40,
  "failed": 0
Rails regeneration matches committed vector: board_automations.json
Rails regeneration matches committed vector: board_automation_settings.json
Rails regeneration matches committed vector: board_sla_nudge.json
Rails regeneration matches committed vector: board_tag_assignments_contract.json
Rails regeneration matches committed vector: human_work_http.json
Rails regeneration matches committed vector: work_link_model.json
Rails regeneration matches committed vector: boards_write.json
Rails source hashes match: dispatch-source-hashes.json: 7 files
Rails source hashes match: settings-source-hashes.json: 9 files
Rails source hashes match: source-hashes.json: 4 files
WS12 Rails inventory: 488 declarations; 231 deferred; 8 existing peer tests; 249 ported
```

### Failing-first raw summaries

```text
id-zero-before.log
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1281 filtered out; finished in 1.53s
settings-before-runtime.log
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2552 filtered out; finished in 0.75s
growth-before-current.log
test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 1286 filtered out; finished in 2.70s
digest-attach-before.log
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1293 filtered out; finished in 0.14s
domain-discrimination-red.log
test result: FAILED. 40 passed; 6 failed; 0 ignored; 0 measured; 1245 filtered out; finished in 4.91s
settings-cadence-discrimination-red.log
test result: FAILED. 2 passed; 2 failed; 0 ignored; 0 measured; 2552 filtered out; finished in 15.80s
limit-discrimination-red.log
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1292 filtered out; finished in 0.10s
numeric-model-before.log
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1294 filtered out; finished in 4.45s
numeric-settings-before.log
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2564 filtered out; finished in 17.40s
numeric-model-extra-before.log
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1294 filtered out; finished in 2.24s
tag-zero-model-before.log
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1294 filtered out; finished in 1.72s
tag-zero-settings-before.log
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2564 filtered out; finished in 16.80s
```

### Final focused domain and HTTP checks

```text
board-domain-fresh.log
WS12 automation sla boards=10: 82 SELECTs
WS12 automation sla repeat boards=10: 2 SELECTs
WS12 automation sla boards=100: 802 SELECTs
WS12 automation sla repeat boards=100: 2 SELECTs
WS12 automation digest boards=10: 53 SELECTs
WS12 automation digest repeat boards=10: 1 SELECTs
WS12 automation digest boards=100: 503 SELECTs
WS12 automation digest repeat boards=100: 1 SELECTs
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 1246 filtered out; finished in 21.92s
app-automation-fresh.log
WS12 settings choices-10: Rust 14 SELECTs; Rails 14
WS12 settings choices-100: Rust 14 SELECTs; Rails 14
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 2572 filtered out; finished in 24.23s
```

### Final full workspace summaries

```text
     Running unittests src/main.rs (.scratch/target/debug/deps/campfire-a807c80fc8c66d04)
test result: ok. 2569 passed; 0 failed; 7 ignored; 0 measured; 0 filtered out; finished in 804.00s
     Running unittests src/lib.rs (.scratch/target/debug/deps/campfire_assets-cdbdd67d88332668)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/reference.rs (.scratch/target/debug/deps/reference-2347bcece6177bf3)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.63s
     Running unittests src/lib.rs (.scratch/target/debug/deps/campfire_cable-4d6b475422b0806a)
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/disconnect.rs (.scratch/target/debug/deps/disconnect-61ac8c5472a2da55)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.12s
     Running tests/golden.rs (.scratch/target/debug/deps/golden-cf971b0fd48e37af)
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
     Running tests/protocol.rs (.scratch/target/debug/deps/protocol-378335b477599cc9)
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
     Running unittests src/lib.rs (.scratch/target/debug/deps/campfire_db-f3bb08b4b99eab21)
test result: ok. 1291 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 128.00s
     Running unittests src/lib.rs (.scratch/target/debug/deps/campfire_jobs-77fc70e4df1e0ec2)
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.87s
     Running tests/crash.rs (.scratch/target/debug/deps/crash-ffd2cc9d96703e90)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
     Running unittests src/lib.rs (.scratch/target/debug/deps/campfire_kit-debd80d309a2eefa)
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
     Running tests/front.rs (.scratch/target/debug/deps/front-a46d3267297d2770)
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.03s
     Running tests/http.rs (.scratch/target/debug/deps/http-806f4dc76478730b)
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/params_vectors.rs (.scratch/target/debug/deps/params_vectors-3af71860aaa19374)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
     Running tests/rails_vectors.rs (.scratch/target/debug/deps/rails_vectors-a024baaa2e444cd9)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running unittests src/lib.rs (.scratch/target/debug/deps/campfire_mail-3e0544b69edcd6f2)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
     Running tests/config.rs (.scratch/target/debug/deps/config-fe8eab856e8ded36)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/goldens.rs (.scratch/target/debug/deps/goldens-dae9d22bdf0cb897)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
     Running tests/inbound.rs (.scratch/target/debug/deps/inbound-4c60716535b7b9bb)
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.96s
     Running tests/security.rs (.scratch/target/debug/deps/security-84e62572e2b2dedc)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/smtp.rs (.scratch/target/debug/deps/smtp-217e01f359f1178f)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.30s
     Running unittests src/lib.rs (.scratch/target/debug/deps/campfire_richtext-92c33c467c74c92e)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.37s
     Running tests/corpus.rs (.scratch/target/debug/deps/corpus-3c9e3b2d72d1e357)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 22.36s
     Running tests/fork_regressions.rs (.scratch/target/debug/deps/fork_regressions-bd18eaad4d862c1d)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/hardening.rs (.scratch/target/debug/deps/hardening-5e74d76ad77dec36)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.11s
     Running tests/markdown_corpus.rs (.scratch/target/debug/deps/markdown_corpus-6def8a327c913bbb)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.34s
     Running tests/markdown_security.rs (.scratch/target/debug/deps/markdown_security-37f311817860c55c)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
     Running tests/reference_tests.rs (.scratch/target/debug/deps/reference_tests-c9d5ebb0c6d59d57)
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/sgid_json_corpus.rs (.scratch/target/debug/deps/sgid_json_corpus-4fcd070c69f3303d)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.09s
     Running unittests src/lib.rs (.scratch/target/debug/deps/campfire_routes-c8ce2fd0f8242ea5)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running unittests src/lib.rs (.scratch/target/debug/deps/campfire_storage-f9b0ec84b311aef4)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
     Running tests/vectors.rs (.scratch/target/debug/deps/vectors-cda60e4b99b84c81)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.54s
     Running unittests src/lib.rs (.scratch/target/debug/deps/campfire_views-f50b3f11c6fb79e2)
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
     Running tests/core.rs (.scratch/target/debug/deps/core-16db2090785ad396)
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.93s
     Running tests/direct_forms.rs (.scratch/target/debug/deps/direct_forms-5b6675951fad5e28)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/event_pages.rs (.scratch/target/debug/deps/event_pages-e9bd0b0d30895cbf)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
     Running tests/events.rs (.scratch/target/debug/deps/events-88e21f4ec791ddc6)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/google.rs (.scratch/target/debug/deps/google-c463882574f96e8c)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/room_access_forms.rs (.scratch/target/debug/deps/room_access_forms-83e376f2c9b21e70)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/room_forms.rs (.scratch/target/debug/deps/room_forms-bc5fa6812f2339a2)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/room_header.rs (.scratch/target/debug/deps/room_header-1d7651adad6e3ba9)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/room_inbound.rs (.scratch/target/debug/deps/room_inbound-167d6ca1cc7fa8da)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/room_join.rs (.scratch/target/debug/deps/room_join-520cc25a2f7339ed)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/room_shell.rs (.scratch/target/debug/deps/room_shell-3e25c9035dee0c9e)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/sidebar.rs (.scratch/target/debug/deps/sidebar-fab9eadc7155226c)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/ws17_dm_profile.rs (.scratch/target/debug/deps/ws17_dm_profile-efd3aea32ab42d42)
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
     Running tests/ws17_settings.rs (.scratch/target/debug/deps/ws17_settings-2b16c440c8fdca14)
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
     Running unittests src/lib.rs (.scratch/target/debug/deps/html5ever-95781707b1c822c7)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (.scratch/target/debug/deps/rails_compat-70f1aa0587efabd6)
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.18s
   Doc-tests campfire_assets
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_cable
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_db
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_jobs
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_kit
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_mail
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_richtext
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_routes
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_storage
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_views
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests html5ever
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests rails_compat
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Aggregating those raw summaries (not a Cargo-generated line):

```text
Workspace summary totals: 4597 passed; 0 failed; 16 ignored
```

Existing ignored cases from the full workspace output; no new ignore was introduced:

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test controllers::internal_huddle_tests::huddle_gateway_own_node_suite_against_rust_endpoints ... ignored, requires Node and the gateway pinned ws package; run explicitly with --ignored
test controllers::rooms::system_browser_tests::huddle_system_cases_in_real_browser ... ignored, requires Docker and the pinned Playwright image; run parity/system/ws13
test controllers::rooms::system_browser_tests::livekit_stage_system_cases_in_real_browser ... ignored, requires the project-local LiveKit server; run parity/system/ws13-livekit
test integrations::agent_jobs::drive_attachment_cases::ws14g_agent_polling_http_carries_an_empty_drive_array ... ignored, pending WS11-API PR: /agents/events shape at 60d97bd is not on main
test integrations::agent_jobs::drive_attachment_cases::ws14g_agent_polling_http_carries_drive_file_ids_and_urls_only ... ignored, pending WS11-API PR: /agents/events shape at 60d97bd is not on main
test jobs::tests::push_latency ... ignored, a measurement, not a test
test record_reference ... ignored, needs a running reference app; see the module docs
test tests::differential_test::scenario_matches_ruby ... ignored, needs CAMPFIRE_RUBY_SCENARIO_DB, the reference app's database after scenario.rb
test tests::fixtures_test::export_database_for_rails ... ignored, writes a database to CAMPFIRE_EXPORT_DB for the Rails rollback check
test tests::fixtures_test::fixtures_match_ruby_row_for_row ... ignored, needs CAMPFIRE_RUBY_FIXTURES_DB, a database the reference app filled with `db:fixtures:load` at CAMPFIRE_FIXTURES_NOW
test tests::two_factor_rollback_test::read_rails_rollback_changes ... ignored, requires Rails to mutate the rollback fixture; run reference-tools/auth/rollback.sh
test acme_tls_alpn_certificate_cached_and_reused ... ignored, requires a local Pebble ACME CA, PEBBLE_MINICA root certificate and TLS ports 5001/5002
test export_for_rails ... ignored, exports a database for the Rails rollback check; requires CAMPFIRE_MAIL_EXPORT_DIR
test crates/kit/src/error.rs - error::halt (line 91) ... ignored
test crates/kit/src/lib.rs - (line 7) ... ignored
```

### Final compiler and release-input summaries

```text
clippy-fresh.log
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 03s
release-inputs.log
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 08s
Fresh-clone workspace: exit 0
Fresh-clone strict clippy: exit 0
Fresh-clone release-inputs check: exit 0
```

### Scratch cleanup

Regenerable Cargo output was removed after all checks. Logs, oracle outputs, seeds, media tools
and the clean source clone are preserved. No other worktree or target was changed.

```text
Owned scratch target cleanup: 2 targets; no active executable from either target
Before: .scratch/target: 36G; clean-clone rust/target: 44K
Deleted: .scratch/target
Deleted: .scratch/board-automations-2-clean/source/rust/target
After: 0 owned Cargo targets remain
```
