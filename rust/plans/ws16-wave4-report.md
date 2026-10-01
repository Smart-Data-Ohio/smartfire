# WS16 — Slack importer foundations (PARTIAL)

Branch: `rust/ws16-slack-import`. Implementation slices pushed as `ed7878bb` (database lifecycle/credentials) and `49434c01` (converter/client). The final verification/report commit follows them; its pushed SHA is in the worker reply.

**Acceptance is not met. The importer cannot yet run through the UI or jobs.** This delivery is a coherent domain foundation, deliberately unwired until the remaining runner, undo and controller work lands. No Rails files, schema, parity masks or allowlists were changed. No PR was opened.

## Reference

Pinned Rails is `d7c7de9264c63015be398001d7a1094e7695a6db`. The reference container's 21 Slack model/job/controller files were compared byte-for-byte with that commit. `origin/main` at `65ad0d39` contains no Slack changes after the pin, including the recent LIFO undo, kept parent/pending scheduled reply, finish-every-room, and later-importer naming rules. Thus these files use the pin, not an unneeded main override. Actual destructive undo and finishing rules are still deferred below; only LIFO eligibility and blocking/name text are implemented.

The reference image is locally tagged `ws16-reference:d7c7de92`; all reference runner containers use the `ws16` namespace and no network. Default/first_run seeds were built from the archived pin with WS19 tooling and validated. Existing imports seed tooling was not changed because no Slack HTML rendering is delivered.

## Changes by file

- `rust/crates/db/src/models/slack_import.rs`: typed creation inputs, row reads, atomic run claim, UTC microsecond JSON lease stamps, ownership-token acquire/refresh/release, pending-job stamps/cooldown, oldest-run kick, stalled sweep, cancellation, failure marking, LIFO overlap scan with crash-mapping fallback, exact undo-blocking text including the later actor, atomic undo claim/reset, capped issues, and StepJob/UndoJob argument types.
- `rust/crates/db/src/models/slack.rs`: workspace/connection row reads and creates, required validations, private ciphertext, Rails-compatible encryption/decryption, connected/configured predicates, and connection deletion with run nullification. Neither credential model implements Debug.
- `rust/crates/db/src/models.rs` and `src/tests.rs`: register the two modules and their tests.
- `rust/crates/db/src/tests/slack_import_test.rs`: 20 lifecycle tests, including independent SQLite writer races, stale takeover, old-token rejection, lease-preserving cancellation, queued and stalled sweep overlap, undo serialization/LIFO/name text, cap, association checks and rollback when the enqueue sink rejects persistence.
- `rust/crates/db/src/tests/slack_test.rs`: 4 credential tests, Rails-written envelopes read in Rust, Rust-written row encryption, wrong-key rejection, presence/unique-index behavior and dependent nullification. An optional output path exports ciphertext for actual Rails model readback; tests never require that file to exist.
- `rust/crates/campfire/src/integrations/slack/{mod.rs,markdown.rs}`: pure mrkdwn conversion and its corpus/timing tests. Rails vectors caught and fixed an extra newline in empty fenced blocks. Successful ordinary Slack payloads match all 481 fixture/corpus/generated vectors byte-for-byte, including flags, file links, mentions, Unicode truncation and URL/code shielding.
- `rust/crates/campfire/src/integrations/slack/client.rs` and `client/tests.rs`: fixed Slack host, user-token GETs for all seven read endpoints, per-tier 18/min and 45/min pacing, four-attempt retry budget, 1/2/4-second backoff when pacing is enabled, Retry-After propagation/defaults, Rails error mapping vectors, and callback counts. Tests use real local TLS with a Slack SAN and fake DNS/dialing on ports 53300–53399, never slack.com.
- `rust/crates/campfire/src/integrations/slack/fixtures/*.json`: all 17 Slack JSON fixtures vendored byte-identically inside Rust, with a test comparing them to our Rails files. No build-time includes escape `rust/`.
- `rust/crates/campfire/src/integrations.rs`: register the staged Slack domain with dead-code allowance until its runtime consumers are implemented. No runtime route/job/scheduler registrations are added.
- `rust/reference-tools/slack/{markdown_vectors.rb,client_vectors.rb,crypto_vectors.rb}` and `rust/vectors/slack/{markdown.json,client.json,crypto.json}`: our Rails oracles, reproducible semantic inputs, deliberately fake token/secret plaintexts. Ciphertext IVs are random on regeneration.
- `rust/reference-tools/slack/verify_rust_crypto.rb`: loads Rust-produced ciphertext into real Rails model columns with raw SQL, decrypts it and checks both rows are valid.
- `rust/reference-tools/slack/check_lease_mutations.py`: rejects five deliberately broken implementations and restores both source files even on failure.
- `rust/reference-tools/slack/{check_reference.py,write_test_inventory.py}` and `rust/plans/ws16-test-inventory.md`: live source-pin check and an exhaustive mapping of all 244 Rails Slack tests. **66 covered behaviors, 6 partial, 172 deferred; every partial/deferred test has owner WS16 continuation.** No claim is made that the original Rails test suite ran against Rust.

## Design, validations and callbacks

Domain modules have no HTML dependencies. Mutations require the existing BEGIN IMMEDIATE writer transaction. Leases use one conditional UPDATE and json_set/json_remove, preserving unrelated cursor/progress keys. Claim and undo scans are serialized with new run creation by the writer lock and retain Rails' conditional predicates. A stopped run's fresh lease keeps claims blocked; completed lease-looking state does not. Sweep may enqueue two stale-running jobs, as Rails does; competing workers can acquire only one fresh step lease. Overlapping queued sweeps stamp/enqueue exactly once.

SQL leases stay in the db model rather than importing campfire_jobs' generic JsonLease: jobs already depends on db. No circular dependency or shared jobs-crate change is introduced.

Jobs are emitted through the established EventSink.persist/after-commit seam, so rejecting persistence rolls back the model operation. This is tested with an injected rejecting sink, **not** a full HTTP path or registered durable Slack queue. The real queue/runtime integration is deferred.

| Written model | Rails contract implemented | Still deferred |
|---|---|---|
| SlackWorkspace | client_id/client_secret presence; optional configured_by; encrypted secret; DB foreign keys/timestamps | credential update/removal, dependent workspace destruction and controller sudo/audit flow |
| SlackConnection | required workspace/user; slack_user_id presence; encrypted optional token; unique DB indexes; connected predicate; dependent imports nullify without timestamp touch | reconnect update, disconnect authorization/revocation/audit, user-deactivation hook |
| SlackImport | required workspace/user; optional connection; typed kind/mode; lifecycle status transitions; lease/heartbeat/save timestamps; job emission | Ruby option coercion/time-bound normalization; runner progress writes/completion; dependent deletion |
| SlackImport::Issue | required import; level enum; message presence; created_at only; cap plus one suppression notice | rendering and runner-generated warnings |
| SlackImport::Record | read only for LIFO crash fallback; test-only fixture inserts | production mapper/writer inserts, validation/association adapter and identity range helpers |

## Verification

All commands below were executed during this session from `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws16`, with Rust 1.98.1, CARGO_BUILD_JOBS=2 and at most 8 test threads. Commands are shown with their raw summary lines. The final root tests include the clippy cleanup (boxed optional scope payloads, unchanged retry behavior and a redundant test Result cleanup).

### Rails source and oracles

```sh
python3 rust/reference-tools/slack/check_reference.py
```

```text
Slack Rails reference: 21 source files match d7c7de9264c63015be398001d7a1094e7695a6db; no Slack drift on origin/main
```

```sh
PARITY_NAMESPACE=ws16 PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" rust/parity/bin/seed build default first_run
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

```sh
PARITY_NAMESPACE=ws16 PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb default
```

```text
"passed": 29,
  "failed": 0
```

```sh
PARITY_NAMESPACE=ws16 PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb first_run
```

```text
"passed": 4,
  "failed": 0
```

```sh
PARITY_NAMESPACE=ws16 PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 rust/parity/bin/reference runner rust/reference-tools/slack/markdown_vectors.rb
```

```text
Slack markdown vectors: 481 cases generated from Rails
```

```sh
PARITY_NAMESPACE=ws16 PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 rust/parity/bin/reference runner rust/reference-tools/slack/client_vectors.rb
```

```text
Slack client vectors: 12 Rails error and success mappings generated
```

```sh
PARITY_NAMESPACE=ws16 PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 rust/parity/bin/reference runner --seed default rust/reference-tools/slack/crypto_vectors.rb
```

```text
Slack encryption vectors: 2 Rails-written encrypted columns generated
```

### Deliberate failures and final owned tests

These assertions failed against deliberately broken sources, which the harness restored before the passing final tests. The plaintext-write mutation tests the actual credential writer, not a mocked encryption implementation.

```sh
TMPDIR="$PWD/.scratch/tmp" python3 rust/reference-tools/slack/check_lease_mutations.py
```

```text
cancelled lease ignored: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 687 filtered out; finished in 0.09s
fresh lease stolen: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 687 filtered out; finished in 0.10s
wrong token accepted: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 687 filtered out; finished in 0.33s
sweep duplicates queued job: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 687 filtered out; finished in 0.09s
plaintext credential write: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 687 filtered out; finished in 0.70s
Slack mutation guards: 5 broken implementations rejected; source restored
```

```sh
TMPDIR="$PWD/.scratch/tmp" SLACK_RUST_CRYPTO_OUTPUT="$PWD/.scratch/crypto/db/rust-slack-crypto.json" CARGO_BUILD_JOBS=2 mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire_db --lib slack_ -- --test-threads=8
```

```text
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 664 filtered out; finished in 3.73s
```

```sh
TMPDIR="$PWD/.scratch/tmp" CARGO_BUILD_JOBS=2 CABLE_TEST_PORT_RANGE=53300-53399 INTEGRATION_TEST_PORT_RANGE=53300-53399 CI=1 mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire integrations::slack -- --test-threads=8
```

```text
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 925 filtered out; finished in 4.67s
```

All 34 owned tests ran; none skipped/ignored. One app test executes all 481 converter vectors; another executes 12 Rails client mappings.

For reverse encryption readback, `.scratch/crypto/db/production.sqlite3` was reset to a fresh copy of the default seed and its old WAL/SHM files were removed, preserving the just-exported JSON file.

```sh
PARITY_NAMESPACE=ws16 PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 rust/parity/bin/reference runner --storage "$PWD/.scratch/crypto" rust/reference-tools/slack/verify_rust_crypto.rb
```

```text
Slack encryption readback: Rails decrypted and validated 2 Rust-written columns
```

### Clean checkout and baseline

A fresh shared-object clone at `49434c01` had no untracked fixtures or seeds. Its owned suite passed. Subsequent production edits only boxed the client scope error payloads and resolved clippy style findings; the final root suites above cover those edits.

```sh
git clone --shared --branch rust/ws16-slack-import "$PWD" .scratch/fresh
TMPDIR="$PWD/.scratch/tmp" CARGO_TARGET_DIR="$PWD/.scratch/fresh-target" CARGO_BUILD_JOBS=2 mise exec rust@1.98.1 -- cargo test --locked --manifest-path .scratch/fresh/rust/Cargo.toml -p campfire_db --lib slack_ -- --test-threads=8
```

```text
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 664 filtered out; finished in 2.97s
```

```sh
TMPDIR="$PWD/.scratch/tmp" CARGO_TARGET_DIR="$PWD/.scratch/fresh-target" CARGO_BUILD_JOBS=2 CABLE_TEST_PORT_RANGE=53300-53399 INTEGRATION_TEST_PORT_RANGE=53300-53399 CI=1 mise exec rust@1.98.1 -- cargo test --locked --manifest-path .scratch/fresh/rust/Cargo.toml -p campfire --bin campfire integrations::slack -- --test-threads=8
```

```text
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 925 filtered out; finished in 4.67s
```

### Broad workspace run and clippy

The default and first_run seeds were present and CI=1 enforced seed availability. The broad run completed 32 test executables: 2,095 passed, 1 failed, 9 explicitly ignored. In particular campfire passed 933 with 2 ignored; db passed 684 with 4 ignored. Cargo stopped at the storage-vector failure, so later test executables/doctests are not claimed to have run. No Slack-owned tests were ignored.

```sh
CARGO_BUILD_JOBS=2 CABLE_TEST_PORT_RANGE=53300-53399 INTEGRATION_TEST_PORT_RANGE=53300-53399 CI=1 mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever -- --test-threads=8
```

```text
test result: ok. 933 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 187.77s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.24s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 684 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 46.93s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.16s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.96s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.72s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.10s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.99s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.98s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.34s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.92s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.13s
```

The one failure is an enforced media prerequisite, not an unexplained parity mismatch:

```text
pipeline_matches_the_reference: pinned media required: expected libvips "8.16.1" / ffmpeg "ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers", found libvips 8.18.6 / ffmpeg ffmpeg version n9.0.2 Copyright (c) 2000-2026 the FFmpeg developers
```

It was reproduced after checking the fresh clone out to `origin/main` (`65ad0d39`), using the same separate target and host tools:

```sh
git -C .scratch/fresh checkout --detach "$(git rev-parse origin/main)"
TMPDIR="$PWD/.scratch/tmp" CARGO_TARGET_DIR="$PWD/.scratch/fresh-target" CARGO_BUILD_JOBS=2 CI=1 mise exec rust@1.98.1 -- cargo test --locked --manifest-path .scratch/fresh/rust/Cargo.toml -p campfire_storage --test vectors pipeline_matches_the_reference -- --test-threads=8
```

```text
pipeline_matches_the_reference: pinned media required: expected libvips "8.16.1" / ffmpeg "ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers", found libvips 8.18.6 / ffmpeg ffmpeg version n9.0.2 Copyright (c) 2000-2026 the FFmpeg developers
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.28s
```

Explicit ignored cases from that run:

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test jobs::tests::push_latency ... ignored, a measurement, not a test
test record_reference ... ignored, needs a running reference app; see the module docs
test tests::differential_test::scenario_matches_ruby ... ignored, needs CAMPFIRE_RUBY_SCENARIO_DB, the reference app's database after scenario.rb
test tests::fixtures_test::export_database_for_rails ... ignored, writes a database to CAMPFIRE_EXPORT_DB for the Rails rollback check
test tests::fixtures_test::fixtures_match_ruby_row_for_row ... ignored, needs CAMPFIRE_RUBY_FIXTURES_DB, a database the reference app filled with `db:fixtures:load` at CAMPFIRE_FIXTURES_NOW
test tests::two_factor_rollback_test::read_rails_rollback_changes ... ignored, requires Rails to mutate the rollback fixture; run reference-tools/auth/rollback.sh
test acme_tls_alpn_certificate_cached_and_reused ... ignored, requires a local Pebble ACME CA, PEBBLE_MINICA root certificate and TLS ports 5001/5002
test export_for_rails ... ignored, exports a database for the Rails rollback check; requires CAMPFIRE_MAIL_EXPORT_DIR
```

```sh
TMPDIR="$PWD/.scratch/tmp" CARGO_BUILD_JOBS=2 mise exec rust@1.98.1 -- cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
```

```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 23.84s
```

```sh
python3 rust/reference-tools/slack/write_test_inventory.py
```

```text
Slack Rails test inventory: 66 covered, 6 partial, 172 deferred; 244 total
```

## Remaining work — owner WS16 continuation

1. Port Ruby normalize_options precisely (Array/to_s/presence/string keys and ISO8601 date bounds); controller parameter coercion is not implemented. NewImport currently requires already-normalized options.
2. Complete workspace setup/update/removal and the manifest endpoint/view; OAuth v2 user scopes, authorize URL, exchange/revoke/team.info, session-bound single-use state and return paths, scope/team/account checks, sudo enforcement, audits and credential updates. No new OAuth security controller tests are claimed.
3. Register StepJob/UndoJob on the existing serial slack_import queue and the 30-second periodic sweep. Implement the 25-second runner phase machine, actual progress/heartbeat saving and token fencing, cancellation boundaries, connection/scope/auth errors, release-in-ensure and release-before-reenqueue, atomic real queue inserts and delayed Retry-After resumes. Prove these through real HTTP/queue/runner execution, not just the state primitives.
4. Implement UserMapper, ConversationMapper, MessageWriter and production Record model operations: matching/claimable placeholders/guests/bots, channel/private/DM/MPIM/archived policies, targeted-room validation and atomic setup, bulk quiet writes, source rendering, message/thread/reply timestamps/order, reaction/pin bookkeeping, identity-range/index lookups and catch-up/coverage.
5. Implement destructive Undoer and finishing for every target room, memberships/read pointers, kept records/foreign activity, saved/poll/pinned message preservation, kept thread parents and pending scheduled reply rules. Existing LIFO eligibility must be used; it does not implement deletion.
6. Finish client parity beyond this fixture foundation: exact Ruby transport exception class/message text (currently a generic network message), socket/reset/timeout behavior including status-body reads, response sequence recovery, malformed JSON shapes and Ruby coercion/error semantics, and explicit newest-first fixture assertions. The converter likewise has no malformed attachment/files/hash-coercion error parity; successful normal Slack payloads and the committed corpus are what has been proved.
7. Add all admin/personal connection/import/run/status/issue controllers, authorization branches and views; byte-identical HTML/JSON/frames against the existing imports parity seed; implement the system interaction test. No rendering/controller behavior or system interactions are completed here.
8. Run full fixture import → undo → reimport against both implementations, comparing DB rows and Rails validations; finish all individually deferred tests in `rust/plans/ws16-test-inventory.md`. The full workspace gate also needs a canonical media-tool environment and the tests/doctests after the current storage prerequisite failure. Owning infrastructure workstreams: WS18/WS19; Slack behavior/test continuation remains WS16.

## Cross-workstream boundaries, questions and cleanup

Only db module/test registries and the app integration registry were touched outside the owned Slack files; no other workstream implementation was duplicated. WS1 encryption was reused unchanged. WS3's queue and periodic infrastructure were inspected but not modified or registered prematurely. WS8 message callbacks/domain APIs were not changed. No allowlists/masks were added or loosened.

No product decision is reopened. The main operational limitation is host media version drift; the main implementation limitation is the deliberately staged runtime/UI.

The fresh-checkout/baseline target `.scratch/fresh-target` was removed after its results were captured. No test process or persistent reference container remains from these checks. Primary `rust/target` and untracked scratch logs/crypto fixtures remain in this worktree for review; there is no cargo target under `.scratch/`. No stash, release profile, extra job count, external Slack call, PR, or pixel work was used.
