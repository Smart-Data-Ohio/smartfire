# WS16 Google sign-in → Slack placeholder claim — complete

Verified executable SHA: `297eb0ae8fde93368452417292960cce9f4fbf00`. Branch: `rust/ws16-placeholder-claim`, created from origin/main `0700e59d4` after #184 and #190 merged. The final reply carries the report-only pushed SHA. This resolves the previous WS14g owner block; no requested work remains.

## Result and scope

The new pinned Rails producer and Rust HTTP test compare **30 complete response bodies as literal bytes**, status, Location, Content-Type and redirect flash, with ownership snapshots after every response. There are no body replacements, masks, asset normalizations or allowlist changes in this comparison. Rails executes its actual router/controllers through ActionDispatch::Integration::Session (Rack HTTP); Rust executes its actual Axum router over a local TCP listener with a browser cookie jar. Both use real sessions and CSRF checks. This does not claim a Rails TCP/Puma transport comparison or production provider availability.

The fixture first creates a claimable, passwordless `Jane Slack` / `Jane@smartdata.net` placeholder through the real Slack user/conversation/message mappers, including private-channel membership, a root message and a thread reply. A first Google sign-in with verified `jane@smartdata.net` links the existing row (ID 9000000001), preserves its name/email/role and historical ownership, and creates one GoogleIdentity without a GoogleAccount. The interaction then performs mandatory two-factor enrollment, Google sudo confirmation, Slack opt-in, personal preview and direct-message import. Final ownership has the same original user, seven selected memberships and four imported/historical messages. Direct replies, channel threads and thread membership are checked too.

Snapshots compare selected columns of users, memberships, messages, channel_threads, thread_memberships, slack_import_records, slack_imports, google_identities and audit_logs. Additional checks cover session ownership/verification timestamps, GoogleAccount count, Slack connection ownership/scopes, encrypted/decryptable token storage, password absence and two-factor enrollment with ten backup codes.

Negative interactions include invalid CSRF for Google start, enrollment, Google sudo and personal preview; preview before opt-in; forged Slack state; and replay of the consumed callback. All 13 outbound provider requests, Google PKCE forms and Slack query/form inputs match Rails. Google uses WS14g's existing Recorded transport, fixture JWKS and RS256 signer with the real merged token verifier. Slack uses only local recorded TLS responses; bearer headers are assembled at test time. Tokens are encrypted at rest and decrypt correctly. Neither real Slack nor real Google is called.

The background worker is stopped for this deterministic HTTP differential. Preview/import steps execute the actual lease/Runner/perform_import path, with a zero step budget and provider pacing disabled in both fixtures; this evidence does not claim production queue timing.

## Changes by file

- `crates/campfire/src/app/google_tests/slack_claim.rs` and registration in `app/google_tests.rs`: reuse WS14g's merged handlers/fixtures; full HTTP sequence, real cookie jar/CSRF, signed Google claims, job driving, ownership and provider-request comparisons.
- `crates/campfire/src/app/google_tests/slack_claim.json`: shared local Slack user, conversation, root/reply inputs, including pre-claim historical data.
- `reference-tools/slack/google_claim_http.rb` and `vectors/slack/google_claim_http.json`: Rails-generated 30-response oracle, initial and per-stage row snapshots, provider calls and source hashes.
- `controllers/slack/runs.rs` and `controllers/slack/setup.rs`: populate Rails' audit target labels `SlackImport #<id>` and `SlackWorkspace #<id>`. Existing User audit labels were already correct; the WS16 target audit found no other omitted labels.
- `controllers/presenters/accounts.rs` and `presenters/view_context.rs`: build the account logo's timestamp version using the request's time zone, matching Rails' `updated_at.to_fs(:number)`.
- `controllers/slack/run_tests.rs`, `controllers/slack/tests.rs`, `reference-tools/slack/runs_http.rb`, `reference-tools/slack/connections_http.rb`, and their vectors: expand the existing 119 run and 37 setup/connection cases to compare target_label. Their existing body comparator is unchanged.
- `reference-tools/slack/check_google_claim_mutations.py`: reject case-sensitive placeholder lookup, acceptance of forged Slack state and the missing workspace audit label; restore source bytes in finally and rerun the passing control.
- The following test-only cross-workstream seams fix issued entropy as fixture inputs, leaving authentication, verification, encryption, session creation and writes real.

## Flagged seams and cross-workstream touches

WS14g: `integrations/google/sign_in.rs`, `test_support.rs` and `controllers/slack.rs` add scoped `cfg(test)` state/nonce/PKCE entropy. A middleware on this test's HTTP server installs the inputs; no request parameter or production configuration can enable the seam. The real state signer, nonce verifier, JWT signature verifier, PKCE exchange and sudo check still run.

WS9: `crates/db/src/database.rs`, `db/src/lib.rs`, `db/src/models/two_factor.rs`, `campfire/src/app.rs` and `test_support.rs` add an optional per-database `FixtureAuthInputs`, behind the existing test-support feature. It fixes only issued TOTP-secret/backup-code entropy, using the existing two-factor view fixture's codes. The real pending-secret encryption, TOTP confirmation, credential insertion, spent-step/session verification and audit paths run. The default is None; ordinary builds expose no fixture input. Mandatory enrollment is completed through HTTP, without manually marking a session verified.

Test server: `controllers/presenters/test_support.rs` adds a boot helper combining the existing network/clock/environment fixture inputs. WS8 presenters receive the request-time-zone logo fix above. No Rails application source changed. No new dependency, migration, route, production auth bypass, test exclusion or pixel work was introduced.

## Rails reference and replay

Rails pin is `d7c7de9264c63015be398001d7a1094e7695a6db`. Seeds come from the pure pin. Body producers use `ws16-reference:d7c7de92-layout-2e20b24c`, containing only the common brief's approved application-layout / people.css / profile_card_controller.js drift. Setup/connection producer uses the pure pin image. The new oracle records hashes of ten Rails Google, enrollment and Slack controller/model sources; no Google/Slack Ruby source drift is introduced.

A fresh GitHub clone at `.scratch/ws16-claim-final` was updated to the tested executable SHA. Fresh default and first_run seeds were built and validated with Rails. All three affected producers were replayed there; the resulting committed vectors were byte-identical.

Executed preparation and replay:

```sh
python3 .scratch/prepare-claim-fresh.py
python3 .scratch/check-claim-reference.py
```

The preparation archives the exact pin into the clone's `rust/parity/.ci/reference`. The fresh seed build runs `parity/bin/seed build default first_run` with that reference and `PARITY_IMAGE=ws16-reference:d7c7de92`; Rails validations run the following script:

```python
from pathlib import Path
import subprocess, os
rust=Path.cwd()/'.scratch/ws16-claim-final/rust'
env=dict(os.environ,PARITY_NAMESPACE='ws16-claim-final',PARITY_OWNER='ws16',
 PARITY_IMAGE='ws16-reference:d7c7de92',CAMPFIRE_REFERENCE=str(rust/'parity/.ci/reference'))
for seed in ['default','first_run']:
 subprocess.run([str(rust/'parity/bin/reference'),'runner','--seed',seed,'--time',
  '2026-03-02T16:00:00Z','--freeze',str(rust/'reference-tools/campfire/verify_parity_seed.rb'),seed],env=env,check=True)
 print('WS16 fresh seed validated: '+seed,flush=True)
```

```python
from pathlib import Path
import os,subprocess
root=Path.cwd()/'.scratch/ws16-claim-final'
rust=root/'rust'
env=dict(os.environ,PARITY_NAMESPACE='ws16-claim-replay',PARITY_OWNER='ws16',CAMPFIRE_REFERENCE=str(rust/'parity/.ci/reference'))
for name,seed,layout in [('google_claim_http','default',True),('connections_http','first_run',False),('runs_http','first_run',True)]:
 env['PARITY_IMAGE']='ws16-reference:d7c7de92'+('-layout-2e20b24c' if layout else '')
 subprocess.run([str(rust/'parity/bin/reference'),'runner','--seed',seed,'--time','2026-03-02T16:00:00Z','--freeze',str(rust/f'reference-tools/slack/{name}.rb')],env=env,check=True)
subprocess.run(['git','diff','--exit-code','--','rust/vectors/slack'],cwd=root,check=True)
print('Fresh Rails HTTP replay: 3 producers; 186 responses; committed vectors byte-identical')
```

Raw seed and replay summaries:

```text
Rails source check: 10 controller/model hashes match d7c7de92; 30 responses; 13 provider calls
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
  "passed": 29,
  "failed": 0
WS16 fresh seed validated: default
  "passed": 4,
  "failed": 0
WS16 fresh seed validated: first_run
Google → Slack claim Rails oracle: 30 HTTP responses; 2 Google verifications; personal preview/import completed; 9 ownership tables per stage
Slack connection HTTP oracle: 37 real Rails callback/disconnect/setup/remove cases generated
Slack run HTTP oracle: 119 real Rails action cases with signed sessions and verified CSRF generated
Fresh Rails HTTP replay: 3 producers; 186 responses; committed vectors byte-identical
```

## Failing-first evidence and gates

Regression commit `7edad8a33` precedes the production fixes. Baseline and security-mutation proofs ran on `88263dd82`. For the baseline, the four production fix files in the fresh clone were temporarily restored to `7edad8a33`, while retaining the final strict-byte comparison. It failed at the first preview audit target_label comparison; seven earlier captured page bodies also differed solely in the account logo version. The proof script checks the mismatches, then restores exact source bytes in finally. The expanded workspace/setup audit assertion also fails when its label fix is removed.

The first full workspace run then exposed a fixture-placement mistake: the new shared input had been put in the Rails-mirrored Slack fixture directory. The existing directory-wide test failed. Commit `297eb0ae8` moves the byte-identical input beside the Google test and updates its two path references, without changing or skipping that test. The final executable has only those fixture-path changes after the proof SHA. A focused control and a complete workspace rerun verify the correction.

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2096 filtered out; finished in 1.03s
Baseline logo-body probe: 7 captured responses differ; Rust v=20260101160000; Rails v=20260101110000
Baseline import audit probe: preview-start target_label differs from Rails
Failing-first raw-body check: 7 captured pages; only the request-zone logo version differs
case_sensitive_claim: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2096 filtered out; finished in 0.79s
forged_slack_state: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2096 filtered out; finished in 0.93s
workspace_audit_label: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2096 filtered out; finished in 15.27s
Google → Slack claim HTTP parity: 30 responses; 2 Google verifications; 2 personal jobs; 9 ownership tables; 0 byte mismatches
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2096 filtered out; finished in 1.09s
Google → Slack claim mutations: 3 rejected; restored control passed; source restored
Fixture-location failure before correction:
test result: FAILED. 2091 passed; 1 failed; 5 ignored; 0 measured; 0 filtered out; finished in 540.38s
Fixture-location control after correction:
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2096 filtered out; finished in 0.00s
```

This proves the comparison detects a lost claim and a forged OAuth state in addition to the actual behavior regressions. Mutations are serial, source is restored before the full gates, and the restored control checks all 30 raw bodies.

Pinned Rust/media container wrapper and unchanged host slot loop:

```sh
#!/bin/bash
set -euo pipefail
root=$(git rev-parse --show-toplevel)
name="$1"; shift
docker run --rm --name "$name" --network none --cpus 4 --user 1000:1000 \
  -v "$root/.scratch/ws16-claim-final:/src" \
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

Raw pinned versions:

```text
rustc 1.98.1 (48a229cea 2026-09-01)
libvips 8.16.1
ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers
```

Executed gate commands (Cargo runs in the fresh clone, after Rails finished writing vectors):

```sh
bash .scratch/pinned-claim.sh ws16-claim-metadata cargo metadata --offline --locked --format-version 1
python3 .scratch/claim-proof-and-gates.py
python3 .scratch/claim-diff-check.py
python3 .scratch/claim-final-gates.py
```

The proof command runs the failing baseline and mutation checker above, restores source, then starts the first workspace run. That run exposed the fixture-location failure. After its completion and the fixture-only fix, the final gate script runs the focused mirrored-fixture control, followed by exactly:

```sh
cargo test --offline --locked --workspace --exclude html5ever --no-fail-fast -- --test-threads=4
cargo clippy --offline --locked --workspace --all-targets -- -D warnings
bash ci/with-release-inputs.sh cargo build --offline --locked --workspace --bins
```

Raw metadata, all workspace test/doc-test summaries, explicit ignores, strict clippy and release-input build lines:

```text
cargo metadata --offline --locked: 13 workspace members; success
Pinned workspace totals: 59 summary blocks; 4029 passed; 0 failed; 14 ignored
test result: ok. 2092 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 630.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.74s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.83s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1219 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 177.75s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.86s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.03s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.10s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.32s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 26.08s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.36s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.11s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.23s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.89s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.59s
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
    Finished `dev` profile [unoptimized] target(s) in 1m 14s
    Finished `dev` profile [unoptimized] target(s) in 1m 51s
```

The standard workspace command excludes vendored html5ever tests; strict clippy includes all workspace targets. Existing ignored tests are listed rather than counted as passes. Missing seeds fail under CI=1, and this run has no seed skips. Release-inputs validates production builds with only crates and the explicit asset context.

## Cleanup and remaining

```sh
bash .scratch/pinned-claim.sh ws16-claim-clean cargo clean --offline --locked
python3 .scratch/check-claim-cleanup.py
```

```text
     Removed 20023 files, 12.2GiB total
WS16 cleanup: 0 scratch Cargo targets; 0 claim-check containers; 0 listeners in 53300-53399; primary rust/target preserved
```

At most four test threads were used; all compilers used the unchanged machine-wide slot loop with Cargo jobs=2. No Python model-server process was touched. The normal primary rust/target is preserved; all scratch Cargo targets and this worker's listeners/containers are gone. No stash, rebase or force-push was used.

Remaining: **none for this request**. The combined real first-Google-sign-in → Slack opt-in/placeholder-claim comparison is complete, and the prior owner block is resolved. There are no deferred Rails interaction cases, open questions or partial items for this slice.
