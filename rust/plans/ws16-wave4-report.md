# WS16 PR #184 review fixes — complete; Google claim interaction owner-blocked

This update closes the three Astra findings and both requested audits. All WS16-owned work is complete. The exact remaining dependency is WS14g's real first-Google-sign-in start/callback path for the combined placeholder-claim HTTP comparison. PR #184 is already open; that owner dependency remains explicit.

Verified executable SHA: `30f256d9de3336fedc5947504b6fb5c6ba95326c`. The final reply supplies the report-only pushed SHA. Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws16`; branch: `rust/ws16-slack-import`.

Main was merged before writing the fixes: #176 at `7442031dfefe4454a6fc67743a74de322e978ad4`, merge commit `961b933a2a58ba67c14af6f936632a955ec8d0ce`. Conflict resolution preserves Slack's importing suppression alongside WS11's callback phases, streaming validation and thread deletion callback order. `d5155cb6` restores main's normal-create reference phases while retaining the import-specific path. The review fixes are `508222d40945af642c5c95f11007a3299d290006`. Before the full gates, main's #183 queue-observation changes at `59ad94de3d0c53dfc15cae95901453f6506a7e83` were merged with `30f256d9`. No stash or rebase.

## Fixes and audit

- **P2 — conversation auto-merge:** query the database with Rails' actual predicate, `deleted_at IS NULL AND type=? AND LOWER(name)=? ORDER BY id LIMIT 1`. Only the incoming value uses Ruby downcase. SQLite's ASCII folding remains on the stored name. `équipe` into `Équipe` creates room 862; `Équipe` into lowercase `équipe` merges room 861; even `Équipe` into stored `Équipe` creates a separate room, exactly as Rails does. Preview and real import both compare the recorded action, room id/name, created mapping and membership count.
- **Comparison audit:** Slack user matching already uses SQL `LOWER(email_address)`, which is retained. The audit found Rust string lowering's contextual final sigma differs from Ruby: `ΟΣ` becomes `οσ` in Ruby and `ος` in Rust. Fix every Ruby downcase site in WS16's room query value, email query/index lookup, large-group naming and domain handling. `crates/campfire/data/slack-ruby-downcase.json` contains all 1,433 non-identity scalar mappings recorded from pinned Ruby. The helper preserves expansions and Ruby's Unicode version without changing SQLite's behavior. Seven email preview/import cases include asymmetric Latin/Greek matching and Rails' unchanged duplicate-email failure. Three bot-handle observations prove exact case-sensitive keys and repeat reuse; Slack ids, bot ids, mention-map keys and explicit targets remain exact comparisons. Group sorting still uses Ruby Unicode downcase, rather than SQLite ASCII folding. UTC `Z` parsing is an ASCII comparison and is unrelated to names/handles.
- **P3 — nonexistent admin run:** both controllers now halt with `head :not_found`. All seven admin lookup actions (show, status, plan, start_import, catch_up, cancel, undo) match Rails' status 404 and zero response bytes with real signed sessions and CSRF. Personal scoping behavior remains covered.
- **P3 — equal-time ordering:** admin and personal run indexes use `created_at DESC, id DESC`. The audit also fixes setup's newest active-run query. The three complete HTTP bodies record ids 853/854 with equal creation times, selecting 854 first. Other SQL ordering already matches Rails: queued claims use creation/id ascending; stalled sweeps, workspace singleton selection, issue pages and undo batches use id ascending; later overlapping runs use started_at/id descending; finishing uses message creation/id descending; room-target lists retain Rails' `ORDER BY LOWER(name)` with no invented id tie-breaker. Conversation discovery sorts exact Slack ids, and large-group names retain Ruby downcase ordering.

`reference-tools/slack/review_regressions.rb` and the extended `runs_http.rb` execute Rails itself. No Rust output constructs the oracle. The new case vectors supplement the original 244-declaration ledger; the declaration count is not inflated. Full run HTTP coverage is now 119 complete response bodies, plus the established 83 run template bodies and 11 setup bodies. Request signing/CSRF is real; render-only CSRF/CSP entropy is fixed before rendering. Random encrypted Set-Cookie wire values are not a byte-identical claim; status, redirects, decoded session/flash, persisted run fields, audits and durable job arguments are compared separately. No mask, exclusion or allowlist was changed. No real Slack call or literal auth header was introduced.

## Failing-first evidence

These observations ran before the fix against executable source `d5155cb6`. The comparison test exposed all three casing differences; exact bot handles already passed. Seven missing admin actions and three timestamp ties were independently selected from the actual HTTP fixture. Every selected HTTP regression returned a failing libtest result before the controller/query changes.

```text
assertion `left == right` failed: "équipe" into "Équipe" preview
  left: Object {"action": String("merge"), "room_id": Number(861)}
 right: Object {"action": String("create"), "room_id": Null}
  left: String("ΟΣ, οςA, οτ +8")
 right: String("οςA, ΟΣ, οτ +8")
assertion `left == right` failed: "ΟΣ@example.com" preview
  left: Object {"matched": Number(0), "placeholders": Number(1), "deactivated": Number(0), "bots": Number(0), "total": Number(1)}
 right: Object {"matched": Number(1), "placeholders": Number(0), "deactivated": Number(0), "bots": Number(0), "total": Number(1)}
test result: FAILED. 1 passed; 3 failed; 0 ignored; 0 measured; 1884 filtered out; finished in 1.73s
```

```text
"admin-missing-show" complete HTTP body differs at byte 0
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1887 filtered out; finished in 1.52s
```

```text
"admin-missing-status" complete HTTP body differs at byte 0
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1887 filtered out; finished in 1.68s
```

```text
"admin-missing-plan" complete HTTP body differs at byte 0
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1887 filtered out; finished in 1.90s
```

```text
"admin-missing-start_import" complete HTTP body differs at byte 0
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1887 filtered out; finished in 1.76s
```

```text
"admin-missing-catch_up" complete HTTP body differs at byte 0
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1887 filtered out; finished in 1.71s
```

```text
"admin-missing-cancel" complete HTTP body differs at byte 0
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1887 filtered out; finished in 1.56s
```

```text
"admin-missing-undo" complete HTTP body differs at byte 0
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1887 filtered out; finished in 1.74s
```

```text
"admin-index-equal-created-at" complete HTTP body differs at byte 31727
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1887 filtered out; finished in 1.75s
```

```text
"personal-index-equal-created-at" complete HTTP body differs at byte 33531
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1887 filtered out; finished in 2.22s
```

```text
"setup-active-equal-created-at" complete HTTP body differs at byte 36474
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1887 filtered out; finished in 2.30s
```

Focused comparison verification after the fix:

```sh
CARGO_BUILD_JOBS=2 mise exec rust@1.98.1 -- cargo test --offline --locked --manifest-path rust/Cargo.toml -p campfire slack_review_ -- --test-threads=8 --nocapture
```
```text
Slack review email parity: 7 Rails preview/import cases matched
Slack review room parity: 11 Rails preview/import cases matched
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 1884 filtered out; finished in 3.92s
```

## Rails reference and fresh-clone verification

Reference pin: `d7c7de9264c63015be398001d7a1094e7695a6db`. The 21 Slack Rails files are unchanged on merged main, so the later-importer drift contingency does not apply. Complete HTTP pages use the three approved `2e20b24c` layout, people CSS and profile-card-controller inputs in `ws16-reference:d7c7de92-layout-2e20b24c`; all other producers use `ws16-reference:d7c7de92`.

The independent GitHub clone at `.scratch/ws16-review-final` has no object alternates. It was updated to executable SHA `30f256d9` before compiling, with no existing target or seed copied in. Both fresh seeds were built by pinned Rails and explicitly checked. All 26 producers replayed there, including 13 import/undo/reimport sequences across all 89 tables and the new review producer; a Git diff verified every golden and the production Ruby mapping table remained byte-identical. Seed/oracle/reference commands and their raw summaries:

```sh
PARITY_NAMESPACE=ws16-review-final PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 CAMPFIRE_REFERENCE="$PWD/.scratch/ws16-review-final/rust/parity/.ci/reference" .scratch/ws16-review-final/rust/parity/bin/seed build default first_run
python3 .scratch/ws16-review-verify-seeds.py
python3 .scratch/ws16-review-oracles.py
python3 .scratch/ws16-review-final/rust/reference-tools/slack/check_reference.py
python3 .scratch/ws16-review-final/rust/reference-tools/slack/write_test_inventory.py
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
Slack fresh-clone oracle regeneration: 26 producers; all vector bytes unchanged
Slack Rails reference: 21 source files match d7c7de9264c63015be398001d7a1094e7695a6db; no Slack drift on origin/main
Slack Rails test inventory: 244 covered, 0 partial, 0 deferred; 244 total
```

The full Rust checks used pinned image `sha256:80bed826ce3b998e8ba75b85b25d18055066760982413e4483dd9779c5f053b2` (Rust 1.98.1, libvips 8.16.1, ffmpeg 7.1.5), network `none`, offline locked Cargo, eight test threads and two Cargo jobs. The container wrapper mounts the configured host rustc slot pool/count and uses the same flock slot loop; no throttle setting was changed. The only extra Cargo target was the fresh clone's target. The release-input guard builds from crates and its explicit asset contexts; the production downcase include is inside crates. The workspace excludes only the established vendored html5ever test package. No seed skips were detected; the twelve explicit ignores are listed below.

```sh
.scratch/pinned-review.sh ws16-review-metadata cargo metadata --offline --locked --no-deps --format-version 1
.scratch/pinned-review.sh ws16-review-suite cargo test --offline --locked --workspace --exclude html5ever --no-fail-fast -- --test-threads=8
python3 .scratch/summarize-suite.py .scratch/review-suite.log
.scratch/pinned-review.sh ws16-review-clippy cargo clippy --offline --locked --workspace --all-targets -- -D warnings
.scratch/pinned-review.sh ws16-review-release bash ci/with-release-inputs.sh cargo build --offline --locked --bin campfire
```
```text
cargo metadata --offline --locked: 13 workspace members; success
Pinned workspace totals: 58 summary blocks; 3757 passed; 0 failed; 12 ignored
Raw libtest summaries:
test result: ok. 1885 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 625.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.92s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 1170 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 86.43s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.46s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.53s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.17s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.84s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.55s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.21s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.88s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.78s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.43s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.50s
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

Strict clippy:
    Finished `dev` profile [unoptimized] target(s) in 1m 05s
Release-input build:
    Finished `dev` profile [unoptimized] target(s) in 1m 18s
```

## Cleanup and exact remaining scope

After all gates passed, Cargo removed the sole scratch target. Any removal after Cargo cleanup was limited to an empty target directory. The normal worktree target and source/oracle evidence are preserved. No listener/container remains, no Python model-server operation was performed, and no real Slack endpoint was called.

```sh
.scratch/pinned-review.sh ws16-review-clean cargo clean --offline --locked
```
```text
     Removed 19158 files, 9.4GiB total
Scratch Cargo targets remaining: 0
Original worktree rust/target preserved: yes
WS16 verification containers running: 0
Listeners in WS16 port range: 0
```

**No WS16-owned item remains. Only owner-blocked work remains:** the combined first-Google-sign-in placeholder-claim interaction waits for WS14g's actual `/session/google` start/callback handlers. `controllers/sessions.rs:120` still names that seam. Slack opt-in/OAuth/import/undo interactions, placeholder eligibility and retained Google-account/identity rows are covered; none is represented as proof of the missing first-sign-in callback. The 244 original declarations remain behavior-covered, with zero partial/deferred declarations in the ledger; this does not claim the original Ruby test classes executed against Rust. Pixel comparisons are outside the phase and are not remaining work.

The tracked report is `rust/plans/ws16-wave4-report.md`, mirrored byte for byte to the requested external `wave4/ws16-report.md`. The report-only commit is pushed and remote synchronization is checked before the final reply; executable source stays at verified SHA `30f256d9`.
