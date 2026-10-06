# WS16 PR #195 enrollment boundaries — complete

Tested executable and test source: `0d94b6a345163aed38d6fd463537679cf2d5530c`. Branch: `rust/ws16-placeholder-claim`. The final reply identifies the report-only pushed SHA. This report supersedes the prior PR #195 review report for this bounded follow-up.

## Changes and exact Rails boundaries

`crates/campfire/src/authentication.rs::enroll` now orchestrates separate database writes; `controllers/two_factor.rs::setup_create` calls that domain operation and retains the separate audit write. It shares the app's existing encryption handle across writes. No rendering, request or cookie logic enters the domain layer.

Pinned `TwoFactor::SetupsController#create` has no enclosing action transaction. The Rails SQL notification trace records these mutation groups, with BEGIN/COMMIT or BEGIN/ROLLBACK around each:

| Group | Statements sharing the transaction | Effect of a later failure |
| --- | --- | --- |
| Pending credential creation | One credential INSERT, when absent (`create_or_find_by!`) | The unconfirmed credential remains after confirmation failure. |
| Confirmation | Credential UPDATE (secret, confirmation, replay stamp, timestamp) and current setup-secret DELETE (`with_lock`) | Both roll back together; pending credential creation remains. |
| Backup replacement | Delete the old code set and INSERT all ten digests | A rejected fourth INSERT rolls back the three tentative codes and restores an existing old set. Confirmation remains. |
| Verification | Current session UPDATE (`update!`) | Enrollment and all ten codes remain if verification fails. |
| Each other session | Huddle callback, dependent presence/setup-secret DELETEs, session DELETE (`destroy_all` calls individual `destroy`) | A failed deletion rolls back that session's dependents; earlier session destructions remain committed. Durable jobs remain atomic with their individual triggering destruction. |
| Enable audit | Audit INSERT (`record!`) | Enrollment, codes, verification and successful destructions remain after audit failure. |

None of these Rails mutation statements autocommits on its own outside a model transaction. Controller relation reads run without an action transaction; model validation/lock reads belong to their model transaction. The best-effort cable disconnect is outside the persisted mutation groups. Rust uses one write per group; the disconnect runs separately after persisted steps and cannot undo them. Empty/read-only transactions do not count as mutation groups in the differential.

`crates/db/src/models/two_factor.rs::extend_expiry` also skips an unchanged expiry, matching Active Record's dirty tracking. The wrong-code retry otherwise issued an UPDATE and timestamp touch that Rails skipped at the same frozen time.

## Differential and failing-first evidence

New `app/google_tests/slack_claim/boundaries.rs` and `reference-tools/slack/google_enrollment_boundaries.rb` exercise actual signed Google callbacks, first-factor sessions, real TOTP, real CSRF verification and real SQLite triggers. The helper is loaded by the existing Google claim review producer. Its six pinned source hashes and thirteen scenarios are stored in `vectors/slack/google_enrollment_boundaries.json`.

The cases reject: credential insertion; confirmation; current setup-secret consumption; old-code deletion; fourth new-code insertion with and without an old set; session verification; first and second other-session destruction; the second session's setup-secret deletion; and the audit. Success and wrong-code retry also compare mutation groups. Each case compares the complete response body, status, Location and Content-Type, retained credential timestamps/replay stamp and decrypted fixture properties, full backup digests, session verification/activity, pending secret expiries, audit details, and the next setup response. Two extra sessions deliberately expose partial `destroy_all` completion and dependent-deletion rollback. New random unconfirmed secret ciphertext is not a stable oracle field; the test checks decryptability/presence, confirmed setup secret and preservation of the existing fixture secret.

The recorder uses SQLite STMT/PROFILE events to capture one client SQL call, equivalent to Rails' `sql.active_record` notification. Foreign-key/trigger subprograms can repeat the parent's STMT event; these are not extra client calls. Separate backup INSERTs remain separate, including the fourth rejected insert. No response rewrite, mask, allowlist or new authentication/entropy seam was added. Existing rendering/auth-input fixtures are reused; each independent Rails scenario clears fixture cache state, matching a fresh Rust app while retaining the rate-limit callbacks.

The final recorder was rerun against exact reviewed production `d8dcb3951e14b9c235a3de6bd67ac22a4129f65a` in the fresh clone with only the test/producer/vector patch applied. Production files were checked pristine. Nine cases fail retained-state assertions; success, audit failure and wrong-code retry fail transaction-group assertions. The early credential-insert failure is an already-correct control. All twelve discriminating cases fail before the fix, and all thirteen pass afterward.

Executed baseline and corrected commands through the pinned wrapper below:

```sh
# Fresh clone checked out d8dcb3951, with only the test/producer/vector patch:
.scratch/pinned-boundary.sh ws16-boundary-baseline cargo test --offline --locked -p campfire google_enrollment_boundary_ -- --test-threads=2 --nocapture
# Fresh clone restored, then fast-forwarded from GitHub to the tested source:
.scratch/pinned-boundary.sh ws16-boundary-controls cargo test --offline --locked -p campfire google_tests::slack_claim -- --test-threads=2 --nocapture
.scratch/pinned-boundary.sh ws16-boundary-two-factor cargo test --offline --locked -p campfire app::two_factor_tests:: -- --test-threads=2
```

Raw failing-first evidence:

```text
Baseline production source: d8dcb3951e14b9c235a3de6bd67ac22a4129f65a; only test module/producer registrations changed; production pristine
Rust enrollment boundary audit: status=500; credentials=1; backups=10; pending=0; verified=1; sessions=1; write transactions=2
assertion `left == right` failed: audit: write transaction boundaries
Rust enrollment boundary backup_delete: status=500; credentials=1; backups=2; pending=3; verified=0; sessions=3; write transactions=1
assertion `left == right` failed: backup_delete: retained rows
Rust enrollment boundary backup_insert: status=500; credentials=0; backups=0; pending=3; verified=0; sessions=3; write transactions=1
assertion `left == right` failed: backup_insert: retained rows
Rust enrollment boundary backup_replace_insert: status=500; credentials=1; backups=2; pending=3; verified=0; sessions=3; write transactions=1
assertion `left == right` failed: backup_replace_insert: retained rows
Rust enrollment boundary confirm_update: status=500; credentials=0; backups=0; pending=3; verified=0; sessions=3; write transactions=1
assertion `left == right` failed: confirm_update: retained rows
Rust enrollment boundary credential_insert: status=500; credentials=0; backups=0; pending=3; verified=0; sessions=3; write transactions=1
Rust enrollment boundary session_destroy_first: status=500; credentials=0; backups=0; pending=3; verified=0; sessions=3; write transactions=1
assertion `left == right` failed: session_destroy_first: retained rows
Rust enrollment boundary session_destroy_second: status=500; credentials=0; backups=0; pending=3; verified=0; sessions=3; write transactions=1
assertion `left == right` failed: session_destroy_second: retained rows
Rust enrollment boundary session_setup_delete_second: status=500; credentials=0; backups=0; pending=3; verified=0; sessions=3; write transactions=1
assertion `left == right` failed: session_setup_delete_second: retained rows
Rust enrollment boundary session_verify: status=500; credentials=0; backups=0; pending=3; verified=0; sessions=3; write transactions=1
assertion `left == right` failed: session_verify: retained rows
Rust enrollment boundary success: status=200; credentials=1; backups=10; pending=0; verified=1; sessions=1; write transactions=2
assertion `left == right` failed: success: write transaction boundaries
Rust enrollment boundary setup_delete: status=500; credentials=0; backups=0; pending=3; verified=0; sessions=3; write transactions=1
assertion `left == right` failed: setup_delete: retained rows
Rust enrollment boundary wrong_code: status=422; credentials=1; backups=0; pending=3; verified=0; sessions=3; write transactions=2
assertion `left == right` failed: wrong_code: write transaction boundaries
test result: FAILED. 1 passed; 12 failed; 0 ignored; 0 measured; 2132 filtered out; finished in 3.88s
```

Raw corrected controls:

```text
Rust enrollment boundary backup_delete: status=500; credentials=1; backups=2; pending=2; verified=0; sessions=3; write transactions=2
Rust enrollment boundary audit: status=500; credentials=1; backups=10; pending=0; verified=1; sessions=1; write transactions=7
Rust enrollment boundary backup_insert: status=500; credentials=1; backups=0; pending=2; verified=0; sessions=3; write transactions=3
Rust enrollment boundary backup_replace_insert: status=500; credentials=1; backups=2; pending=2; verified=0; sessions=3; write transactions=2
Rust enrollment boundary credential_insert: status=500; credentials=0; backups=0; pending=3; verified=0; sessions=3; write transactions=1
Rust enrollment boundary confirm_update: status=500; credentials=1; backups=0; pending=3; verified=0; sessions=3; write transactions=2
Rust enrollment boundary session_destroy_second: status=500; credentials=1; backups=10; pending=1; verified=1; sessions=2; write transactions=6
Rust enrollment boundary session_destroy_first: status=500; credentials=1; backups=10; pending=2; verified=1; sessions=3; write transactions=5
Rust enrollment boundary session_verify: status=500; credentials=1; backups=10; pending=2; verified=0; sessions=3; write transactions=4
Rust enrollment boundary session_setup_delete_second: status=500; credentials=1; backups=10; pending=1; verified=1; sessions=2; write transactions=6
Rust enrollment boundary setup_delete: status=500; credentials=1; backups=0; pending=3; verified=0; sessions=3; write transactions=2
Rust enrollment boundary success: status=200; credentials=1; backups=10; pending=0; verified=1; sessions=1; write transactions=7
Rust enrollment boundary wrong_code: status=422; credentials=1; backups=0; pending=3; verified=0; sessions=3; write transactions=1
Google → Slack claim HTTP parity: 30 responses; 2 Google verifications; 2 personal jobs; 9 ownership tables; 0 byte mismatches
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 2281 filtered out; finished in 5.18s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 2291 filtered out; finished in 2.69s
```

## Main, reference and fresh-clone gates

Merged main `573762b5987522edadbdb855532c5dc14a88b526` (#188) with merge commit `37f5d755576876e3c7b69d7e14f23b4bd77efa32`, then main `056ab49acffd52007334356ada0bdba3774c0fe2` (#191) with merge commit `d816ef164b47088808e18a8aa9352a0493024c84`. Locked metadata succeeded after both merges. Main was fetched again before the final suite and was still at 056ab49ac. No stash, rebase or Rails app edit.

Verification runs from a fresh GitHub clone in `.scratch/ws16-boundary-final`, restored after the baseline experiment and fast-forwarded to the tested source. It has its own target. Exact Rails pin `d7c7de9264c63015be398001d7a1094e7695a6db` was archived into its `rust/parity/.ci/reference`. All three seeds (`default`, `first_run`, `agents_ui`) were built fresh and validated by Rails. The Google/claim and admin-run producers use only the common brief's approved layout/assets drift in `ws16-reference:d7c7de92-layout-2e20b24c`; connection/setup uses the pure pin image. Enrollment's six source hashes match the pure pin.

Seed commands from the clone's `rust/`, with `PARITY_OWNER=ws16`, an isolated `ws16-` namespace, the pure pinned image and the absolute archived `CAMPFIRE_REFERENCE`:

```sh
parity/bin/seed build default first_run agents_ui
parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/reference-tools/campfire/verify_parity_seed.rb" default
parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze "$PWD/reference-tools/campfire/verify_parity_seed.rb" first_run
parity/bin/reference runner --seed agents_ui --time 2026-03-02T16:00:00Z --freeze "$PWD/reference-tools/campfire/verify_parity_seed.rb" agents_ui
```

Executed `python3 .scratch/boundary-reference-check.py` and `python3 .scratch/boundary-replay.py`. Replay runs `parity/bin/reference runner --seed <seed> --time 2026-03-02T16:00:00Z --freeze <absolute producer>` for `google_claim_http.rb` (default), `connections_http.rb` and `runs_http.rb` (first_run), followed by `git diff --exit-code -- rust/vectors/slack` in the clone. All three existing producers and the new fourth vector regenerate byte-identically, including 217 complete response records. Recorded/local providers only; no real Google or Slack calls or pixel work.

Raw seed/source/oracle summaries:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
seed: building agents_ui
seed: agents_ui -> parity/.seed/agents_ui (6.1M)
  "passed": 29,
  "failed": 0
WS16 fresh seed validated: default
  "passed": 4,
  "failed": 0
WS16 fresh seed validated: first_run
  "passed": 40,
  "failed": 0
WS16 fresh seed validated: agents_ui
Rails source check: 10 controller/model hashes match d7c7de92; 30 responses; 13 provider calls
Rails enrollment source check: 6 pinned source hashes; 13 cases; 26 complete responses
Google claim review oracle: 3 signed callback cases; preview SELECTs 10=2, 200=2; enrollment audit failure {enrolled: true, backups: 10, pending_secrets: 0, verified_sessions: 1, sessions: 1, audits: 0}
Rails enrollment boundary credential_insert: status=500; credentials=0; confirmed=false; backups=0; pending=3; verified=0; sessions=3; write transactions=1
Rails enrollment boundary confirm_update: status=500; credentials=1; confirmed=false; backups=0; pending=3; verified=0; sessions=3; write transactions=2
Rails enrollment boundary setup_delete: status=500; credentials=1; confirmed=false; backups=0; pending=3; verified=0; sessions=3; write transactions=2
Rails enrollment boundary backup_delete: status=500; credentials=1; confirmed=true; backups=2; pending=2; verified=0; sessions=3; write transactions=2
Rails enrollment boundary backup_insert: status=500; credentials=1; confirmed=true; backups=0; pending=2; verified=0; sessions=3; write transactions=3
Rails enrollment boundary backup_replace_insert: status=500; credentials=1; confirmed=true; backups=2; pending=2; verified=0; sessions=3; write transactions=2
Rails enrollment boundary session_verify: status=500; credentials=1; confirmed=true; backups=10; pending=2; verified=0; sessions=3; write transactions=4
Rails enrollment boundary session_destroy_first: status=500; credentials=1; confirmed=true; backups=10; pending=2; verified=1; sessions=3; write transactions=5
Rails enrollment boundary session_destroy_second: status=500; credentials=1; confirmed=true; backups=10; pending=1; verified=1; sessions=2; write transactions=6
Rails enrollment boundary session_setup_delete_second: status=500; credentials=1; confirmed=true; backups=10; pending=1; verified=1; sessions=2; write transactions=6
Rails enrollment boundary audit: status=500; credentials=1; confirmed=true; backups=10; pending=0; verified=1; sessions=1; write transactions=7
Rails enrollment boundary success: status=200; credentials=1; confirmed=true; backups=10; pending=0; verified=1; sessions=1; write transactions=7
Rails enrollment boundary wrong_code: status=422; credentials=1; confirmed=false; backups=0; pending=3; verified=0; sessions=3; write transactions=1
Rails enrollment boundary oracle: 13 cases; literal HTTP responses, retained rows and write transaction groups
Google → Slack claim Rails oracle: 30 HTTP responses; 2 Google verifications; personal preview/import completed; 9 ownership tables per stage
Slack connection HTTP oracle: 37 real Rails callback/disconnect/setup/remove cases generated
Slack run HTTP oracle: 119 real Rails action cases with signed sessions and verified CSRF generated
Fresh Rails HTTP replay: 3 producers; 217 recorded responses (186 original + 5 prior review + 26 enrollment boundary responses); committed vectors byte-identical
```

Pinned container wrapper (Cargo runs from `/src/rust` in the clone):

```sh
#!/bin/bash
set -euo pipefail
root=$(git rev-parse --show-toplevel)
name="$1"; shift
docker run --rm --name "$name" --network none --cpus 4 --user 1000:1000 \
  -v "$root/.scratch/ws16-boundary-final:/src" \
  -v "$root/.scratch/rustc-wrapper.sh:/rustc-wrapper:ro" \
  -v /tmp/rust-port-rustc-slots:/rustc-slots \
  -v /home/riels/.cache/rust-port/rustc-slots:/slot-count:ro \
  -v /home/riels/.cargo/registry:/src/rust/.cargo-home/registry:ro \
  -w /src/rust --env CARGO_HOME=/src/rust/.cargo-home \
  --env RUSTC_WRAPPER=/rustc-wrapper --env TMPDIR=/src/tmp-review \
  --env CI=1 --env CARGO_BUILD_JOBS=2 \
  --env CARGO_PROFILE_TEST_DEBUG=0 --env CARGO_PROFILE_DEV_DEBUG=0 \
  --env RUST_TEST_THREADS=4 --env CABLE_TEST_PORT_RANGE=53300-53399 \
  --env MAIL_TEST_PORT_RANGE=53300-53399 --env GITHUB_TEST_PORT_RANGE=53300-53399 \
  --env INTEGRATION_TEST_PORT_RANGE=53300-53399 \
  sha256:80bed826ce3b998e8ba75b85b25d18055066760982413e4483dd9779c5f053b2 "$@"
```

The wrapper uses the existing host slot loop and its configured four slots, CARGO_BUILD_JOBS=2, four CPUs and network none. Focused checks use two test threads and the workspace uses four. No extra rustc jobs/slot changes, external workers or model-server interaction.

Executed final gates:

```sh
.scratch/pinned-boundary.sh ws16-boundary-metadata cargo metadata --offline --locked --format-version 1
.scratch/pinned-boundary.sh ws16-boundary-workspace cargo test --offline --locked --workspace --exclude html5ever --no-fail-fast -- --test-threads=4
python3 .scratch/summarize-suite.py .scratch/boundary-workspace.log
.scratch/pinned-boundary.sh ws16-boundary-clippy cargo clippy --offline --locked --workspace --all-targets -- -D warnings
.scratch/pinned-boundary.sh ws16-boundary-release bash ci/with-release-inputs.sh cargo build --offline --locked --workspace --bins
```

Locked metadata decoded successfully with 13 workspace members; no lockfile change. The workspace uses rust/AGENTS.md's documented vendored html5ever exclusion. The release-input build exposes only Cargo files/crates plus the explicit asset context. No production source requires vectors/reference tools or files outside crates. Full suite totals, raw summaries and existing ignored cases follow; ignored tests are not counted as passes. No missing-seed skips, measured or filtered cases in the workspace run.

Raw workspace summaries and ignored cases:

```text
Pinned workspace totals: 59 summary blocks; 4276 passed; 0 failed; 14 ignored
Raw libtest summaries:
test result: ok. 2294 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 549.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.29s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1254 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 134.12s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.54s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.93s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.70s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.15s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.93s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.57s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.20s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.18s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.31s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.94s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.64s
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

Raw strict clippy and release-input build summaries:

```text
    Finished `dev` profile [unoptimized] target(s) in 4m 31s
    Finished `dev` profile [unoptimized] target(s) in 1m 47s
```

## Ownership, remaining work and cleanup

This is the authorized two-factor boundary correction in the Google → Slack claim flow: it touches the WS9 authentication domain/controller and setup-secret model, plus WS16 test/oracle/report files. No WS14g seam was needed. The four #187 board N+1 paths identified by Astra were not edited by this fix; they remain WS12/WS14g's responsibility. No WS16-owned item remains from this review after these gates; this slice is complete.

The scratch target was cleaned with `.scratch/pinned-boundary.sh ws16-boundary-clean cargo clean --target-dir /src/rust/target`; the empty directory was then removed. Primary rust/target is preserved. Raw cleanup:

```text
     Removed 20314 files, 13.1GiB total
WS16 cleanup: 0 scratch Cargo targets; 0 boundary-check containers; 0 listeners in 53300-53399; primary rust/target preserved
```
