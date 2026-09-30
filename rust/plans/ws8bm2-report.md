# WS8bm2 review corrections and continuation

**PARTIAL.** All three Astra P2 findings are fixed and pushed. Five deterministic regressions failed first against the exact reviewed `bcbac9f85b182329fd54aa332a07fec609ab6255`. Saved status updates preserve concurrent claims/schedules and Rails' loaded-record JSON response; compact dates preserve clocks and offsets; scheduled sends reach the subscribed unread stream. Additional slices provide all eight ordered human-edit replacement targets, correct composer identifiers for five room types, and broader compact-calendar parity.

The source verified in the independently prepared remote clone is `219f860cfccc1d20c0a39c293abb3f0007e537e6`. The final report commit only changes this report. The final workspace run passed **1723 tests, 0 failed, 11 existing ignores**; clippy exited 0. All **24/24** Rails fixtures replayed byte identically. No timing bounds or test concurrency were changed. WS17 physical tagged sending, WS11 agent credentials/invocation and WS13 huddle execution remain explicitly flagged seams. All 45 browser cases remain reserved for the end-to-end Rust-server phase.

## Changes by file and pushed slice

- `62f6e3e2`: merge commit incorporating `eaba80d5` (main, PR #168). Locked Cargo metadata exited 0 immediately after the merge; no lockfile or dependency changes.
- `a27fecd9`, completed by `219f860c`: `db/src/models/saved_item.rs` writes only dirty columns. A status write preserves a concurrent dispatch claim or reminder reschedule; a reminder-only write preserves concurrent status. `db/src/tests/saved_item_test.rs` and `campfire/src/controllers/message_features/saved_tests.rs` verify these paths. The instance retains unassigned loaded attributes, matching Rails `update!` and the actual controller payload helper. `reference-tools/messaging/review_saved_race.rb` and its committed vector capture both persisted rows and serialized stale responses.
- `5bb541d7`: `db/src/slash_commands/calendar.rs` preserves compact clocks, T-separated clocks and trailing numeric zones/Z; `time_parser.rs` handles short and seconds-bearing offsets and delegates its calendar fallback to the builder parser. Relative leading/trailing slash grammar retains Rails' intentional ISO matching/truncation behavior. `message_features/date_tests.rs` compares 88 builder and 88 slash results across UTC, New York, Berlin and Apia, including DST gaps/folds; 88 reminder and 88 scheduled HTTP requests persist the expected timestamps, and 88 dispatch pairs prove pending just before and sent exactly at due time.
- `0d27d7a0`: `db/src/broadcasts.rs::unread_rooms_stream_name` is shared by `campfire/src/channels/unread_rooms.rs`, scheduled dispatch and slash dispatch. `message_features/scheduled_tests.rs` subscribes an actual WebSocket to `UnreadRoomsChannel` and checks its exact identifier and roomId payload. The reference-source checker now includes the Rails unread channel.
- `4cb12141`: `controllers/messages.rs` replaces presentation, metadata, GitHub, X, quote, Fizzy, LinkedIn and generic embed containers in Rails order, including empty containers. `views/src/messages.rs::MetaPartial` is a pure partial adapter. `message_features/provider_tests.rs` compares **40/40 complete socket frames** from five actual Rails HTTP edits, including populated public/private cards and an edited empty message. Populated cases submit unchanged text so this proves replacement composition without claiming provider reference/write callback parity. The existing quote-test socket helper is shared.
- `0dde49ae`: `views/src/messages/composer.rs::Facts.room_param_key` accepts concrete STI identity; `controllers/rooms.rs` supplies it through the ordinary HTTP presenter. Five complete Open/Closed/Voice/Stage/Board composers match Rails, and five ordinary room requests preserve both input id and label target. Original room/direct/thread/Drive composer goldens still pass. Thread-shell mounting and execution remain with their owners.
- `77b8b715`: another **96** actual Rails cases cover packed 10/12-digit clocks, short/ordinal defaults, T/space-separated clocks, fractions, ignored compact named zones, nil results and exceptions in four zones. Total sampled builder coverage is **359** (69 + 106 + 88 + 96). This is sampled grammar parity, not universal Ruby Date parity.
- `8e01b5d0`: updates the existing hub test to consume and verify metadata and all six card-container frames in order before testing boosts. The first fresh-clone run exposed the old two-frame expectation. This was a deterministic assertion failure caused by the changed edit sequence, not a timing flake.

Crate-local paths above refer to `campfire/src/`, `db/src/` or `views/src/` under `rust/crates/`; Ruby tools and vectors are under `rust/`. All runtime test inputs are committed or built independently from the pinned Rails seed. The domain changes have no HTML dependency. Provider adapters remain read-only, and no physical push or owner credential/transport implementation was added.

## Failing-first evidence at bcbac9f8

A scratch archive of the reviewed revision was constructed under this worktree, with only the five new regression tests and their Rails date input fixture added. The saved controller/model, scheduled model and old time parser were checked byte-for-byte against `git show bcbac9f8:<path>` before execution. Both runtime seeds were built for that archive. No production fix was present.

```bash
git archive bcbac9f8 | tar -x -C .scratch/review-baseline
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 .scratch/review-baseline/rust/parity/bin/seed build default first_run > .scratch/review-fixes/baseline-seed.log 2>&1
TMPDIR="$PWD/.scratch/review-baseline/.scratch/tmp" CARGO_TARGET_DIR="$PWD/rust/target" CARGO_INCREMENTAL=0 CI=1 CABLE_TEST_PORT_RANGE=52500-52549 MAIL_TEST_PORT_RANGE=52550-52599 mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/review-baseline/rust/Cargo.toml --locked -j4 -p campfire review_regression -- --test-threads=4 --nocapture > .scratch/review-fixes/baseline-regressions.log 2>&1
```
The completed baseline command exited 101 at the intended assertions (an earlier test-authoring compile error was corrected before this run). Raw baseline evidence:

```text
REVIEW_EARLY_HTTP {"id":3,"room_id":486777696,"thread_id":null,"markdown_source":"Reviewer date probe","send_at":"2026-03-05T00:00:00.000Z","sent_at":null,"dropped_at":null}
REVIEW_RACE after HTTP patch: status=done reminded_at=None
REVIEW_EARLY_DISPATCH sent_at_00_01=true expected_due_14_30=true
REVIEW_RACE after HTTP patch: status=done reminded_at=None
REVIEW_RACE repeated_dispatch=true durable_push_jobs=2
REVIEW_SCHEDULED_UNREAD frame=Err(Elapsed(()))
test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 604 filtered out; finished in 3.13s
```

1. **Saved claim race:** the dispatch transaction claims the row, signals readiness, and blocks on a channel. The HTTP PATCH performs its stale read and queues its writer. The test waits for `queued_writes() > 0`, then releases the dispatch transaction. No sleeps establish the interleaving. At the reviewed revision, a second dispatch is true and there are two durable push jobs. After the fix, the persisted claim is `2026-03-02 16:02:00`, the second dispatch is false and there is one job. An independent reschedule interleaving retains 18:00 in the database. Rails' stale JSON still returns its loaded 16:01 reminder/nil claim; the new oracle exercises the real private controller payload helper and Rust HTTP tests check these fields.
2. **Compact dates:** the reviewed parser stores midnight for `20260305 14:30`, and a real dispatch sends at 00:01. The fix stores 14:30 and stays pending at 00:01. The date matrix catches trailing -0500, short ±05 and seconds-bearing numeric offsets, plus compact clocks and DST edge behavior. The separate early-send regression checks actual dispatch again at 14:30.
3. **Unread stream:** the reviewed scheduled-send test receives `Err(Elapsed(()))` from an actual subscribed socket. The fixed socket receives `{"roomId":486777696}` under the `UnreadRoomsChannel` identifier. Its one-second test bound is unchanged.

Raw final-source observations (the reschedule-only case has no fired claim):

```text
WS8bm2 STI composer HTTP: 5/5 room types preserve Rails reply-control ids and labels
WS8bm2 broader dates: 202/202 Rails coercion/compact-width cases match
REVIEW_EARLY_HTTP {"id":3,"room_id":486777696,"thread_id":null,"markdown_source":"Reviewer date probe","send_at":"2026-03-05T14:30:00.000Z","sent_at":null,"dropped_at":null}
REVIEW_EARLY_DISPATCH sent_at_00_01=false expected_due_14_30=true
WS8bm2 date HTTP: 88 saved timestamps, 88 scheduled timestamps and 88 before/due dispatch pairs match Rails
WS8bm2 provider edits: 5 HTTP edits, 40/40 real socket replacement frames byte-identical to Rails
REVIEW_RACE after HTTP patch: status=done reminded_at=None
REVIEW_RACE after HTTP patch: status=done reminded_at=Some(2026-03-02 16:02:00)
REVIEW_RACE repeated_dispatch=false durable_push_jobs=1
REVIEW_SCHEDULED_UNREAD frame=Ok("{\"identifier\":\"{\\\"channel\\\":\\\"UnreadRoomsChannel\\\"}\",\"message\":{\"roomId\":486777696}}")
```

A production-literal scan also found the slash dispatcher's literal correct name; both dispatchers now use the common helper. The only production literal is the helper itself; the independent subscription test retains a literal protocol expectation.

```bash
rg -n 'user_.*(unread_rooms|unreads)' rust/crates --glob '*.rs'
```
```text
rust/crates/db/src/broadcasts.rs:19:    format!("user_{user_id}_unreads")
rust/crates/campfire/src/channels/broadcasts.rs:250:    /// `broadcast_unread_room`: `{ roomId: }` to each member's `user_<id>_unreads`, leaving out
rust/crates/campfire/src/channels/tests/reference_test.rs:292:        &format!("user_{}_unreads", id("jz")),
rust/crates/campfire/src/controllers/message_features/scheduled_tests.rs:772:        "scheduled sends never reach the subscribed user_*_unreads stream"
```

## Fresh-clone validation

The remote branch was cloned under `.scratch/review-final/repo`, without copying worktree seed data, vectors, secrets or Cargo output. Both seeds were rebuilt there. The clone was fast-forwarded from the remote after the socket-test correction and the additional saved-response parity correction; its final source SHA is recorded above. All tracked source remained clean. Existing build output in that independently prepared clone was reused for subsequent runs, and all results below belong to its final source. Scratch contains generated output and runtime seed copies, not hidden test inputs.

The first full run at `77b8b715` failed only the outdated hub assertion:

```text
test result: FAILED. 635 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 110.66s
```
The focused corrected hub check passed, and an intermediate full run at `8e01b5d0` passed. After the real Rails stale-response differential prompted a production correction, the full suite was rerun again at the final source. No failure was treated as inherited, ignored or hidden through timing changes.

```bash
git clone --single-branch --branch rust/ws8bm2-message-features https://github.com/Smart-Data-Ohio/smartfire.git .scratch/review-final/repo
cd .scratch/review-final/repo/rust
mkdir -p ../.scratch/tmp
CARGO_BUILD_JOBS=2 PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 parity/bin/seed build default first_run > ../.scratch/final-seed.log 2>&1
cd ..
git pull --ff-only > .scratch/final-pull.log 2>&1
git rev-parse HEAD > .scratch/final-source-sha.txt
CARGO_BUILD_JOBS=2 mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path rust/Cargo.toml > .scratch/final-metadata.json
CARGO_BUILD_JOBS=2 TMPDIR="$PWD/.scratch/tmp" CARGO_TARGET_DIR="$PWD/rust/target" CARGO_INCREMENTAL=0 CI=1 CABLE_TEST_PORT_RANGE=52500-52549 MAIL_TEST_PORT_RANGE=52550-52599 mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -j4 --workspace --exclude html5ever -- --test-threads=4 --nocapture > .scratch/final-workspace-test.log 2>&1
CARGO_BUILD_JOBS=2 TMPDIR="$PWD/.scratch/tmp" CARGO_TARGET_DIR="$PWD/rust/target" CARGO_INCREMENTAL=0 mise exec rust@1.98.1 -- cargo clippy --manifest-path rust/Cargo.toml --locked -j4 --workspace --exclude html5ever --all-targets -- -D warnings > .scratch/final-clippy.log 2>&1
```
Seed build, locked metadata, final full suite and clippy exited 0. Metadata produced JSON and has no native summary line. Raw seed, test and clippy summaries:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```
```text
test result: ok. 636 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 109.33s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.59s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.41s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 447 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 99.47s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.29s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.99s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.99s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.30s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.74s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.31s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.08s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.82s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.99s
test result: ok. 40 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.44s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.28s
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
```
```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 59.64s
```

The eleven existing ignores cover the reference recorders, WS11 bot management, push-latency measurement, database export/scenario/Rails/rollback checks, mail rollback export and two kit doctests. ACME validation remains conditional on PEBBLE_MINICA. Version-dependent media byte checks report the libvips/ffmpeg mismatch against recorded versions. The intentional missing-seed guard test reports its empty temporary directory; both real required seeds were present, rebuilt and validated. No owned seeded case skipped. The standard command explicitly excludes vendored html5ever. Expected panic tests log panics and pass; they are not suite failures. No timing flake was observed in the final run.

Independent Rails checks run from the same clone:

```bash
python3 rust/reference-tools/messaging/verify_oracles.py > .scratch/final-oracle-replay.log 2>&1
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb default > .scratch/final-validate-default.log 2>&1
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb first_run > .scratch/final-validate-first-run.log 2>&1
PARITY_IMAGE=ws8bm2-reference:d7c7de92 python3 rust/reference-tools/messaging/features-reference-check.py > .scratch/final-reference-check.log 2>&1
```
```text
WS8bm2 Rails oracle: 8 poll reads/ballots; 6 poll creates; 4 pin writes; 11 partials; 10 zone/date probes
WS8bm2 oracle replay: features.json byte-identical
WS8bm2 saved Rails oracle: 11 HTTP responses; 6 item partials; 1 empty page
WS8bm2 oracle replay: saved.json byte-identical
WS8bm2 scheduled Rails oracle: 12 HTTP responses; 8 row partials; 1 empty page; 2 composer controls
WS8bm2 oracle replay: scheduled.json byte-identical
WS8bm2 search Rails oracle: 15 parsed queries; 15 chip partials; 1 empty page; 1 clear stream; 18 zone/date selections; 1 populated sections partial; 1 load-older control; 1 empty older stream; 4 HTTP responses; 771 Unicode word ranges
WS8bm2 oracle replay: search.json byte-identical
WS8bm2 preload Rails oracle: 6 complete message fragments; 9 committed fixture tables
WS8bm2 oracle replay: preloads.json byte-identical
WS8bm2 slash Rails oracle: 20 dispatch responses; 17 picker responses; 61 play presentation fragments; 3 format responses; 12 huddle readiness cases
WS8bm2 oracle replay: slash.json byte-identical
WS8bm2 links/files Rails oracle: 15 Files sections; 5 quote HTTP responses; 2 quote partials; 16 size values; 9 fixture tables
WS8bm2 oracle replay: links_files.json byte-identical
WS8bm2 reminder push Rails oracle: 27 policy cases; 2 captured real job payload/subscription handoffs; 3 fixture tables
WS8bm2 oracle replay: reminder_push.json byte-identical
WS8bm2 quote integration Rails oracle: 4 containers; 7 messages; 3 fixture tables
WS8bm2 oracle replay: quote_integration.json byte-identical
WS8bm2 root cache Rails oracle: 6 composite keys; 12 provider frame containers; 10 fixture tables
WS8bm2 oracle replay: root_cache.json byte-identical
WS8bm2 panels Rails oracle: 4 pin panels; 2 sidebar links
WS8bm2 oracle replay: panels.json byte-identical
WS8bm2 date Rails oracle: 69 Time.zone.parse cases in 3 zones
WS8bm2 oracle replay: date_inputs.json byte-identical
WS8bm2 Rails saved race: claim preserved=true; reschedule preserved=true
WS8bm2 oracle replay: review_saved_race.json byte-identical
WS8bm2 review date Rails oracle: 88 compact/offset/DST cases in 4 zones
WS8bm2 oracle replay: review_dates.json byte-identical
WS8bm2 compact width Rails oracle: 96 short/ordinal/fraction/compact-clock cases in 4 zones
WS8bm2 oracle replay: date_compact_widths.json byte-identical
WS8bm2 provider Rails oracle: 11 GitHub containers; 7 embed/LinkedIn containers; 8 fixture tables
WS8bm2 oracle replay: providers.json byte-identical
WS8bm2 provider Rails oracle: 11 GitHub containers; 7 embed/LinkedIn containers; 8 fixture tables
WS8bm2 provider edit Rails oracle: 5 HTTP edits; 40 exact replacement frames
WS8bm2 oracle replay: provider_edits.json byte-identical
WS8bm2 event cards Rails oracle: 3 populated containers; 5 fixture tables
WS8bm2 oracle replay: event_cards.json byte-identical
WS8bm2 broader date/coercion Rails oracle: 106 calendar cases; 15 parameter string/presence probes; 8 reminder HTTP responses
WS8bm2 oracle replay: date_coercions.json byte-identical
WS8bm2 composer Rails oracle: 4 complete Markdown composers including thread and Drive-share controls
WS8bm2 oracle replay: composer.json byte-identical
WS8bm2 STI composer Rails oracle: 5 complete Open/Closed/Voice/Stage/Board Markdown composers
WS8bm2 oracle replay: composer_sti.json byte-identical
WS8bm2 X preload Rails oracle: 3 populated containers; 4 persisted posts
WS8bm2 oracle replay: twitter_preloads.json byte-identical
WS8bm2 oracle replay: twitter_cards.json byte-identical
WS8bm2 oracle replay: twitter_text.json byte-identical
WS8bm2 oracle replay: 24/24 independently replayed fixtures byte-identical
```
```text
WS8bm2 reference source check: 106 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
```
```text
"passed": 29,
  "failed": 0
```
```text
"passed": 4,
  "failed": 0
```
The default seed has 29/29 Rails checks; first_run has 4/4. The consumed-file check concerns the pinned reference image. Main also carries the authorized post-pin status-popup/board-nudge changes; their controller/UI and physical push parity remain with WS8b-r2 and WS17/WS12, respectively. No source masks or golden allowlists changed here.

## Rails cases grouped by file

| Rails controller file under `test/controllers/` | Named behaviors covered / total | Remaining integration |
| --- | --- | --- |
| `rooms/polls_controller_test.rb` | 16 / 16 | Browser phase; additional performance/runtime proof below |
| `messages/pins_controller_test.rb` | 6 / 6 | Action menus/browser phase |
| `rooms/pins_controller_test.rb` | 3 / 3 | Browser phase; additional performance proof below |
| `saved_items_controller_test.rb` | 12 / 12 | WS17 physical send seam; browser phase |
| `scheduled_messages_controller_test.rb` | 19 / 19 | Composer mounted; thread-shell/browser phase |
| `searches_controller_test.rb` | 36 / 36 | Read-only provider composition mounted; provider endpoints/callbacks and populated whole-page proof |
| `rooms/slash_commands_controller_test.rb` | 9 / 10 | WS11 agent invocation seam |
| `autocompletable/icons_controller_test.rb` | 6 / 6 | Browser phase |
| `autocompletable/slash_commands_controller_test.rb` | 7 / 7 | Browser phase; WS11 invocation seam |
| `autocompletable/users_controller_test.rb` | 5 / 5 | Odd ActiveRecord lookup/pager shapes remain |
| `rooms/message_links_controller_test.rb` | 12 / 12 | Browser phase |
| `rooms/files_controller_test.rb` | 8 / 8 | WS8b-r Files tab/shell; odd-shape coercions; browser phase |
| **Total named controller behavior ports** | **139 / 140** | **One WS11 agent seam** |

| Rails system file under `test/system/` | Browser cases proved / total |
| --- | --- |
| `polls_test.rb` | 0 / 4 |
| `pins_saved_test.rb` | 0 / 7 |
| `slash_commands_test.rb` | 0 / 26 |
| `search_files_test.rb` | 0 / 4 |
| `scheduled_messages_test.rb` | 0 / 4 |
| **Total** | **0 / 45; reserved for end-to-end phase, none attempted** |

## Precise remaining work and ownership

1. **Last controller behavior:** agent invocation in `rooms/slash_commands_controller_test.rb` remains the sole 1/140 named behavior seam, owned by WS11. Read-only agent metadata/autocomplete is present; credential acceptance and execution are not implemented here.
2. **Provider callbacks/endpoints:** WS15g/WS15e/WS14e own network fetches, reference synchronization, provider-write broadcasts, private GitHub/Fizzy card endpoints and event attendance endpoints. The human-edit composition sequence is complete; changed URLs still need these owner callbacks. Prove populated complete search/older-window pages after those callbacks integrate. Audit other owners' deletion/unpin request-origin scopes.
3. **Shell/panel/Drive:** wire the owning thread shell, configured Drive-share availability/endpoints (WS14g), action menus, Files tab and full WS8b-r shell. Ordinary Markdown/schedule mounting and concrete room composer ids are complete. Legacy WS6 fixture fallbacks remain partial. Post-pin status-popup/sidebar changes belong to WS8b-r2.
4. **Date/coercion:** the two calendar paths now share the sampled fallback and 359 builder cases pass. Signed/long years, expanded ISO and additional unprobed compact/zone grammar, exact exceptional messages, odd ActiveRecord array/hash lookups, user filtering, structured pager/link parameters, option stripping and file-object coercions still need actual Rails differentials. No universal grammar or coercion parity is claimed.
5. **Additional non-browser proof:** constant-query poll/pin-list measurements and the periodic poll-close job's actual runtime socket delivery remain unverified. Existing vote/pin sockets and zero-query quote/provider checks do not prove those separate paths.
6. **Flagged owner seams:** WS17 physical tagged reminder send is `campfire/src/jobs/reminders.rs` and intentionally leaves the durable job unfinished until transport is installed; policy/payload/tag/subscriptions are retained. WS11 credential recognition is `concerns.rs::WS11_AGENT_AUTHENTICATION_SEAM`; command execution is flagged in the dispatcher. WS13 huddle launch is flagged in the dispatcher. No physical send, agent integration or huddle execution was added.
7. **End-to-end phase:** all 45 inventoried browser cases remain deferred against the Rust server. No browser harness, browser/pixel matrix, deployment or Rust Docker build was attempted.

No new product decision is required. Logs are retained under `.scratch/review-fixes/` and the independently prepared clone's `.scratch/`; its regenerable target directory is cleaned after verification per the shared resource rule. This external report and the tracked copy are byte-identical. The branch remains partial at the specific boundaries above.
