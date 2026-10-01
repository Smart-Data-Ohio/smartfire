# WS16 PR #184 ordering follow-up — complete; Google claim interaction owner-blocked

This follow-up closes Astra's remaining P3 on a727f3f8. All WS16-owned, unblocked work is complete. The exact remaining dependency is WS14g's real first-Google-sign-in start/callback path on main for the combined placeholder-claim HTTP comparison. PR #184 is already open.

Verified executable SHA: `03cdf59e418824f7d2f9f5ae35efe24399dfac54`. The final reply supplies the report-only pushed SHA. Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws16`; branch: `rust/ws16-slack-import`. Fetching main confirmed `59ad94de3d0c53dfc15cae95901453f6506a7e83` is already an ancestor; no new merge was necessary. No stash or rebase.

## Changes and ordering audit

- `crates/campfire/src/integrations/slack/users.rs`: load mappings using Rails' indexed SQLite `IN` predicate from `UserMapper#users_for`, with the id set carried by `json_each` in one binding, retaining its actual row order in `IndexMap`. This matters because Rails' mapping order differs from both record insertion order and incoming member order. Compact and deduplicate requested ids before constructing SQL, like Rails' compact_blank.uniq; 40,000 repeated ids collapse to one key, and large distinct sets still use one JSON binding.
- `conversations.rs`: preserve the ordered map through the stable Ruby-downcase name sort. For `Alice`, `alice`, `ALICE`, `Z0`–`Z7`, both directly supplied users and SQL-loaded users produce exactly `Alice, alice, ALICE +8`. Preview maps also preserve input insertion order. Membership inversion now selects the last alias in the original map, like Ruby Hash#invert, instead of imposing a sorted-key order.
- `store.rs`: keep the ordered mapping through missing-author appends. Finish rooms in first-occurrence order from written conversation ids followed by mapping rows, matching Rails' concatenation and `uniq`, using `IndexSet`.
- `writer.rs`: preserve ordered users across pages, mentions and author fallback. Mention ids use `IndexSet`; only missing keys are loaded, matching Rails' `users.merge!(users_for(missing))`. Markdown receives its existing keyed display-name map.
- `crates/campfire/Cargo.toml` and `Cargo.lock`: reference the already pinned workspace indexmap dependency. No dependency version changed.
- `reference-tools/slack/ordering_vectors.rb`, `vectors/slack/ordering.json` and five tests beside conversations/store/writer: record the actual pinned Rails group name, mapping order, missing-author appends, mention merge, alias membership keys and finishing sequence. The producer reverses member and record insertion order to distinguish correct SQL preload order from merely preserving the input array. Rust's finishing regression records actual room updates through a SQLite trigger.

The audit covers Slack integrations, jobs, controllers, views and Slack database models. Remaining HashMap/HashSet uses do not supply output order: Ruby-downcase scalar lookup; API-tier timing lookup; email lookup; known-user/message/member/domain membership checks; markdown mention lookup; inverse membership lookup after ordered construction; and touched-conversation overlap checks whose result is an order-independent boolean. Test-only query parameter and catch-up maps also use keyed lookups. Conversation discovery and SQL row/list ordering retain their existing Rails behavior. No mask, allowlist, exclusion, HTTP golden normalization, or Rails application source changed. No real Slack calls or literal auth headers were introduced.

## Failing-first evidence

Before production changes, the five new regressions ran against a727f3f8's implementation. The tied-name test completed all 50 repetitions (100 group resolutions), collecting mismatches instead of aborting at the first one. The finishing test also completed all 50 repetitions. A preliminary fixture run exposed a missing message UUID; the corrected fixture run below reached all five semantic assertions.

```sh
CARGO_BUILD_JOBS=2 CABLE_TEST_PORT_RANGE=53300-53399 INTEGRATION_TEST_PORT_RANGE=53300-53399 mise exec rust@1.98.1 -- cargo test --offline --locked --manifest-path rust/Cargo.toml -p campfire slack_ordering_ -- --test-threads=2 --nocapture
```
```text
Slack tied-name probe: 50 repetitions; 100 group resolutions; 142 order mismatches
0: mapped name Some("ALICE, Alice, alice +8")
  left: Array [String("CALIAS:UB"), String("CALIAS:UZ")]
 right: Array [String("CALIAS:UA"), String("CALIAS:UB")]
  left: Array [String("U10"), String("U00"), String("U02"), String("U01")]
 right: Array [String("U10"), String("U00"), String("U01"), String("U02")]
Slack finishing-order probe: 50 repetitions; 37 order mismatches
test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 1888 filtered out; finished in 1.14s
```

The same command after the ordering fix (before adding the large-duplicate edge):

```text
Slack tied-name probe: 50 repetitions; 100 group resolutions; 0 order mismatches
Slack finishing-order probe: 50 repetitions; 0 order mismatches
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 1888 filtered out; finished in 1.55s
```

The new batch query was also checked with 40,000 repeated ids against Rails. Before deduplication, the regression reached SQLite and failed with `too many SQL variables`:

```sh
CARGO_BUILD_JOBS=2 CABLE_TEST_PORT_RANGE=53300-53399 INTEGRATION_TEST_PORT_RANGE=53300-53399 mise exec rust@1.98.1 -- cargo test --offline --locked --manifest-path rust/Cargo.toml -p campfire slack_ordering_tied_group_names_repeat_rails_order_50_times -- --test-threads=1 --nocapture
```
```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1892 filtered out; finished in 1.20s
```

Rails also accepted 40,001 distinct requested ids. The parameter-list version failed this additional regression before the one-JSON-binding change:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1892 filtered out; finished in 0.66s
```

The final focused five-test command passed with both large-id cases included:

```text
Slack tied-name probe: 50 repetitions; 100 group resolutions; 0 order mismatches
Slack finishing-order probe: 50 repetitions; 0 order mismatches
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 1888 filtered out; finished in 1.40s
```

The full fresh-clone suite and process repeat below include the corrected duplicate case. The initial fresh suite was deliberately interrupted to fix this query edge; only the final completed run is included in totals.

## Fresh-clone Rails and Rust verification

The independent GitHub clone `.scratch/ws16-order-final` was created from the pushed branch and fast-forwarded to the final verified executable SHA before the final gates, without object alternates or source, target or seed copies from the primary worktree. The final build reused only artifacts built inside this same fresh clone during the interrupted run. Both seeds were newly built by pinned Rails and explicitly checked. The final expanded ordering producer was replayed separately after the large-id observations were added, and a Git diff checked all current vectors and the Ruby mapping table again. All 27 producers (the existing 26 plus the new ordering producer) regenerated byte-identical vectors and the production Ruby downcase table. This includes 119 full run HTTP bodies with sessions/CSRF, 13 import→undo→reimport sequences across 89 tables and the earlier review regressions. The original declaration ledger remains 244 covered, 0 partial, 0 deferred; the five new tests supplement it.

The Slack pin is `d7c7de9264c63015be398001d7a1094e7695a6db`. The source checker confirmed all 21 Slack Rails files match the pin and no Slack drift exists on origin/main. HTTP producer `runs_http` retains only the approved 2e20b24c layout/people-CSS/profile-card-controller inputs. Other producers use the pinned image.

Commands executed this turn:

```sh
git clone --single-branch --branch rust/ws16-slack-import https://github.com/Smart-Data-Ohio/smartfire.git .scratch/ws16-order-final
git -C .scratch/ws16-order-final fetch origin main:refs/remotes/origin/main
PARITY_NAMESPACE=ws16-order-final PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 CAMPFIRE_REFERENCE="$PWD/.scratch/ws16-order-final/rust/parity/.ci/reference" .scratch/ws16-order-final/rust/parity/bin/seed build default first_run
python3 .scratch/ws16-order-verify-seeds.py
python3 .scratch/ws16-order-oracles.py
python3 .scratch/ws16-order-final/rust/reference-tools/slack/check_reference.py
python3 .scratch/ws16-order-final/rust/reference-tools/slack/write_test_inventory.py
PARITY_NAMESPACE=ws16-order-final PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 CAMPFIRE_REFERENCE="$PWD/.scratch/ws16-order-final/rust/parity/.ci/reference" .scratch/ws16-order-final/rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze "$PWD/.scratch/ws16-order-final/rust/reference-tools/slack/ordering_vectors.rb"
git -C .scratch/ws16-order-final diff --exit-code -- rust/vectors/slack rust/crates/campfire/data/slack-ruby-downcase.json
```

```text
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
Slack setup views: 11 complete Rails template bodies generated with shared deterministic CSRF inputs
Slack connection HTTP oracle: 37 real Rails callback/disconnect/setup/remove cases generated
Slack OAuth vectors: 41 real Rails exchange/revoke/team cases, 4 authorization URLs, manifest and signed state generated
Slack markdown vectors: 481 cases generated from Rails
Slack mapper vectors: 9 fixture users; preview, import and repeat recorded
Slack run views: 83 complete Rails template bodies generated with shared deterministic CSRF inputs
Slack run HTTP oracle: 119 real Rails action cases with signed sessions and verified CSRF generated
Slack options oracle: 33 Ruby coercion and ISO time-bound cases generated
Slack client vectors: 32 Rails error and success mappings generated
Slack undo huddle oracle: real Rails User.destroy callbacks; all rows and fields in six affected tables generated
Slack payload edges: 12 client, 23 converter, 23 mapper, 12 conversation, 11 transport, 11 HTTP cases generated
Slack real-save rendering: 4 persisted body and mention goldens generated
Slack review comparison oracle: 11 room cases; 7 email cases; 3 handle cases; Unicode group name and 4 Ruby downcase values recorded
Slack Ruby downcase table: 1433 mappings recorded from the pinned Ruby runtime
Slack ordering oracle: 11-member tied-name group; mapped, missing-author, mention, alias and finishing orders recorded
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
Slack fresh-clone oracle regeneration: 27 producers; all vector bytes unchanged
Slack Rails reference: 21 source files match d7c7de9264c63015be398001d7a1094e7695a6db; no Slack drift on origin/main
Slack Rails test inventory: 244 covered, 0 partial, 0 deferred; 244 total
Slack ordering oracle: 11-member tied-name group; mapped, missing-author, mention, alias and finishing orders recorded
```

The Rust gates ran inside image `sha256:80bed826ce3b998e8ba75b85b25d18055066760982413e4483dd9779c5f053b2`, network none, offline locked Cargo, CI=1, two Cargo jobs and two test threads. Containers use the configured host rustc slot pool/count via the same flock loop; its count and host configuration were unchanged. The fresh clone's target is the sole extra Cargo target. The established vendored html5ever test-package exclusion remains unchanged; strict clippy includes all workspace targets. No seed skip occurred. Production release inputs include crates and explicit asset contexts only; the new vectors are test inputs, and all production includes remain inside crates.

The container helper used for the commands below:

```sh
#!/bin/bash
set -euo pipefail
root=$(git rev-parse --show-toplevel)
name="$1"; shift
docker run --rm --name "$name" --network none --cpus 4 --user 1000:1000 \
  -v "$root/.scratch/ws16-order-final:/src" \
  -v "$root/.scratch/rustc-wrapper.sh:/rustc-wrapper:ro" \
  -v /tmp/rust-port-rustc-slots:/rustc-slots \
  -v /home/riels/.cache/rust-port/rustc-slots:/slot-count:ro \
  -v /home/riels/.cargo/registry:/src/rust/.cargo-home/registry:ro \
  -w /src/rust --env CARGO_HOME=/src/rust/.cargo-home \
  --env RUSTC_WRAPPER=/rustc-wrapper --env TMPDIR=/src/tmp-review \
  --env CI=1 --env CARGO_BUILD_JOBS=2 \
  --env CARGO_PROFILE_TEST_DEBUG=0 --env CARGO_PROFILE_DEV_DEBUG=0 \
  --env RUST_TEST_THREADS=2 --env CABLE_TEST_PORT_RANGE=53300-53399 \
  --env INTEGRATION_TEST_PORT_RANGE=53300-53399 \
  sha256:80bed826ce3b998e8ba75b85b25d18055066760982413e4483dd9779c5f053b2 "$@"
```

```sh
.scratch/pinned-order.sh ws16-order-metadata cargo metadata --offline --locked --format-version 1
.scratch/pinned-order.sh ws16-order-suite cargo test --offline --locked --workspace --exclude html5ever --no-fail-fast -- --test-threads=2
python3 .scratch/summarize-suite.py .scratch/order-suite.log
.scratch/pinned-order.sh ws16-order-repeat python3 /src/tmp-review/order-repeat.py
.scratch/pinned-order.sh ws16-order-clippy cargo clippy --offline --locked --workspace --all-targets -- -D warnings
.scratch/pinned-order.sh ws16-order-release bash ci/with-release-inputs.sh cargo build --offline --locked --bin campfire
```

Raw summaries:

```text
cargo metadata --offline --locked: 13 workspace members; success
Pinned workspace totals: 58 summary blocks; 3762 passed; 0 failed; 12 ignored
Raw libtest summaries:
test result: ok. 1890 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 1301.49s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.68s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.75s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.43s
test result: ok. 1170 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 293.98s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.60s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.09s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.37s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.24s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 7.00s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.63s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 22.82s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.67s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.39s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.90s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.13s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s
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
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.57s
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
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1892 filtered out; finished in 0.75s
Slack tied-name determinism: 50 consecutive test invocations passed; 2500 repetitions; 5000 group resolutions; 0 order mismatches
    Finished `dev` profile [unoptimized] target(s) in 1m 16s
    Finished `dev` profile [unoptimized] target(s) in 1m 38s
```

The process-level repeat ran the exact group regression 50 consecutive times. Each invocation constructs new mappings and performs 50 repetitions for both direct and SQL-loaded users: 2,500 repetitions and 5,000 real group resolutions in total, all with zero mismatches. The normal full-suite run separately includes all five ordering regressions. Twelve explicit ignores in the raw workspace output are not represented as passing tests.

Cleanup:

```sh
.scratch/pinned-order.sh ws16-order-clean cargo clean --offline --locked
python3 .scratch/check-order-cleanup.py
```
```text
     Removed 19678 files, 11.7GiB total
WS16 cleanup: 0 scratch Cargo targets; 0 ordering containers; 0 listeners in 53300-53399; primary rust/target preserved
```

## Remaining and cross-workstream scope

No WS16-owned unblocked work remains. Only the owner-blocked combined real first-Google-sign-in → Slack opt-in/placeholder-claim HTTP comparison remains, pending WS14g's callback path on main. Existing Slack claim models and the personal opt-in/Slack OAuth/import/undo HTTP flow remain covered by the regenerated producers and workspace tests; seeded Google identity/account retention is not presented as a real Google callback interaction. No Google-owned implementation or Python model-server state was changed.

The prior Unicode room/email, empty admin 404 and timestamp/id ordering fixes remain intact and passed in the full suite and regenerated Rails output. The five new regressions did not change the original 244-declaration coverage ledger. No owner-blocked dependency was silently marked complete.
