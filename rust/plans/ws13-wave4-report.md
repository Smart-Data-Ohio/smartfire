# WS13 continuation report — partial

Verified and pushed source: `dbccd062f8ecf39335595f5615b340f7d87025cf` on `rust/ws13-huddles`. Main snapshot `65ad0d39` was merged with merge commit `006f93e7`; WS13b PR #172 was **not merged**. A documentation-only report commit follows this verified source.

Both Designers goldens and the approved #163 application-layout reconciliation are complete. All **38 complete room pages** match Rails byte for byte, including both Designers pages. All **226 controller/integration declarations** remain complete. **31/106 original system declarations** now pass in real browser interactions: Stage 11/15, audio 8/8, roster 8/8 and voice 4/12. The gateway's own Node suite passes 16/16 against Rust.

Fresh-clone workspace verification passed **2413 tests, zero failures, 13 ignored** across 50 raw summaries; that total is derived, not an invented Cargo aggregate line. Seeded app: 1034/0/4. Database: 689/0/4. The gateway and browser launchers are explicitly rerun rather than silently left ignored. Final all-target clippy passed with `-D warnings`. Every verification command below was executed in this continuation. The fresh-clone target was removed afterwards.

The Rails oracle remains `d7c7de92`, with only the approved #163 sidebar/application-layout overlay from `2e20b24c`. Its tracked sources and isolated oracle image have SHA256 checks. No root Rails expectations were adjusted, and no screenshot/pixel phase remains.

## Pushed slices

| Commit | Completed slice |
| --- | --- |
| `006f93e7` | Merge main's GitHub #167, attachments #171, held-listener deflake #173 and WS17 #170. Preserve both agent/huddle grant revocation and native room/sidebar composition. |
| `13fcb68f` | Both Designers pages, native quote cards, private cross-room placeholders and warm-parent dependency invalidation. |
| `8af04d6e` | Eleven complete ordinary Stage browser declarations. |
| `6a2dbf68` | Eight complete audio-processing browser declarations. |
| `0b3441bd` | Quote jumps follow their parent render origin, including background/default contexts. |
| `5f9bb494` | Restore main's test-module cfg guard lost during the room-controller merge. |
| `0c5828ad` | Keep the duplicated OOO facts adapter in its integration-test scope; preserve WS17 replays and the production shell adapter. |
| `7f03b849` | Eight complete roster declarations and browser proxy/controller lifecycle fixes. |
| `62691627` | Keep the full agent recovery assertions while auditing enqueues before the isolated worker can acknowledge them. |
| `d7c66249` | Four complete voice declarations using public room CRUD, real sessions and CSRF. |
| `4b9598e5` | Match voice's inherited Rails driver preset and refresh stale inventory prose. |
| `dbccd062` | Keep regression probes at eight test threads. |

## Changes by file and design

- `crates/campfire/src/boot.rs`, `controllers/presenters/test_support.rs`, `controllers/rooms.rs`, `controllers/presenters/view_context.rs` and `crates/db/src/models/membership.rs`: reconcile main's clients, workers, preferences and agent revocation with the huddle config, domain callbacks and room composition. Test helpers support all clients plus an optional huddle config. Derived sound fields reset before main's WS17 preference facts are composed. Both revocation paths survive the merge.
- `crates/campfire/src/controllers/presenters/message_links.rs`, its recorded `quote_child_vectors.json`, `presenters.rs`, `crates/views/src/message_links.rs`, `messages.rs`, `lib.rs`, `templates/messages/_message.html` and `templates/messages/message_links/_card.html`: native same-room child rendering, neutral direct-room labels, complete ERB whitespace, source edit/name cache dependencies, and parent-origin binding. Cross-room cached parents carry lazy frames and no source body/name facts. Public message components and domain signatures stay unchanged; the quote stamp combines with main's GitHub stamp.
- `reference-tools/huddle_quote_children.rb`, `huddle_sidebar_reference.py`, the tracked private oracle Docker context and approved `application.html.erb`, `ws13_verify_reference.py`, `ws13_verify_corpora.py`, `huddle_discrimination.py` and the affected full-room/page corpora: real Rails captures, the two additional Designers acceptances, #163's exact layout action, pin verification and three executable regressions. All 29 complete corpora regenerate byte-identically from tracked inputs.
- `crates/campfire/src/controllers/rooms/system_browser_tests.rs`, `parity/system/ws13`, `ws13-support.mjs`, `ws13-stage.test.mjs`, `ws13-audio.cases.mjs`, `ws13-roster.cases.mjs`, `ws13-voice.cases.mjs`: production Rust router/assets, signed sessions, actual SQLite state, public forms and live Cable in a pinned browser. Fixture/state routes exist only in the ignored test launcher. Stage keeps its 1400×1000 driver; audio/roster/voice use their original 1400×1400 presets. Original SDK/track/analyser stubs and every original assertion are retained. Voice setup preserves creator David and members David/Jason through the public CRUD endpoint; the 404 probe uses a real inaccessible room.
- `crates/campfire/src/jobs/tests.rs`, `controllers/internal_huddle_declaration_tests.rs`, `integrations/agent_jobs.rs`: audit actual enqueues while consumers may acknowledge rows, or stop the isolated fixture worker before a queue-specific audit. No queue assertion, rejected-enqueue trigger or production worker behavior is removed.
- `crates/db/src/models/status_settings.rs`, `rust/plans/ws13-deferred-tests.md`, `ws13_verify_declarations.py`: preserve the test-only OOO facts replay and the complete 548-title catalogue with per-file counts. No new model/job/service Rails declarations were ported; WS13b owns them.

The full merge initially exposed stale seed assumptions, derived preference composition, callback queue races and the approved layout drift. Both actual parity seeds were rebuilt from tracked scripts. The merge also lost a cfg(test) guard, which final production clippy caught and which is now restored. The duplicate OOO member-facts helper is test-scoped because production already uses the equivalent uncached shell; all main WS17 integration assertions still run.

One fresh workspace run caught `ws11_recovery_continues_after_one_durable_enqueue_failure` reading zero queued rows after its real worker had acknowledged the expected row. The fix stops that independent worker before creating candidates and keeps the rejected candidate, event timestamps and exact queued-row assertion. It passes in the final eight-thread full suite. This was explained and fixed here; it was not labeled an inherited failure.

The first extended browser run timed out on DOM-ready after twelve contexts, cascading into previously passing Stage cases. Closing each context's forwarding proxy with it removed the cascade. A later setup run caught a null Stimulus controller even though Cable had connected; the fixture now waits explicitly for that controller within the existing two-second selector budget. No concurrency or timing threshold was lowered/widened. Navigation keeps the driver's separate default budget; all original explicit selector/broadcast/connection waits remain. Stage's original synthetic connected-event declaration holds unavailable signaling pending so the isolated proxy cannot route external TLS to Rust HTTP. Selenium's whitespace-only inline-stack display assertion uses CSS display/visibility rather than Playwright's nonzero-box predicate. Final fresh browser results: 31 passed, zero failed/skipped.

## Exact remaining WS13 work

**75 original system declarations remain**, individually named and marked in `rust/plans/ws13-deferred-tests.md`:

| Rails file | Complete | Remaining |
| --- | ---: | ---: |
| `test/system/huddles_test.rb` | 0/31 | 31 real LiveKit |
| `test/system/huddle_join_notices_test.rb` | 0/16 | 16 ordinary browser |
| `test/system/huddle_invitations_test.rb` | 0/10 | 10 ordinary browser |
| `test/system/voice_channels_test.rb` | 4/12 | 8 ordinary browser |
| `test/system/huddle_presence_test.rb` | 0/6 | 6 ordinary browser |
| `test/system/stage_test.rb` | 11/15 | 4 real LiveKit |
| `test/system/huddle_roster_test.rb` | 8/8 | 0 |
| `test/system/huddle_audio_test.rb` | 8/8 | 0 |

The remaining 40 ordinary cases need their backend grant/sighting/broadcast/reconnect fixture adapter and original interactions. The 35 real LiveKit cases assert browser WebRTC tracks, audio/video, refreshed-token reconnects or server enforcement; recorded administrative Twirp/client responses cannot substitute for those assertions. All 35 titles and their reasons stay inventoried for real-server E2E implementation. No screenshot/pixel items remain.

The 548-title catalogue is **294 complete / 254 open across 33 files**. It retains WS13b's historical 37 complete / 179 open domain declarations until the lead reconciles its own report. Those 216 domain titles are WS13b-owned, not new WS13 remaining implementation work.

## Cross-workstream reconciliation

WS13b and WS17 seams remain:

```rust
enqueue_huddle_push(tx: &mut Tx<'_>, request: &PushRequest); // void
prepare_push(tx: &mut Tx<'_>, request: &PushRequest, policy_allowed: bool)
    -> Result<Option<PushDelivery>>;
publish_ring(tx: &mut Tx<'_>, request: &RingRequest, sound_allowed: bool)
    -> Result<()>;
resolve_overdue(tx: &mut Tx<'_>, user_id: Option<i64>) -> Result<()>;
```

Main's WS17 consumer handles `Notifications::HuddlePushJob`. No second policy/transport path or duplicate throttle claim was added. `Notifications::HuddleRingJob` still has no registered handler in this merged snapshot; its policy/banner delivery integration remains at the lead-owned seam and invitation E2E remains open.

The quote bridge touches the shared WS8b-m2 presenter/view. Cross-room quote endpoint authorization/delivery and source-edit quote callback fan-out remain WS8b-m2-owned and are not claimed complete here. No domain signatures changed for WS13b. No questions/approval are pending.

## Fresh-clone verification commands

Clone: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws13/.scratch/fresh-ws13-20261001`. It was created from tracked branch data with no hardlinks and without copying untracked source fixtures. Seeds and all oracle inputs were regenerated there. The workspace run began at `d7c66249`; its clean fast-forwards changed only the voice driver preset, inventory prose and regression-probe thread limit. The compiled Rust/Rails/vector/seed/dependency inputs were verified unchanged. Browser acceptance, regression probes, restoration rebuild and final clippy verify `dbccd062f8ecf39335595f5615b340f7d87025cf`. Only report documentation follows it.

The pinned Rails image supplies libvips 8.16.1 and ffmpeg 7.1.5 for workspace binaries, so CI's media-byte gates execute with their actual producing versions. Host versions differ; CI was not unset and no media assertions were skipped/masked. Builds used two jobs and the inherited machine-wide rustc wrapper; test/probe commands retain eight threads. One extra target existed and was then deleted.

All commands below ran with this equivalent environment (from the fresh clone):

```sh
export CI=1 CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
export TMPDIR="$PWD/.scratch" CAMPFIRE_REFERENCE="$PWD" CARGO_TARGET_DIR="$PWD/rust/target"
export CABLE_TEST_PORT_RANGE=52300-52349 MAIL_TEST_PORT_RANGE=52350-52399
export PARITY_NAMESPACE=ws13 PARITY_OWNER=ws13 PARITY_CPUS=2 PARITY_IMAGE=ws13-reference:d7c7de92
export NPM_CONFIG_CACHE="$PWD/.scratch/npm-cache"
rust/parity/bin/seed build default first_run
(cd rust && mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 >/dev/null)
python3 rust/reference-tools/huddle_sidebar_reference.py
python3 rust/reference-tools/ws13_verify_reference.py
python3 rust/reference-tools/ws13_verify_declarations.py
python3 rust/reference-tools/ws13_verify_corpora.py
mise exec rust@1.98.1 -- cargo --config 'target.x86_64-unknown-linux-gnu.runner=["docker", "run", "--rm", "--name", "ws13-pinned-workspace-test", "--label", "com.smartfire.rust-parity.owner=ws13", "--cpus", "2", "--network", "none", "--user", "1000:1000", "--volume", "/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws13/.scratch/fresh-ws13-20261001:/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws13/.scratch/fresh-ws13-20261001", "--workdir", "/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws13/.scratch/fresh-ws13-20261001", "--env", "CI=1", "--env", "TMPDIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws13/.scratch/fresh-ws13-20261001/.scratch", "--env", "CABLE_TEST_PORT_RANGE=52300-52349", "--env", "MAIL_TEST_PORT_RANGE=52350-52399", "--entrypoint", "/usr/bin/env", "ws13-reference:d7c7de92"]' test --locked --manifest-path rust/Cargo.toml --workspace --no-fail-fast -- --test-threads=8
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire huddle_gateway_own_node_suite_against_rust_endpoints -- --ignored --nocapture --test-threads=8
rust/parity/system/ws13
python3 rust/reference-tools/huddle_discrimination.py cross-room-quote-cache-leaks-private-source quote-source-cache-dependencies-bypassed quote-renderer-origin-binding-bypassed
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire controllers::presenters::message_links::tests -- --nocapture --test-threads=8
mise exec rust@1.98.1 -- cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
git diff --exit-code
(cd ../.. && mise exec rust@1.98.1 -- cargo clean --manifest-path .scratch/fresh-ws13-20261001/rust/Cargo.toml --target-dir .scratch/fresh-ws13-20261001/rust/target)
```

Locked metadata exits zero with stdout discarded as requested. Strict TOML parsing rejects duplicate dependency keys and reports 77 unique keys. Reference/declaration checks and metadata were repeated on the final clean source. Actual raw markers/summaries:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
Locked cargo metadata: ok
Workspace dependency keys: 77 unique; duplicates: []
Sidebar reference: tracked #163 source SHA256 verified; isolated image built
Reference identity: 94 files match d7c7de92
Post-#163 sidebar source: tracked SHA256 matches 2e20b24c
Post-#163 application layout: tracked source and oracle image SHA256 match 2e20b24c
Rails declaration catalogue: 548 titles retained; 294 passed; 254 partial/deferred; 33 files; source titles match
WS13 corpora: 29 complete files regenerated byte-identically
    Finished `dev` profile [unoptimized] target(s) in 24.53s
```

Workspace raw summaries, in execution order:

```text
test result: ok. 1034 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 804.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.83s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.89s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 689 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 106.30s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.86s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.09s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.01s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.35s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 32.68s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.34s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.25s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.36s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.00s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.71s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.66s
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
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Raw ignored entries (13 in the workspace; gateway/browser are separately executed below):

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test controllers::internal_huddle_tests::huddle_gateway_own_node_suite_against_rust_endpoints ... ignored, requires Node and the gateway pinned ws package; run explicitly with --ignored
test controllers::rooms::system_browser_tests::huddle_system_cases_in_real_browser ... ignored, requires Docker and the pinned Playwright image; run parity/system/ws13
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

Gateway's own Node suite and its Rust launcher:

```text
ℹ tests 16
ℹ suites 0
ℹ pass 16
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 15750.231875
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1037 filtered out; finished in 16.29s
```

Real-browser Stage/audio/roster/voice suite and its Rust launcher:

```text
ℹ tests 31
ℹ suites 0
ℹ pass 31
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 47923.589333
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1037 filtered out; finished in 51.37s
```

The new warm-cache/origin tests failed before their fixes. Compiled deliberate regressions caught privacy leakage, missing source dependencies and bypassed origin binding; compilation failure does not count as detection. Raw failing-first probes and restored baseline:

```text
quote-renderer-origin-binding-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1037 filtered out; finished in 1.15s
quote-source-cache-dependencies-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1037 filtered out; finished in 1.64s
cross-room-quote-cache-leaks-private-source: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1037 filtered out; finished in 1.22s
WS13 discrimination: 3 compiled regressions detected; sources restored
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1035 filtered out; finished in 1.50s
```

Source restoration checks exit zero with no diff. Cleanup/check markers:

```text
     Removed 6393 files, 2.6GiB total
Post-workspace fast-forwards: only voice driver preset, inventory prose, regression-probe thread limit
Compiled Rust, Rails, vector, seed and dependency inputs unchanged: ok
Fresh-clone target removed: ok
```

Logs remain under the owning worktree's `.scratch/fresh-logs/` (including earlier failures under `.scratch/fresh-queue-race-logs/`); the report includes their raw summary lines above. There are no WS13 test containers left running. The source worktree contains only allowed scratch output after committing this report.
