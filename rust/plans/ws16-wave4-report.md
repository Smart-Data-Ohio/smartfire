# WS16 PR #184 analyzed-query follow-up — complete; Google interaction owner-blocked

Verified executable SHA: `a987114bec4d7c02119aeea63527082c7b5d5b62`. Branch: `rust/ws16-slack-import`. The final reply carries the report-only pushed SHA. This follow-up fixes Astra's remaining P3 on 8657d57c and merges main `0681adc6894ba93d8e279f1590f9856165bdbfcd` with a merge commit. No WS16-owned unblocked work remains on this merged main. The exact remaining owner dependency is WS14g's real first-Google-sign-in start/callback path for the combined Slack opt-in/placeholder-claim HTTP comparison.

## Changes

- `rust/crates/campfire/src/integrations/slack/users.rs`: replace the JSON subquery with the exact SQL shape executed by pinned Rails' `UserMapper#records_for(...).pluck(:slack_key, :record_id)`. Keep `IndexSet` compaction/deduplication and preserve SQLite's returned order in `IndexMap`. Do not add an ORDER BY or force an index.
- Pinned Rails has prepared statements enabled. Its SQLite adapter's `bind_params_length` is **999**; `AbstractAdapter#to_sql_and_binds` recompiles the entire relation without bindings when that limit is exceeded. Thus 997 distinct IDs use 999 bindings (workspace, kind, IDs), while 998 or more IDs use safely quoted literals for all fields. A singleton uses equality. SQLite string quoting doubles apostrophes and leaves backslashes unchanged. Bound and literal SQL bytes and values are checked against real `sql.active_record` notifications, rather than assuming relation `to_sql` describes executed bindings.
- `rust/reference-tools/slack/query_order_vectors.rb` and `rust/vectors/slack/query_order.json`: pinned Rails expectations for 1600 real mappings whose rowid order differs from insertion and index order. Twenty cases cover fresh/analyzed databases; singleton, unsorted, duplicate/blank IDs; 997/998 boundaries; 1200 and 1600 mappings; 41600 distinct IDs; apostrophe, backslash, newline, Unicode and SQL-looking strings. The producer records executed SQL fingerprints, actual bindings, query-plan details, ordered keys and group names. Long missing-ID lists are reconstructed from recorded counts, avoiding duplicated large fixtures.
- `rust/crates/campfire/src/integrations/slack/users/tests.rs`: compare all 20 executed SQL fingerprints, bindings, plans and actual ordered mapping results. Run ANALYZE after seeding. Resolve fresh/analyzed 1600-member groups using the captured ordered mappings only after query checks, so membership writes cannot contaminate the analyzed dataset. Both expected group names are recorded by Rails: fresh `ALICE, Alice, alice +1597`; analyzed `Alice, alice, ALICE +1597`.
- Merge conflict `rust/crates/db/src/tests.rs`: retain Slack's two modules and main's activity module, together with every other registration from both parents. Conflict `rust/crates/db/src/models/message.rs`: retain main's Rails filename normalization in the existing formatted attachment fallback; preserve all quiet Slack-import message behavior. Locked metadata succeeds after the merge without a lockfile repair.

No Rails application source, parity masks/allowlists, or test exclusions changed. No new hash iteration supplies output order. No real Slack calls or literal authorization headers were introduced. Compiler slots remain unchanged; Cargo uses two jobs and this run uses two test threads. The Python model server was not touched.

## Failing-first regression

Before changing production SQL, the new regression ran against the JSON-subquery implementation. It collected six mapping-order mismatches and the analyzed group-name mismatch, including all three reviewer cases and both binding thresholds. SQL/binding/plan checks and stricter dataset isolation were then added to the passing final regression.

```sh
CARGO_BUILD_JOBS=2 CABLE_TEST_PORT_RANGE=53300-53399 INTEGRATION_TEST_PORT_RANGE=53300-53399 mise exec rust@1.98.1 -- cargo test --offline --locked --manifest-path rust/Cargo.toml -p campfire slack_query_order_matches_rails_before_and_after_analyze -- --test-threads=2 --nocapture
```
Raw initial failure:
```text
Slack query group "fresh_all_1600_real_mappings": Rails="ALICE, Alice, alice +1597"; Rust="ALICE, Alice, alice +1597"
Slack query group "analyzed_all_1600_real_mappings": Rails="Alice, alice, ALICE +1597"; Rust="ALICE, Alice, alice +1597"
Slack query-order parity: 20 query cases; 2 group names; 7 mismatches
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1893 filtered out; finished in 2.26s
```

The same focused command after the final fix:

```text
Slack query group "fresh_all_1600_real_mappings": Rails="ALICE, Alice, alice +1597"; Rust="ALICE, Alice, alice +1597"
Slack query group "analyzed_all_1600_real_mappings": Rails="Alice, alice, ALICE +1597"; Rust="Alice, alice, ALICE +1597"
Slack executed SQL parity: 20 SQL fingerprints, bindings and query plans matched
Slack query-order parity: 20 query cases; 2 group names; 0 mismatches
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1893 filtered out; finished in 1.36s
```

## Fresh-clone verification

The independent GitHub clone `.scratch/ws16-query-final` starts at the pushed merge SHA, with no object alternates, copied target artifacts or copied seeds. Both seeds were freshly built by pinned Rails and checked explicitly. The full workspace suite ran with CI=1, so missing seeds cannot silently skip tests. All 28 Rails producers regenerated byte-identical vectors, including the 119 real session/CSRF run HTTP responses, 13 import→undo→reimport sequences across 89 tables, prior comparison regressions and the new analyzed-query oracle. The original declaration inventory remains 244 covered, 0 partial, 0 deferred; review regressions supplement that inventory.

Slack reference: `d7c7de9264c63015be398001d7a1094e7695a6db`. All 21 Slack Rails files match this pin; no Slack drift exists on merged origin/main. Only `runs_http` uses the previously approved 2e20b24c layout/people CSS/profile-card-controller drift image. All other producers use the pinned image.

Commands executed from the assigned worktree:

```sh
git clone --single-branch --branch rust/ws16-slack-import https://github.com/Smart-Data-Ohio/smartfire.git .scratch/ws16-query-final
git -C .scratch/ws16-query-final fetch origin main:refs/remotes/origin/main
mkdir -p .scratch/ws16-query-final/rust/parity/.ci/reference
git -C .scratch/ws16-query-final archive d7c7de9264c63015be398001d7a1094e7695a6db | tar -x -C .scratch/ws16-query-final/rust/parity/.ci/reference
PARITY_NAMESPACE=ws16-query-final PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 CAMPFIRE_REFERENCE="$PWD/.scratch/ws16-query-final/rust/parity/.ci/reference" .scratch/ws16-query-final/rust/parity/bin/seed build default first_run
python3 .scratch/ws16-query-verify-seeds.py
python3 .scratch/ws16-query-final/rust/reference-tools/slack/check_reference.py
python3 .scratch/ws16-query-final/rust/reference-tools/slack/write_test_inventory.py
python3 .scratch/ws16-query-oracles.py
```

Raw seed/reference/oracle summaries:

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
Slack Rails reference: 21 source files match d7c7de9264c63015be398001d7a1094e7695a6db; no Slack drift on origin/main
Slack Rails test inventory: 244 covered, 0 partial, 0 deferred; 244 total
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
Slack query fresh_unsorted_mixed_case: 8 distinct ids; 10 binds; 8 matches; SEARCH slack_import_records USING INDEX index_slack_import_records_on_slack_identity (slack_workspace_id=? AND slack_kind=? AND slack_key=?)
Slack query fresh_duplicates_blanks: 4 distinct ids; 6 binds; 4 matches; SEARCH slack_import_records USING INDEX index_slack_import_records_on_slack_identity (slack_workspace_id=? AND slack_kind=? AND slack_key=?)
Slack query fresh_single: 1 distinct ids; 3 binds; 1 matches; SEARCH slack_import_records USING INDEX index_slack_import_records_on_slack_identity (slack_workspace_id=? AND slack_kind=? AND slack_key=?)
Slack query fresh_bound_limit_997: 997 distinct ids; 999 binds; 997 matches; SEARCH slack_import_records USING INDEX index_slack_import_records_on_slack_identity (slack_workspace_id=? AND slack_kind=? AND slack_key=?)
Slack query fresh_literal_limit_998: 998 distinct ids; 0 binds; 998 matches; SEARCH slack_import_records USING INDEX index_slack_import_records_on_slack_identity (slack_workspace_id=? AND slack_kind=? AND slack_key=?)
Slack query fresh_over_999_real_mappings: 1200 distinct ids; 0 binds; 1200 matches; SEARCH slack_import_records USING INDEX index_slack_import_records_on_slack_identity (slack_workspace_id=? AND slack_kind=? AND slack_key=?)
Slack query fresh_all_1600_real_mappings: 1600 distinct ids; 0 binds; 1600 matches; SEARCH slack_import_records USING INDEX index_slack_import_records_on_slack_identity (slack_workspace_id=? AND slack_kind=? AND slack_key=?)
Slack query fresh_over_32766_ids: 41600 distinct ids; 0 binds; 1600 matches; SEARCH slack_import_records USING INDEX index_slack_import_records_on_slack_identity (slack_workspace_id=? AND slack_kind=? AND slack_key=?)
Slack query fresh_bound_escaping: 6 distinct ids; 8 binds; 1 matches; SEARCH slack_import_records USING INDEX index_slack_import_records_on_slack_identity (slack_workspace_id=? AND slack_kind=? AND slack_key=?)
Slack query fresh_literal_escaping: 1205 distinct ids; 0 binds; 1200 matches; SEARCH slack_import_records USING INDEX index_slack_import_records_on_slack_identity (slack_workspace_id=? AND slack_kind=? AND slack_key=?)
Slack query analyzed_unsorted_mixed_case: 8 distinct ids; 10 binds; 8 matches; SEARCH slack_import_records USING INDEX index_slack_import_records_on_slack_identity (slack_workspace_id=? AND slack_kind=? AND slack_key=?)
Slack query analyzed_duplicates_blanks: 4 distinct ids; 6 binds; 4 matches; SEARCH slack_import_records USING INDEX index_slack_import_records_on_slack_identity (slack_workspace_id=? AND slack_kind=? AND slack_key=?)
Slack query analyzed_single: 1 distinct ids; 3 binds; 1 matches; SEARCH slack_import_records USING INDEX index_slack_import_records_on_slack_identity (slack_workspace_id=? AND slack_kind=? AND slack_key=?)
Slack query analyzed_bound_limit_997: 997 distinct ids; 999 binds; 997 matches; SCAN slack_import_records
Slack query analyzed_literal_limit_998: 998 distinct ids; 0 binds; 998 matches; SCAN slack_import_records
Slack query analyzed_over_999_real_mappings: 1200 distinct ids; 0 binds; 1200 matches; SCAN slack_import_records
Slack query analyzed_all_1600_real_mappings: 1600 distinct ids; 0 binds; 1600 matches; SCAN slack_import_records
Slack query analyzed_over_32766_ids: 41600 distinct ids; 0 binds; 1600 matches; SCAN slack_import_records
Slack query analyzed_bound_escaping: 6 distinct ids; 8 binds; 1 matches; SEARCH slack_import_records USING INDEX index_slack_import_records_on_slack_identity (slack_workspace_id=? AND slack_kind=? AND slack_key=?)
Slack query analyzed_literal_escaping: 1205 distinct ids; 0 binds; 1200 matches; SCAN slack_import_records
Slack query ordering oracle: 20 executed SQL cases; 1600 mapped users; before/after ANALYZE; SQLite 3.53.2
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
Slack fresh-clone oracle regeneration: 28 producers; all vector bytes unchanged
```

Oracle replay driver (the referenced seed validator invokes `reference-tools/campfire/verify_parity_seed.rb` for each seed):

```python
from pathlib import Path
import os
import subprocess

root = Path.cwd() / '.scratch/ws16-query-final'
rust = root / 'rust'
env = dict(os.environ, PARITY_NAMESPACE='ws16-query-final', PARITY_OWNER='ws16',
           PARITY_IMAGE='ws16-reference:d7c7de92',
           CAMPFIRE_REFERENCE=str(rust / 'parity/.ci/reference'))
producers = [(name, []) for name in ['setup_views', 'connections_http', 'oauth_vectors',
    'markdown_vectors', 'mapper_vectors', 'run_views', 'runs_http', 'options_vectors',
    'client_vectors', 'undo_huddle', 'payload_vectors', 'rendering_vectors', 'review_regressions', 'ordering_vectors', 'query_order_vectors']]
producers += [('sequence_vectors', ['workspace']), ('sequence_vectors', ['personal'])]
producers += [('sequence_vectors', ['workspace', name]) for name in ['saved_reply',
    'poll_reply', 'pending_quoted_reply', 'sent_reply', 'foreign_thread',
    'room_event_schedule', 'claimed_session', 'claimed_google_account',
    'claimed_google_identity', 'claimed_password', 'placeholder_authorship']]
for name, args in producers:
    current_env = dict(env)
    if name == 'runs_http':
        current_env['PARITY_IMAGE'] = 'ws16-reference:d7c7de92-layout-2e20b24c'
    subprocess.run([str(rust / 'parity/bin/reference'), 'runner', '--seed', 'default' if name == 'mapper_vectors' else 'first_run',
        '--time', '2026-03-02T16:00:00Z', '--freeze',
        str(rust / f'reference-tools/slack/{name}.rb'), *args], env=current_env, check=True)
subprocess.run(['git', '-C', str(root), 'diff', '--exit-code', '--', 'rust/vectors/slack', 'rust/crates/campfire/data/slack-ruby-downcase.json'], check=True)
print('Slack fresh-clone oracle regeneration: 28 producers; all vector bytes unchanged')
```

Pinned Rust/media image and unchanged host compiler-slot loop:

```sh
#!/bin/bash
set -euo pipefail
root=$(git rev-parse --show-toplevel)
name="$1"; shift
docker run --rm --name "$name" --network none --cpus 4 --user 1000:1000 \
  -v "$root/.scratch/ws16-query-final:/src" \
  -v "$root/.scratch/rustc-wrapper.sh:/rustc-wrapper:ro" \
  -v /tmp/rust-port-rustc-slots:/rustc-slots \
  -v /home/riels/.cache/rust-port/rustc-slots:/slot-count:ro \
  -v /home/riels/.cargo/registry:/src/rust/.cargo-home/registry:ro \
  -w /src/rust --env CARGO_HOME=/src/rust/.cargo-home \
  --env RUSTC_WRAPPER=/rustc-wrapper --env TMPDIR=/src/tmp-review \
  --env CI=1 --env CARGO_BUILD_JOBS=2 \
  --env CARGO_PROFILE_TEST_DEBUG=0 --env CARGO_PROFILE_DEV_DEBUG=0 \
  --env RUST_TEST_THREADS=2 --env CABLE_TEST_PORT_RANGE=53300-53399 \
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

Runtime versions:

```text
rustc 1.98.1 (48a229cea 2026-09-01)
libvips 8.16.1
ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers
```

Compiler/test gates ran sequentially after Rails finished writing all vectors (metadata only reads manifests):

```sh
.scratch/pinned-query.sh ws16-query-metadata cargo metadata --offline --locked --format-version 1
.scratch/pinned-query.sh ws16-query-suite cargo test --offline --locked --workspace --exclude html5ever --no-fail-fast -- --test-threads=2
python3 .scratch/summarize-suite.py .scratch/query-suite.log
.scratch/pinned-query.sh ws16-query-clippy cargo clippy --offline --locked --workspace --all-targets -- -D warnings
.scratch/pinned-query.sh ws16-query-release bash ci/with-release-inputs.sh cargo build --offline --locked --workspace --bins
```

Raw summaries (all workspace test/doc-test summaries and explicit ignores):

```text
cargo metadata --offline --locked: 13 workspace members; success
Pinned workspace totals: 58 summary blocks; 3882 passed; 0 failed; 12 ignored
Raw libtest summaries:
test result: ok. 1981 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 1341.11s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.56s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.41s
test result: ok. 1194 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 218.05s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.54s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.13s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.20s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.84s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.23s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.51s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.53s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.15s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.77s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.60s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 79 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.61s
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
    Finished `dev` profile [unoptimized] target(s) in 59.37s
    Finished `dev` profile [unoptimized] target(s) in 1m 07s
```

The existing vendored html5ever test-package exclusion is the standard workspace command; strict clippy includes all workspace targets. Explicit ignored tests are listed above and are not counted as passed. There were zero missing-seed skips. The release-input build uses only crates plus the explicit asset context, with no vectors/parity/reference tools available to production code.

Cleanup commands:

```sh
.scratch/pinned-query.sh ws16-query-clean cargo clean --offline --locked
python3 .scratch/check-query-cleanup.py
```

```text
     Removed 19005 files, 9.6GiB total
WS16 cleanup: 0 scratch Cargo targets; 0 query-check containers; 0 listeners in 53300-53399; primary rust/target preserved
```

## Remaining

This review fix, merge and requested verification are complete. No WS16-owned unblocked work remains on merged main. Only the combined real first-Google-sign-in → Slack opt-in/placeholder-claim HTTP comparison remains owner-blocked on WS14g's start/callback path reaching main. The merged session controller still identifies that seam explicitly. Existing personal Slack opt-in/OAuth/import/undo HTTP coverage and seeded Google-identity retention are not represented as proof of a real Google callback interaction. No pixel work is deferred.
