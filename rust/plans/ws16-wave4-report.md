# WS16 Wave 4 report — partial, run controllers and retained-undo completion

Status: **partial; WS16-owned work remains. This is not the only-owner-blocked handoff.** No product decision or approval is blocking this slice. The exact remaining declarations and response-comparison boundary are below; none is claimed completed because another owner merged. The lead should not infer PR readiness from the completed controller slice.

Assigned worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws16`, branch `rust/ws16-slack-import`. Prior foundations and OAuth/setup/manifest are retained and exercised in the final workspace run. This report supersedes the previous OAuth-only report.

Rails reference: `d7c7de9264c63015be398001d7a1094e7695a6db`, with the shared approved layout/assets drift from `_common.md`. The source checker verifies all 21 Slack reference files against the actual image and confirms no Slack drift on `origin/main`. Main through `b908ebc2` is merged, including #177 users/accounts (`72fc8b05`) and #172 huddles. Merge commits are `e12fef93` and `40e06f02`; no rebase or stash was used. Locked/offline metadata validates all 13 workspace members.

Pushed slices:

- `3f1ab564`: all fifteen run/import actions, body goldens, HTTP session/CSRF comparisons, Ruby option/date normalization.
- `47304761`: eleven retained undo/reimport scenarios, shared huddle destruction callbacks and atomic rollback, bulk membership fixture clock.
- `942e724f`: Ruby client coercions, local TLS recovery/history ordering, catch-up/finishing regressions, undo-view SQL count, security mutation guard and declaration inventory.
- `4f94fd73`: reproducible Rails-only sequence encryption entropy and all thirteen regenerated sequence vectors. Initial full-suite source was this commit. `c04ab8bc` then removes a redundant `Ok(...?)` in plan sample rendering found by clippy; final full-suite/clippy/release gates use that source. The final report-only SHA is given in the worker reply.

## Changed files and behavior

| Files | Result |
|---|---|
| `crates/campfire/src/controllers.rs`, `controllers/slack/{runs,run_tests}.rs` | All nine admin actions (`index`, `create`, `show`, `status`, `plan`, `start_import`, `catch_up`, `cancel`, `undo`) and all six personal actions (`index`, `create`, `show`, `status`, `cancel`, `undo`) are routed. Rails role/ownership/order, connected/workspace/active checks, per-user single-flight with global queuing, mode branches, selection/target/date filtering, exact notices/alerts/audits and durable queue effects are compared. Both undo controls use one later-stats scan, measured by SQLite trace. |
| `crates/views/src/slack.rs`, templates under `accounts/slack_import_runs/`, `slack/imports/` and `slack/import_runs/` | Index, plan, personal opt-in state, progress, issues, cancellation, undo, later-import naming and queued-behind rendering. Eighty-three complete owned Rails template bodies compare byte for byte. This covers all status/kind/mode branches, escaped malicious samples/names, room selectors, empty/nonempty lists, connection states and 55 issues over three pages. |
| `crates/campfire/src/integrations/slack/options.rs`, `crates/db/src/models/slack_import.rs` | Thirty-three actual Ruby coercion/time vectors, including `Array`, uniqueness/presence/default private behavior, room-target forms, ISO case/offset/fraction/day rollover/hour-24/second-60 handling and exact invalid-time errors. Controller enqueue timestamps follow the requester's time zone; lease stamps remain UTC. |
| `integrations/slack/client.rs`, `client/tests.rs` | Ruby `error.to_s`, `Array(needed).join`, nested/hash/bool/number scopes and Ruby blankness. Thirty-two actual Rails success/error mappings; local TLS 503→502→200, interrupted-dial recovery and per-attempt callback accounting, explicit recorded history ordering. The generic exhausted network error still lacks Rails' class/detail text; it remains partial. |
| `integrations/slack/store/tests.rs` | Native runner/store over local TLS: three-year/three-page history with zero step budget; middle-year bounded then full import; subsequent exact 30-day catch-up; coverage invalidation after native undo; fresh roots/late replies and deleted mapped thread; finishing earlier-created rooms, preserved newer read pointers and unread members; heartbeat/lease refresh and an actual identity-index range seek. |
| `integrations/slack/sequence_tests.rs`, `reference-tools/slack/sequence_vectors.rb`, `vectors/slack/sequence*.json` | The existing workspace/personal differentials plus eleven retained variants. Common SQL input mutations are applied identically and checked before undo. Saved reply, poll reply, pending quoted reply, sent schedule, foreign thread, room event/schedule, session, Google account, Google identity, password and placeholder authorship then compare every field in 89 tables after import, undo and reimport. Google-account removal versus identity/session/password retention follows the actual Rails result. |
| `crates/db/src/models/user.rs` | Automatic open-room bulk membership insertions use the existing `Env::sqlite_now_sql()` provider, already used elsewhere. Production still uses the same SQLite expression; fixture runs now use the same frozen input as Rails. No timestamps are discarded from comparisons. |
| `crates/db/src/models/user/destruction.rs`, `crates/db/src/tests/slack_test.rs`, `reference-tools/slack/undo_huddle.rb`, `vectors/slack/undo_huddle.json` | Slack undo reuses main's typed `HuddleGrant::revoke_for_user` before placeholder destruction. All rows/fields in six affected Rails tables match, with stream and presence events asserted. A rejected cleanup insert proves rollback of grants, streams, memberships, user and events. Shared room/session destruction continues to use main's callbacks. |
| `crates/campfire/Cargo.toml` | SQLite tracing is enabled only as a dev dependency for the query-count regression. |
| `reference-tools/slack/{run_views,runs_http,options_vectors,client_vectors,check_run_mutations,write_test_inventory}.*`, matching vectors and `plans/ws16-test-inventory.md` | Actual Rails oracle producers, failing-first security/body mutations, and a declaration-by-declaration coverage/remaining ledger. |

## Comparison boundaries and design

The real HTTP oracle enables Rails forgery protection and supplies signed sessions and actual CSRF values. The 109 cases compare status, redirect location, flash, every persisted run field, audit rows and durable job requests. Rust reaches the real Axum HTTP stack with the same session/CSRF inputs; fake Slack receives no requests for those run-control actions. The combined interaction executes setup credentials → real sudo password/session → preview → plan → bounded import → status poll → undo, with actual HTTP controls. Separate real HTTP checks verify newest-first/all-owner ordering, pagination of 55 issues and the personal undo page.

The exact HTML claim is **83 complete owned template bodies**, in addition to the previously ported 11 setup bodies. CSRF inputs are shared before rendering; no HTML normalization or output masking was added. **Complete application-wrapped HTTP response bytes have not yet been compared**, and the complete personal opt-in → account claim → import → undo browser interaction remains a WS16 integration check. Personal opt-in/connection states, create modes and owner guards already have body/HTTP coverage; this does not replace that combined interaction.

The thirteen sequence differentials compare all application tables, including empty tables, actual search rows and `sqlite_sequence`. JSON columns compare as JSON values and row sorting is presentation only. Schema bookkeeping and FTS shadow tables remain the original exclusions; no exclusion or allowlist was added or loosened. Every opaque encrypted column remains compared. The Rails producer fixes clock, UUID and initial fixture encryption IV entropy; real Rails key derivation/encryption/authentication/serialization remain active, and Rust loads the resulting initial ciphertext unchanged. Repeated fresh-clone regeneration reproduced every vector byte exactly. Production randomness is unchanged.

Huddle cleanup stays inside the shared callback/transaction seam, with remote cleanup intent persisted for after-commit delivery; there is no network call inside the SQLite writer. The Rails oracle unsets LiveKit inputs; the pinned Rust tests receive none. Tests never dial Slack or LiveKit. Headers are constructed at test time from fake inputs. WS3's durable single-worker queue, 25-second step budget, Tier 2/3 pacing, Retry-After continuation and 30-second sweep remain in place.

## Exact remaining scope — WS16 continuation

The inventory has **214 covered, 4 partial, 26 deferred; 244 Rails declarations**. Covered maps executable behavior and does not claim the original Ruby test class ran against Rust. No remaining declaration is reassigned to another workstream. In addition to the complete HTTP-wrapper/personal account-claim interaction boundary above, every partial/deferred declaration is listed here:

- `test/jobs/slack_import/run_lifecycle_test.rb`: mentions of mapped users outside the channel render as tokens, unknown ids fall back (deferred).
- `test/jobs/slack_import/run_lifecycle_test.rb`: a conversation whose mapped room was deleted is skipped with an issue (deferred).
- `test/jobs/slack_import/run_lifecycle_test.rb`: workspace runs never auto-merge a large group DM by name (deferred).
- `test/jobs/slack_import/run_lifecycle_test.rb`: workspace runs merge a large group DM only into its room target (deferred).
- `test/jobs/slack_import/run_lifecycle_test.rb`: personal runs never auto-merge a large group DM by name (deferred).
- `test/jobs/slack_import/run_lifecycle_test.rb`: dry runs preview a targeted large group DM as a merge (deferred).
- `test/jobs/slack_import/run_lifecycle_test.rb`: personal runs ignore room target ids (deferred).
- `test/jobs/slack_import/run_lifecycle_test.rb`: room setup is atomic: a crash while recording memberships leaves nothing behind (deferred).
- `test/jobs/slack_import/run_lifecycle_test.rb`: date bounds keep every row in range and are sent to Slack (partial).
- `test/jobs/slack_import/run_lifecycle_test.rb`: scope errors fail with the missing scope and leave the connection (partial).
- `test/jobs/slack_import/run_lifecycle_test.rb`: transient failures past the retry budget fail the run (partial).
- `test/jobs/slack_import/run_lifecycle_test.rb`: failed runs stay resumable: a new run continues from the mapping (deferred).
- `test/jobs/slack_import/run_lifecycle_test.rb`: workspace runs exclude private channels when asked (deferred).
- `test/jobs/slack_import/workspace_import_test.rb`: import skips a thread whose parent message was deleted mid-run (deferred).
- `test/jobs/slack_import/workspace_import_test.rb`: completing a run kicks the next queued run (deferred).
- `test/jobs/slack_import/workspace_import_test.rb`: undoing a run kicks the next queued run (deferred).
- `test/jobs/slack_import/workspace_import_test.rb`: truncated reaction lists import the listed users with one issue per message (deferred).
- `test/models/slack/client_test.rb`: network errors retry then raise (partial).
- `test/models/slack/markdown_converter_test.rb`: rendering: mentions resolve to attachments through a real save (deferred).
- `test/models/slack/markdown_converter_test.rb`: rendering: broadcast mentions create no attachments or notifications (deferred).
- `test/models/slack/markdown_converter_test.rb`: rendering: fenced first-line code and bare urls render as intended (deferred).
- `test/models/slack/markdown_converter_test.rb`: rendering: emphasis, links, bullets and quotes render as markdown (deferred).
- `test/models/slack_import_test.rb`: a runner step refreshes its lease wherever it refreshes the heartbeat (deferred).
- `test/models/slack_import_test.rb`: a runner without a lease token leaves the lease alone (deferred).
- `test/models/slack_import_test.rb`: an undoer step refreshes its lease when it saves undo state (deferred).
- `test/models/slack_import_test.rb`: another job's lease write is not mistaken for progress on conflict (deferred).
- `test/models/slack_import_test.rb`: the undo claim itself refuses a queued run that slips in after the pre-check (deferred).
- `test/models/slack_import_test.rb`: failed and cancelled runs kick the next queued run (deferred).
- `test/models/slack_import_test.rb`: step_finishing does not overwrite a cancelled run (deferred).
- `test/models/slack_import_test.rb`: step_finishing completes a running run and kicks the next queued one (deferred).

Remaining malformed root/scalar payload and mapper/converter normalization cases must also be exercised against Rails. The transport class/message gap is a known implementation limit, not an owner block. Already covered Rational/float boundary and scalar error/scope vectors do not establish arbitrary payload parity.

Per-file declaration counts: admin runs 33 covered; setup 10; disconnect 6; personal imports 20; OAuth 22; run lifecycle 16 covered/3 partial/10 deferred; workspace import 30/0/4; client 17/1/0; converter 26/0/4; SlackImport model 33/0/8; system interaction 1/0/0.

## Failing-first evidence

`check_run_mutations.py` ran in the pinned image on the fresh clone at `942e724f`; the authorization and template guards are unchanged in the final source. It simultaneously disabled the admin role gate, admitted another user's personal run and changed plan bytes. The three independent assertions failed, including both real HTTP cases; its `finally` restored all source files. The final suite uses restored sources. Raw output is included below.

The new huddle tests initially failed both the Rails-row callback comparison and cleanup rollback before the raw grant update was replaced by the shared huddle seam. The added Ruby client vectors initially failed on numeric `error` before its Ruby coercion fix. Those historical failures informed the fixes; they are not presented as runnable commands on the fixed source. The final workspace run rechecks both paths.

## Cross-workstream touches and open questions

WS1 verifier/encryption, WS3 queue, WS4 sessions/CSRF, WS5 rich text and the merged view/page APIs are reused. Main's users/accounts removal and huddle callbacks are consumed rather than copied. The only shared User change outside the Slack adapter is the existing fixture SQL-clock provider in open-room membership insertion. The huddle adapter now uses the merged huddle domain seam and does not leave a WS13-owned callback stub. No new broadcast/notifications or per-message saves were introduced. There is no owner-blocked item to hand off in place of the unfinished WS16 checks above.

## Verification environment and commands

An independent fresh GitHub clone at `.scratch/ws16-runs-final2` has the GitHub HTTPS remote and no alternates. It was fast-forwarded to final executable source `c04ab8bc`. Its pinned Rails archive and fresh `default`/`first_run` seeds were built there, then Rails validated 29 default and four first-run assertions. `CI=1` makes missing seeds fail. Source and regenerated vectors were clean before the final run.

Pinned image: `sha256:80bed826ce3b998e8ba75b85b25d18055066760982413e4483dd9779c5f053b2`; observed Rust 1.98.1, libvips 8.16.1 and ffmpeg 7.1.5. The image has the vips shared library, without the CLI; its version was read through `vips_version_string`. Docker networking is `none`; local fixture listeners stay in 53300–53399. Cargo runs offline/locked with two build jobs and eight test threads. The host rustc slot loop is mounted into containers and holds the same machine-wide locks. Only one extra scratch target exists; the worktree's normal `rust/target` is preserved.

The `.scratch/pinned-runs-final.sh NAME COMMAND...` wrapper mounts the fresh clone at `/src`, registry read-only at its Cargo home, a rustc wrapper at `/rustc-wrapper`, `/tmp/rust-port-rustc-slots` at `/rustc-slots`, and the host slot-count file at `/slot-count`. It sets `RUSTC_WRAPPER=/rustc-wrapper`, `TMPDIR=/src/tmp-review`, `CI=1`, `CARGO_BUILD_JOBS=2`, test/dev debug=0 and both test port-range variables. Docker uses user 1000:1000, four CPUs and the exact image above. The rustc wrapper loops over the configured host slots with `flock`, holding its descriptor through the real compiler.

Commands are run from the assigned worktree unless noted. Oracle producer commands use the fresh clone's `rust/` root, `PARITY_IMAGE=ws16-reference:d7c7de92`, `PARITY_OWNER=ws16`, an isolated `ws16-runs-final` namespace and `CAMPFIRE_REFERENCE` pointing at its pinned archive. Each uses `runner --seed first_run --time 2026-03-02T16:00:00Z --freeze`, then the named producer under `reference-tools/slack/`: `run_views.rb`, `runs_http.rb`, `options_vectors.rb`, `client_vectors.rb`, `undo_huddle.rb`, and `sequence_vectors.rb workspace|personal` with all eleven retained scenario arguments listed above. The orchestrator verifies `git diff --exit-code -- rust/vectors/slack` after all regeneration; it adds no mask.


The exact wrapper and compiler-slot loop used for all pinned Cargo commands:

```sh
#!/bin/bash
set -euo pipefail
root=$(git rev-parse --show-toplevel)
name="$1"; shift
docker run --rm --name "$name" --network none --cpus 4 --user 1000:1000 \
  -v "$root/.scratch/ws16-runs-final2:/src" \
  -v "$root/.scratch/rustc-wrapper.sh:/rustc-wrapper:ro" \
  -v /tmp/rust-port-rustc-slots:/rustc-slots \
  -v /home/riels/.cache/rust-port/rustc-slots:/slot-count:ro \
  -v /home/riels/.cargo/registry:/src/rust/.cargo-home/registry:ro \
  -w /src/rust --env CARGO_HOME=/src/rust/.cargo-home \
  --env RUSTC_WRAPPER=/rustc-wrapper --env TMPDIR=/src/tmp-review \
  --env CI=1 --env CARGO_BUILD_JOBS=2 \
  --env CARGO_PROFILE_TEST_DEBUG=0 --env CARGO_PROFILE_DEV_DEBUG=0 \
  --env RUST_TEST_THREADS=8 --env CABLE_TEST_PORT_RANGE=53300-53399 \
  --env INTEGRATION_TEST_PORT_RANGE=53300-53399 \
  sha256:80bed826ce3b998e8ba75b85b25d18055066760982413e4483dd9779c5f053b2 "$@"

# /rustc-wrapper (read-only mount):
#!/bin/bash
rustc="$1"; shift
case " $* " in *" --crate-name "*) ;; *) exec "$rustc" "$@" ;; esac
slots=$(cat /slot-count)
while :; do
  for ((i=0;i<slots;i++)); do
    exec {fd}>>"/rustc-slots/$i"
    flock -n "$fd" && exec "$rustc" "$@"
    exec {fd}>&-
  done
  sleep 0.3
done
```

Rails regeneration orchestration (executed as `python3 .scratch/ws16-rerun-oracles.py`):

```python
from pathlib import Path
import subprocess,os
root=Path.cwd()/'.scratch/ws16-runs-final2'
rust=root/'rust'
env=dict(os.environ,PARITY_NAMESPACE='ws16-runs-final',PARITY_OWNER='ws16',PARITY_IMAGE='ws16-reference:d7c7de92',CAMPFIRE_REFERENCE=str(rust/'parity/.ci/reference'))
for script,args in [('run_views',[]),('runs_http',[]),('options_vectors',[]),('client_vectors',[]),('undo_huddle',[])]+[('sequence_vectors',['workspace']),('sequence_vectors',['personal'])]+[('sequence_vectors',['workspace',name]) for name in ['saved_reply','poll_reply','pending_quoted_reply','sent_reply','foreign_thread','room_event_schedule','claimed_session','claimed_google_account','claimed_google_identity','claimed_password','placeholder_authorship']]:
    subprocess.run([str(rust/'parity/bin/reference'),'runner','--seed','first_run','--time','2026-03-02T16:00:00Z','--freeze',str(rust/f'reference-tools/slack/{script}.rb'),*args],env=env,check=True)
subprocess.run(['git','-C',str(root),'diff','--exit-code','--','rust/vectors/slack'],check=True)
print('Slack regenerated oracle bytes: all controller/client/huddle and thirteen sequence vectors unchanged')
```

The first full workspace run with `--nocapture` had 3405 passing tests, zero failures and 12 explicit ignores. Its summary tool additionally rejected the single literal "skipping locally" diagnostic. Inspection of `controllers/presenters/test_support.rs:119` and the adjacent passing test line shows it is `missing_seed_may_skip_locally`, which deliberately passes a newly empty temporary directory and `ci=false`. The paired `missing_seed_fails_in_ci` also passed. All actual seed lookup uses the set `CI` flag and fails if seeds are absent; fresh seed validation passed. No skip-detection rule or parity mask was loosened. The final run retains standard libtest capture and reruns the full workspace after the clippy fix.

Clippy initially failed on the owned sample renderer's redundant result wrapping at `runs.rs:754`. `c04ab8bc` removes only the outer `Ok` and trailing `?`; error conversion and rendering are unchanged. Both production and all-target checks are repeated, without suppressing the lint.

Commands used for the final Cargo gates and source/ledger checks:

```sh
.scratch/pinned-runs-final.sh ws16-runs-suite cargo test --offline --locked --workspace --exclude html5ever --no-fail-fast -- --test-threads=8
python3 .scratch/summarize-suite.py .scratch/ws16-pinned-workspace.log
.scratch/pinned-runs-final.sh ws16-runs-clippy cargo clippy --offline --locked --workspace --all-targets -- -D warnings
.scratch/pinned-runs-final.sh ws16-runs-release bash ci/with-release-inputs.sh cargo build --offline --locked --bin campfire
.scratch/pinned-runs-final.sh ws16-runs-metadata cargo metadata --offline --locked --format-version 1
python3 .scratch/ws16-runs-final2/rust/reference-tools/slack/check_reference.py
python3 .scratch/ws16-runs-final2/rust/reference-tools/slack/write_test_inventory.py
.scratch/pinned-runs-final.sh ws16-runs-mutations env WS16_CARGO=cargo python3 reference-tools/slack/check_run_mutations.py
```

The full workspace test command follows `rust/AGENTS.md` and excludes only vendored `html5ever`; final clippy includes every workspace member and all targets. No executable test filter is used in the full run. The initial `--nocapture` run supplies the extra body/HTTP/DB differential lines below; the final standard-capture run supplies the final raw totals. No original test assertions were removed.

Raw source/ledger and repeated Rails oracle summary lines:

```text
Slack Rails reference: 21 source files match d7c7de9264c63015be398001d7a1094e7695a6db; no Slack drift on origin/main
Slack Rails test inventory: 214 covered, 4 partial, 26 deferred; 244 total
Slack run views: 83 complete Rails template bodies generated with shared deterministic CSRF inputs
Slack run HTTP oracle: 109 real Rails action cases with signed sessions and verified CSRF generated
Slack options oracle: 33 Ruby coercion and ISO time-bound cases generated
Slack client vectors: 32 Rails error and success mappings generated
Slack undo huddle oracle: real Rails User.destroy callbacks; all rows and fields in six affected tables generated
Slack Rails sequence (workspace): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (personal): import -> undo -> reimport; 89 tables per snapshot; 18 recorded API requests
Slack Rails sequence (saved_reply): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (poll_reply): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (pending_quoted_reply): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (sent_reply): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (foreign_thread): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (room_event_schedule): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (claimed_session): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (claimed_google_account): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (claimed_google_identity): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (claimed_password): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (placeholder_authorship): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack regenerated oracle bytes: all controller/client/huddle and thirteen sequence vectors unchanged
```

Raw seed validator summaries:

```text
  "passed": 29,
  "failed": 0
WS16 fresh seed validated: default
  "passed": 4,
  "failed": 0
WS16 fresh seed validated: first_run
```

Raw failing-first run HTTP/body mutation output:

```text
test controllers::slack::tests::run_tests::slack_run_http_actions_sessions_csrf_rows_audits_and_jobs_match_rails ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1728 filtered out; finished in 1.88s
test controllers::slack::tests::run_tests::slack_run_http_actions_sessions_csrf_rows_audits_and_jobs_match_rails ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1728 filtered out; finished in 1.45s
test controllers::slack::tests::run_tests::slack_run_views_match_every_rails_body_byte ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1728 filtered out; finished in 0.82s
Slack run mutation guards: member-admin and foreign-personal access, and plan bytes rejected; sources restored
```

Additional owned parity lines from the initial full workspace run at `4f94fd73` (the final run repeats these same tests at `c04ab8bc`):

```text
Slack run view parity: 83 complete Rails template bodies matched byte for byte
Slack DB differential (personal): import -> undo -> reimport; 89 tables x 3 snapshots; every row and field matched
Slack DB differential (workspace): import -> undo -> reimport; 89 tables x 3 snapshots; every row and field matched
Slack DB differential (saved_reply): import -> undo -> reimport; 89 tables x 3 snapshots; every row and field matched
Slack DB differential (poll_reply): import -> undo -> reimport; 89 tables x 3 snapshots; every row and field matched
Slack DB differential (pending_quoted_reply): import -> undo -> reimport; 89 tables x 3 snapshots; every row and field matched
Slack DB differential (sent_reply): import -> undo -> reimport; 89 tables x 3 snapshots; every row and field matched
Slack DB differential (foreign_thread): import -> undo -> reimport; 89 tables x 3 snapshots; every row and field matched
Slack DB differential (room_event_schedule): import -> undo -> reimport; 89 tables x 3 snapshots; every row and field matched
Slack DB differential (claimed_session): import -> undo -> reimport; 89 tables x 3 snapshots; every row and field matched
Slack DB differential (claimed_google_account): import -> undo -> reimport; 89 tables x 3 snapshots; every row and field matched
Slack DB differential (claimed_google_identity): import -> undo -> reimport; 89 tables x 3 snapshots; every row and field matched
Slack DB differential (claimed_password): import -> undo -> reimport; 89 tables x 3 snapshots; every row and field matched
Slack DB differential (placeholder_authorship): import -> undo -> reimport; 89 tables x 3 snapshots; every row and field matched
Slack run HTTP parity: 109 Rails action cases matched sessions, CSRF, redirects, flashes, rows, audits and durable jobs
Slack undo huddle parity: six Rails affected tables matched every row and field; stream/presence callbacks emitted
```

Final full workspace totals, all raw libtest summaries and every explicit ignore, in execution order:

```text
Pinned workspace totals: 58 summary blocks; 3405 passed; 0 failed; 12 ignored
Raw libtest summaries:
test result: ok. 1726 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 873.10s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.89s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.42s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 977 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 123.73s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.69s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.95s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.04s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.58s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.20s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.58s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 33.86s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.93s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.93s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.30s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.39s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.85s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Explicit ignored tests:
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test controllers::internal_huddle_tests::huddle_gateway_own_node_suite_against_rust_endpoints ... ignored, requires Node and the gateway pinned ws package; run explicitly with --ignored
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

Final clippy and crates-only restricted-input binary build:

```text
    Finished `dev` profile [unoptimized] target(s) in 51.01s
    Finished `dev` profile [unoptimized] target(s) in 1m 01s
```

All final commands returned zero. The huddle gateway Node suite is an explicit upstream ignore; the huddle result here claims the Rust callback/row/event/rollback coverage, not that ignored gateway suite. No actual seed-dependent case skipped, and no measured or filtered test occurred in the final full run.

Seed validation was repeated from the fresh clone using the following exact orchestration (stdout provided above):

```python
from pathlib import Path
import subprocess, os
rust = Path.cwd() / ".scratch/ws16-runs-final2/rust"
env = dict(os.environ, PARITY_NAMESPACE="ws16-seed-final", PARITY_OWNER="ws16",
           PARITY_IMAGE="ws16-reference:d7c7de92",
           CAMPFIRE_REFERENCE=str(rust / "parity/.ci/reference"))
for seed in ["default", "first_run"]:
    subprocess.run([str(rust / "parity/bin/reference"), "runner", "--seed", seed,
                    "--time", "2026-03-02T16:00:00Z", "--freeze",
                    str(rust / "reference-tools/campfire/verify_parity_seed.rb"), seed],
                   env=env, check=True)
    print("WS16 fresh seed validated: " + seed, flush=True)
```

Locked/offline metadata: valid; workspace members: 13

## Cleanup and remote state

After all gates passed, native Cargo cleanup removed the sole extra target. Checked removal was limited to an empty target directory if one remained; fixture/source artifacts and the original worktree `rust/target` cache are preserved. Recursive scratch target inventory and WS16 container/port inventories are zero. No verification process remains.

```sh
.scratch/pinned-runs-final.sh ws16-runs-clean cargo clean --offline --locked
```

```text
     Removed 19658 files, 11.4GiB total
Scratch Cargo targets remaining: 0
Original worktree rust/target preserved: yes
WS16 verification containers running: 0
Listeners in WS16 port range: 0
```

The final report-only commit is pushed and its remote SHA is read back before the worker reply. No executable source changes follow verified `c04ab8bc`.
