# WS16 PR #195 review fixes — complete

Tested executable source: `3159d48a00365d6baf334c25b414317bde89cf26`. Branch: `rust/ws16-placeholder-claim`. The final reply carries the report-only pushed SHA. Requested P2 fixes and gates are complete; no WS16 item remains from this review.

## Fixes and failing-first evidence

Merged origin/main `7c23b097885101b39e432e8628ab9ebb1251d9be` (#187) using merge commit `c55c48ec4f455880e38808c7b10668f7805b1408`. No stash or rebase. Tests/oracle arrived in `98be16e11` with test API corrections in `c0f3978e1`, before production fixes. The baseline fresh clone checked out exact reviewed production `d98df3e53a10f452311c581ddbc1bf7d4ca758bd`, with only the new test/producer/vector patch applied. All seven focused regressions then failed through libtest assertions, not compilation failures. The clone was restored and fast-forwarded from GitHub before final gates.

1. **Google account selection** (`f0b4a3f6a`): `GoogleIdentity::resolve`, hosted-domain normalization and the deactivated-predecessor pattern use `rails_compat::unicode::downcase`. SQLite `LOWER` and `LIKE` retain Rails' ASCII semantics. Real RS256-signed, verified Google callbacks prove `ΟΣ@smartdata.net` claims `οσ@smartdata.net`, both alone and with an eligible `ος@smartdata.net` administrator; it also rejects the matching rewritten deactivated email. The original 30-response claim interaction is unchanged.

   Audit: Slack email matching, claimable-domain lookup, room matching and group sorting already use pinned Ruby casing (`slack::payload::downcase`) plus SQLite SQL where Rails does. Google subjects and Slack IDs remain exact opaque comparisons; workspace metadata updates retain exact equality. Google domain verification's remaining Rust lowercase operates on domain eligibility only: accepted hostnames are ASCII, and it returns the original verified email to account selection. It does not choose users or normalize their saved email. Profile normalization already uses the shared Ruby helper. No further identity comparison defect was found.

2. **Slack preview reads** (`096d97d48`): batch mapping keys in one scoped `SlackImport::Record` relation and eligible emails in one `User` relation. Deduplicate raw emails before Ruby downcase; preserve id-ascending user indexing so the last duplicate match wins. Dry previews include blank-id candidates while preparing the email index, as Rails does; empty pages return immediately. The existing mapping relation's bind/inlined-literal shape and lack of ORDER BY are preserved, including its analyzed-database group-order regression. Hash maps/sets here feed key lookups only, never output iteration. SQLite trace counts at 10 and 200 fresh members are now **2 and 2**, versus baseline **20 and 400**, using expectations and actual SQL recorded from Rails.

3. **Enrollment audit failure** (`3159d48a0`): confirmed enrollment, ten backup codes, verified session, other-session revocation and its durable jobs commit before the separate `two_factor.enable` audit write. A rejected audit still produces HTTP 500. This changes only the enable audit boundary; durable jobs still commit or roll back with their domain transaction. The Rails differential signs in through the actual Google callback, starts with an unverified session, enrolls with real TOTP and CSRF, and rejects the audit with a SQLite trigger. It compares the complete 500 body/status/Location/Content-Type, all six retained-state counts, and the next setup redirect. The older enrollment rollback test was corrected to the Rails retention expectation. No new authentication or entropy seam was added.

Baseline and corrected controls were executed as follows, through `.scratch/pinned-p2.sh` (wrapper below); baseline uses the d98df3e5 checkout, controls use the tested source:

```sh
cargo test --offline --locked -p campfire google_sigma_ -- --test-threads=2 --nocapture
cargo test --offline --locked -p campfire slack_preview_reads_match_rails_ -- --test-threads=2 --nocapture
cargo test --offline --locked -p campfire google_enrollment_audit_failure_ -- --test-threads=2 --nocapture
cargo test --offline --locked -p campfire enrollment_keeps_confirmed_state_if_the_audit_cannot_be_saved -- --test-threads=2 --nocapture
# Corrected controls:
cargo test --offline --locked -p campfire google_tests::slack_claim -- --test-threads=2 --nocapture
cargo test --offline --locked -p campfire slack_preview_reads_match_rails_ -- --test-threads=2 --nocapture
cargo test --offline --locked -p campfire app::two_factor_tests:: -- --test-threads=2 --nocapture
```

Raw failing-first summaries and observed divergence:

```text
Baseline production source: d98df3e53a10f452311c581ddbc1bf7d4ca758bd
sigma: test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 2100 filtered out; finished in 2.18s
Google identity unicode_other_user: owner=administrator; users=2; sessions=1
Google identity unicode_sigma: owner=new_user; users=2; sessions=1
preview: test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 2101 filtered out; finished in 0.56s
Slack preview members=200: SELECTs=400; Rails=2
Slack preview members=10: SELECTs=20; Rails=2
enrollment: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2102 filtered out; finished in 3.48s
Google enrollment audit failure: {"enrolled":false,"backups":0,"pending_secrets":1,"verified_sessions":0,"sessions":1,"audits":0}
enrollment-existing: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2102 filtered out; finished in 1.08s
d98df3e5 failing-first proof: all three P2s reproduced
```

Raw corrected controls:

```text
Google enrollment audit failure: {"enrolled":true,"backups":10,"pending_secrets":0,"verified_sessions":1,"sessions":1,"audits":0}
Google → Slack claim HTTP parity: 30 responses; 2 Google verifications; 2 personal jobs; 9 ownership tables; 0 byte mismatches
Google identity unicode_other_user: owner=placeholder; users=2; sessions=1
Google identity unicode_sigma: owner=placeholder; users=1; sessions=1
Google identity unicode_predecessor: owner=none; users=1; sessions=0
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 2127 filtered out; finished in 2.26s
Slack preview members=200: SELECTs=2; Rails=2
Slack preview members=10: SELECTs=2; Rails=2
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2130 filtered out; finished in 0.59s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 2124 filtered out; finished in 3.65s
```

## Reference, fresh clone and final gates

All verification uses a fresh GitHub clone at `.scratch/ws16-p2-final`, fast-forwarded to the executable SHA above, with its own Cargo target. Rails reference is exact `d7c7de9264c63015be398001d7a1094e7695a6db`, archived into that clone's `rust/parity/.ci/reference`. Default and first_run seeds were built afresh from the pure pin and validated by Rails. Google/claim and admin-run body producers use only the common brief's approved application-layout / people.css / profile_card_controller.js drift in `ws16-reference:d7c7de92-layout-2e20b24c`; setup/connection uses the pure pin image. No Rails application source was edited.

Executed seed/replay commands (namespace `ws16-p2` for seeds, `ws16-p2-replay` for replay; `PARITY_OWNER=ws16`; `CAMPFIRE_REFERENCE` set to the absolute archived pin):

```sh
parity/bin/seed build default first_run
# Each validation uses PARITY_IMAGE=ws16-reference:d7c7de92:
parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws16/.scratch/ws16-p2-final/rust/reference-tools/campfire/verify_parity_seed.rb default
parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws16/.scratch/ws16-p2-final/rust/reference-tools/campfire/verify_parity_seed.rb first_run
python3 .scratch/p2-reference-check.py
python3 .scratch/p2-replay.py
```

Replay calls `parity/bin/reference runner --seed <seed> --time 2026-03-02T16:00:00Z --freeze <absolute producer>` for `google_claim_http.rb` (default), `connections_http.rb` (first_run) and `runs_http.rb` (first_run), then checks `git diff --exit-code -- rust/vectors/slack` in the clone. All three vectors regenerate byte-identically, without masks. They contain the original 186 response records plus five review responses (three signed callbacks and enrollment's failure/next-request), alongside the two preview query observations. Rails uses actual router/controller Rack HTTP sessions; Rust uses its real Axum TCP router. Google token verification, real sessions/CSRF and TOTP remain active. Provider transports are recorded/local fixtures only; no real Google or Slack call, request/config authentication bypass, pixel work, or extra seam was introduced.

Raw seed/source/replay summaries:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
  "passed": 29,
  "failed": 0
  "passed": 4,
  "failed": 0
Rails source check: 10 controller/model hashes match d7c7de92; 30 responses; 13 provider calls
Google claim review oracle: 3 signed callback cases; preview SELECTs 10=2, 200=2; enrollment audit failure {enrolled: true, backups: 10, pending_secrets: 0, verified_sessions: 1, sessions: 1, audits: 0}
Google → Slack claim Rails oracle: 30 HTTP responses; 2 Google verifications; personal preview/import completed; 9 ownership tables per stage
Slack connection HTTP oracle: 37 real Rails callback/disconnect/setup/remove cases generated
Slack run HTTP oracle: 119 real Rails action cases with signed sessions and verified CSRF generated
Fresh Rails HTTP replay: 3 producers; 191 recorded responses (186 original + 5 review responses); committed vectors byte-identical
```

Pinned wrapper (all Cargo commands run from `/src/rust`, in the fresh clone):

```sh
#!/bin/bash
set -euo pipefail
root=$(git rev-parse --show-toplevel)
name="$1"; shift
docker run --rm --name "$name" --network none --cpus 4 --user 1000:1000 \
  -v "$root/.scratch/ws16-p2-final:/src" \
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

The wrapper uses the unchanged host rustc slot loop (currently four slots), `CARGO_BUILD_JOBS=2`, four CPUs, network none and CI. Focused tests use two test threads; the full workspace uses four. Every rustc compile waits for a host slot. The local model server was not touched.

Raw runtime versions:

```text
rustc 1.98.1 (48a229cea 2026-09-01)
libvips 8.16.1
ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers
```

Executed gates:

```sh
bash .scratch/pinned-p2.sh ws16-p2-metadata cargo metadata --offline --locked --format-version 1
bash .scratch/pinned-p2.sh ws16-p2-workspace cargo test --offline --locked --workspace --exclude html5ever --no-fail-fast -- --test-threads=4
python3 .scratch/summarize-suite.py .scratch/p2-workspace.log
bash .scratch/pinned-p2.sh ws16-p2-clippy cargo clippy --offline --locked --workspace --all-targets -- -D warnings
bash .scratch/pinned-p2.sh ws16-p2-release bash ci/with-release-inputs.sh cargo build --offline --locked --workspace --bins
```

`cargo metadata --offline --locked` decoded successfully with 13 workspace members. No dependency/lockfile change. Release inputs contain only Cargo files and crates plus the explicit asset build context; no production source requires vectors, tools or files outside crates. The full suite has no failures, measured/filtered cases or silent missing-seed skips. Existing ignored cases are listed verbatim below and are not counted as passes.

Raw complete workspace summaries and ignored cases:

```text
Pinned workspace totals: 59 summary blocks; 4098 passed; 0 failed; 14 ignored
Raw libtest summaries:
test result: ok. 2127 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 634.64s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.86s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1253 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 147.77s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.74s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.31s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.20s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.68s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.56s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.23s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.85s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.12s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.53s
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

Raw strict-clippy and release-input build summaries, respectively:

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 36s
    Finished `dev` profile [unoptimized] target(s) in 1m 26s
```

Raw final source-integrity check:

```text
Fresh clone unchanged: Rails vectors, locked dependencies and source match 3159d48a00365d6baf334c25b414317bde89cf26
All requested P2 gates passed
```

## Changed files, cross-workstream scope and remaining work

- `crates/db/src/models/google_identity.rs`: the three Google normalization sites (WS14g identity path).
- `crates/campfire/src/integrations/slack/users.rs`: batched page-level mapping-key and email relations; existing users_for SQL shape preserved.
- `crates/campfire/src/authentication.rs` and `controllers/two_factor.rs`: the enable audit is recorded after the enrollment transaction (WS9 boundary). Domain writes stay separate from rendering.
- `crates/campfire/src/app/google_tests/slack_claim/review.rs` and module registration in `slack_claim.rs`: three signed Unicode callback cases and the real Google → TOTP enrollment audit-failure differential; response fields and resulting account/session state checked.
- `crates/campfire/src/integrations/slack/users/tests.rs`: traced read-count regressions at two sizes, using Rails-generated observations.
- `crates/campfire/src/app/two_factor_tests.rs`: corrected the old rollback expectation to Rails' retained enrollment, codes, current session and revoked-other-session state.
- `reference-tools/slack/google_claim_review.rb`, `google_claim_http.rb`, and `vectors/slack/google_claim_http.json`: append the six reviewed scenarios without changing the original 30-response comparison or its 13 provider calls. Three callbacks + two previews + one enrollment failure are the six scenarios; the enrollment scenario records both failure and next response.

No new dependency, schema/migration, route, Rails application change, auth bypass, ignore annotation or production fixture-input path. WS14g/WS9 touches above are limited to the explicitly requested fixes, with no extra seams. No open question or owner-blocked WS16 item remains in this review. Existing manually driven reference/export checks, ACME infrastructure/docs/measurements and WS11-API polling comparisons remain the unchanged ignored cases listed above; this run does not claim they passed.

Executed cleanup:

```sh
bash .scratch/pinned-p2.sh ws16-p2-clean cargo clean --target-dir /src/rust/target
python3 .scratch/p2-cleanup-check.py
```

Raw cleanup:

```text
     Removed 20021 files, 12.6GiB total
WS16 cleanup: 0 scratch Cargo targets; 0 P2-check containers; 0 listeners in 53300-53399; primary rust/target preserved
```

Primary `rust/target` is preserved. Only regenerable scratch Cargo output was removed; logs and the fresh reference/seed evidence remain under `.scratch/`. The final report commit changes only `rust/plans/ws16-wave4-report.md` after the tested source; the shared external report is byte-identical. No further source changes follow the gates.
