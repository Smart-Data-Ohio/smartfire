# WS16 — runnable Slack import and destructive undo (PARTIAL)

Branch: `rust/ws16-slack-import`. Verified source: `d0f77dd60b20050b576d9879d0a56f2249dafb60`. The final report commit follows that source; its pushed SHA is in the worker reply. This report supersedes the earlier foundations-only report.

The durable importer now runs: serial job handlers, paced/resumable page execution, users and conversation mapping, quiet history/reply writes, finishing, and destructive LIFO undo. Both workspace and personal fixture sequences match Rails after import, undo, and reimport. **WS16 acceptance remains partial: setup/OAuth, Slack HTTP controllers, views and system interaction are not implemented.** Users cannot yet connect or start these imports through the Rust UI. Remaining domain/error/fault scenarios are listed individually in `rust/plans/ws16-test-inventory.md`; their owner remains WS16 continuation.

No Rails source, schema, parity masks, comparison allowlists or pixel work changed. No PR or deployment was created. Work stayed in the assigned worktree, apart from this explicitly requested external report.

## Reference and pushed slices

Rails reference is `d7c7de9264c63015be398001d7a1094e7695a6db`. The source checker reads all 21 Slack model/job/controller files from the reference image and compares their bytes with the pin. `origin/main` at `20dc8ea3ce04818f3fc51236e7957b9a892cb88f` has no Slack drift from that pin. Thus the LIFO-per-conversation, kept-parent, pending scheduled reply, finish-every-room and later-importer naming reference is still the pin; no later Slack-file override was needed. Approved unrelated Rails drift comes through main unchanged.

The earlier foundation slices are `ed7878bb`, `49434c01`, and `54b289ca`. This continuation pushed each coherent slice:

| Commit | Delivered slice |
|---|---|
| `a26f5133` | Runner phase machine and lease/durable continuation protocol. |
| `4d12856d` | Claimable placeholder users and conversation policies. |
| `7486d72b` | Quiet transactional history/reply page writer and SQL runner store. |
| `d3bfd58c` | Real registered serial worker, periodic sweep and destructive resumable undo. |
| `f6988daf` | Actual Rails workspace/personal import → undo → reimport DB differentials and actual overlapping-import LIFO regression. |
| `30f478ec` | Merge main at `20dc8ea3`, preserving both event and Slack scheduler registrations/assertions. |
| `e709cb36` | Undo guard mutation proof and regenerated Rails sequence inputs. |
| `d0f77dd6` | Require the Slack sweep in WS8's second exact scheduler inventory assertion. |

The main merge is a merge commit, not a rebase. Locked Cargo metadata was rechecked successfully. No stash was used.

## Changes by file

Paths are relative to `rust/`; paths starting with `integrations/` abbreviate `crates/campfire/src/integrations/`. Foundation files remain part of the delivery; this table emphasizes the new executable slice.

| File | Change |
|---|---|
| `crates/campfire/src/integrations/slack/runner.rs`, `runner/tests.rs` | Users/discovery/messages/finishing phases, cursors and thread queues, default 25-second monotonic budget, API attempt counts, boundary cancellation, dry-run samples, saved-progress uniqueness recovery, and resumable page transitions. |
| `integrations/slack/jobs.rs`, `jobs/tests.rs` | Registered StepJob/UndoJob consumers on `slack_import`, atomic claim, lease capture/release, delayed Retry-After resume with heartbeat, cancellation/auth race behavior, transient-error propagation, and a real booted worker importing then undoing the fixture. |
| `integrations/slack/users.rs`, `users/tests.rs`; `crates/db/src/models/user.rs` | Owner Slack-ID mapping, case-insensitive email matching including inactive members, domain-eligible claimable human placeholders, deactivated guests/bots/unknown authors, no inappropriate open-room grants, stable mappings and page rollback. |
| `integrations/slack/conversations.rs`, `conversations/tests.rs` | Public workspace channel merge, private/personal isolation, alive Open/Closed explicit targets, archived naming/invisibility, DM/member-set reuse, self/Slackbot exclusions, and large-MPIM policy. Membership creation and mapping order match Rails. |
| `integrations/slack/writer.rs`, `writer/tests.rs`; `crates/db/src/models/message.rs` | Prepare a page before writing, quiet imported Message save with existing validation/render/search/reference domain APIs, source microseconds and edits, history/reply dedupe, channel threads/Direct replies, followers, quiet reactions/pins, truncation issues and final thread counts. Ordinary message callbacks remain on their existing path. |
| `integrations/slack/store.rs`, `store/tests.rs` | Status/token fencing immediately before domain writes, atomic page/progress commits, lease heartbeat renewal, resolution and coverage/bounds state, dry runs, cancellation fencing, completion/kick, and finishing the union of mapped and written rooms with membership/read-pointer rules. |
| `integrations/slack/undoer.rs`, `undoer/tests.rs` | Seven reverse-dependency phases with 200-record cursors, quiet leaves/message/thread deletion, kept root/parent/thread/room/mapping decisions, claimed placeholders, per-destroy savepoints and issues, terminal cleanup, and overlapping-import LIFO/name checks. |
| `crates/db/src/models/channel_thread.rs`, `message.rs` | Imported destruction variants keep dependency/index/reference cleanup while suppressing the ordinary parent stamp and broadcast paths, including dependent replies. |
| `crates/db/src/models/user/destruction.rs` | Dependent user destruction needed by undo: reuse existing message/thread/pin/boost/category/session/agent APIs, Slack association cleanup, avatar purge and Calendar remote-delete producer. The huddle callback seam remains partial below. |
| `crates/db/src/database.rs`, `models/room.rs` | Feature-gated test input providers for UUIDs and SQLite's membership clock, so the same nondeterministic inputs can be supplied to both runtimes. Production retains the existing random UUID/SQLite clock. No HTTP or environment switch controls these providers. |
| `integrations/slack/sequence_tests.rs` | Load Rails' actual initial rows, then run the real Rust runner/store/protocol/undoer against the same recorded pages; compare every field in all 89 table snapshots for workspace and personal scenarios. |
| `integrations/slack/client.rs`, `client/tests.rs` | Existing fixed-host, user-token read client is now consumed by the runner; numeric bound query formatting and reusable local TLS fixture transport support. Tier 2/3 pacing remains 18/min and 45/min, with four attempts and 1/2/4-second paced backoff. |
| `crates/campfire/src/app.rs`, `integrations/jobs.rs`, `jobs/periodic.rs` | Slack network injection, runtime consumer registration and the actual 30-second `SlackImport.sweep_stalled` callback. The pre-existing durable queue configuration has one Slack worker. |
| `integrations/action_claims/tests.rs`, `crates/campfire/src/jobs/tests.rs` | Preserve exact scheduler names/order/interval assertions and include Slack's 30-second sweep. Main's event task is retained. |
| `integrations/github/notifier/tests.rs`, `integrations/web_push/{tests.rs,ws17_delivery_tests.rs}` | Minimal AppState/Env fixture literal updates for the new field/defaults; no integration behavior rewritten. |
| `crates/campfire/src/rich_text.rs` | Expose the existing icon resolver to the writer; no new rendering implementation. |
| `reference-tools/slack/mapper_vectors.rb`, `sequence_vectors.rb`; `vectors/slack/{users,sequence,sequence_personal}.json` | Actual pinned Rails mapper rows and real Rails StepJob/UndoJob sequences, recorded Slack requests, and 18 time-bound observations. Fresh encryption IVs remain in the initial input rows and every compared snapshot. |
| `reference-tools/slack/check_{lease,runner,writer,undo}_mutations.py` | Runtime assertion discrimination for five foundation guards, three runner guards, the quiet writer and two destructive undo guards; source restored in finally blocks. |
| `reference-tools/slack/{check_reference,write_test_inventory}.py`; `plans/ws16-test-inventory.md` | Source-pin validation and all 244 named Rails Slack tests mapped conservatively to coverage, partial or deferred work. |

## Design and parity evidence

Domain code does not depend on HTML. HTTP runs outside the SQLite writer lock. Before committing a page, the store reloads status and the lease token; cancellation or takeover prevents page/cursor writes. Continuations are durable job inserts through EventSink persistence. Retry-After's heartbeat and delayed job insert share a transaction. The job releases its ownership token before publishing the continuation. Global SQLite run claims and ownership-token leases supplement the single durable worker, so another process or an overlapping sweep cannot double-run a fresh step.

Quiet page writes use the ordinary Message validation/render/index/reference machinery with the imported callback path. Each page commits atomically. There is no unread/activity/push/agent fan-out or per-message broadcast, no room touch during the page, and thread count/activity is refreshed when its reply pages finish. The writer regression rejects broadcasts and checks the durable queue contains only the original StepJob. Its deliberately ordinary-callback mutant fails at the thread counter assertion.

Undo uses the actual LIFO eligibility model and naming text. A second fixture import by a different actor produces zero duplicate messages; undoing the earlier import is refused with `A later import by Later importer also imported some of these conversations. It has to be undone first; ask them or an administrator.` Undoing the later run first retains the earlier data, then undoing the earlier run removes it. Kept-content tests cover foreign replies, pending thread/root scheduled replies, polls, saves, foreign pins, claimed placeholders, destroy failure/savepoint rollback and a batch over 200 mapping rows.

Both DB differentials execute the actual Rails jobs and the actual Rust runner/store/undoer. They start from fresh first_run schema seeds, use the same Slack fixture pages, a frozen clock, deterministic UUID inputs, and the actual Rails-created encrypted credential rows as initial inputs. Both use a test queue adapter for this row comparison; durable publication and real worker consumption are tested separately. Only pacing/HTTP transport/clock/UUID inputs are fixed. Every value is retained: IDs, timestamps, mapping ownership, read pointers, counts, richtext bytes, search rows, ciphertext and empty unrelated tables. JSON-typed columns compare as JSON values, and rows are sorted by primary key for presentation; there are no value masks. Framework schema bookkeeping and FTS shadow tables are excluded; `sqlite_sequence` and the actual `message_search_index` are included.

The comparison failed during development at membership grant order and then membership mapping order. Rails grants active users before Slack members, and its membership `pluck` uses the covering index's user order. Matching those actual source operations fixed the differences. Both final three-snapshot comparisons now pass. Regenerating the reference changes only random encrypted credential inputs, which are preserved and compared rather than normalized away.

## Remaining work — owner WS16 continuation

This is a coherent partial delivery of the runnable core, not complete Slack acceptance.

1. **All setup/OAuth HTTP behavior:** credential create/update/clear and encrypted reconnect updates, blank-secret retention, setup removal guards and audits; app manifest and callback URL; user-scope authorize/exchange/revoke/team-info flows; sudo, session/user-bound single-use state, tamper/replay/expiry/CSRF, team/scope/account conflict checks, return-path allowlist, disconnect rules and secret filtering. The read client's auth/team endpoints exist, but OAuth/controller integration does not.
2. **All Slack controllers/views/system interaction:** admin workspace/run list/create/show/status/plan/import/catch-up/cancel/undo, per-user connection/preview/run/status/selection, every authorization branch and exact HTML/JSON/frame/polling/issue-pagination response. Build the imports parity seed and byte-identical goldens. The one Rails system behavior case is deferred; pixel diffs are not remaining work.
3. **Ruby normalization/error semantics:** normalize_options/date parsing, Array/to_s/presence/string-key behavior, malformed client/converter/mapper payloads and Rational timestamp forms, exact transport timeout/reset/status-body and non-Slack exception class/message text. The normal Slack fixture payloads, 481 converter vectors, 12 client vectors and 18 boundary observations are proved; arbitrary malformed input parity is not.
4. **Remaining runner/mapper/writer/finishing regressions:** mentions outside channel, deleted mapped rooms/threads/parents, >10 MPIM targets/dry preview, personal target isolation, membership-setup crash rollback, full bounded history, missing scope and retry exhaustion job scenarios, failed-run resume, private exclusion, catch-up with new/late messages, multi-year/multiple-step history, bounded-test/full-import coverage invalidation, finishing earlier-created rooms and forward-only read pointers, identity-index seeks, finishing heartbeat cadence, completion/undo kick, and truncated reactions. The implementations contain these normal policies, but the original named scenarios remain partial/deferred until individually demonstrated.
5. **Remaining undo variants and model cases:** session/Google/remaining-authorship placeholder claims and retained reimport mappings; rooms with events/schedules; kept saved/poll replies and foreign-created threads; sent versus pending schedule variants; other model normalization/validation/lifecycle cases listed in the inventory. User destruction currently persists huddle revocation but does not implement huddle stream/presence callbacks or their validation seam; WS16 must integrate WS13's domain when available. Unusual dependent account cleanup callbacks also need differential coverage. This is not a claim of complete User#destroy parity.
6. **Client test completion:** explicit newest-first assertion, a recovering 5xx response sequence and successful-attempt callback accounting, plus exact Ruby network exception text. These remain named partial entries.

The ledger contains **95 covered behaviors, 10 partial, 139 deferred; 244 declarations**. Covered means mapped executable behavior, not that Rails' original test class ran against Rust. The exact deferred names and their ownership are in `rust/plans/ws16-test-inventory.md`.

| Rails test file | Covered | Partial | Deferred |
|---|---:|---:|---:|
| `controllers/accounts/slack_import_runs_controller_test.rb` | 0 | 0 | 33 |
| `controllers/accounts/slack_imports_controller_test.rb` | 0 | 0 | 10 |
| `controllers/slack/connections_controller_test.rb` | 0 | 0 | 6 |
| `controllers/slack/imports_controller_test.rb` | 0 | 0 | 20 |
| `controllers/slack/oauth_controller_test.rb` | 0 | 0 | 22 |
| `jobs/slack_import/run_lifecycle_test.rb` | 16 | 3 | 10 |
| `jobs/slack_import/workspace_import_test.rb` | 13 | 2 | 19 |
| `models/slack/client_test.rb` | 14 | 4 | 0 |
| `models/slack/markdown_converter_test.rb` | 26 | 0 | 4 |
| `models/slack_import_test.rb` | 26 | 1 | 14 |
| `system/slack_import_test.rb` | 0 | 0 | 1 |

## Cross-workstream boundaries and open questions

WS1's encryption is reused unchanged. WS3's existing durable queue remains the owner of publication/retry/worker behavior. WS8's Message/Thread APIs gained the narrow importing variants required by the Rails source; ordinary callbacks are retained and the broad suite checks them. The test-only UUID/SQLite-clock seam is in db Env. WS17 and GitHub fixture literals only received the new defaults/transport field. Existing Calendar remote-delete and AgentGrant APIs are reused. Main's event work was merged, not reimplemented by WS16. The huddle callback integration above remains a concrete WS13 boundary, owned here as WS16 continuation rather than silently reassigned.

No product decisions were reopened and no approval is requested. The host media mismatch is addressed by the requested pinned-image verification, rather than waived. Partial scope is explicit above.

## Final verification environment and outcomes

The final fresh clone is `.scratch/canonical`, independently cloned from GitHub and updated from this worker branch. It has no shared-object alternates. Its tested HEAD is `d0f77dd60b20050b576d9879d0a56f2249dafb60`; tracked source is clean. Default and first_run Rails seeds were freshly rebuilt there from the archived pin. CI=1 enforces seed availability. The full run is offline in its own network namespace and uses its own target, the host rustc slot loop, two Cargo jobs, eight libtest threads, and the reserved cable/integration port range. The `html5ever` exclusion is the full-test command documented in rust/AGENTS.md; clippy includes the entire workspace/all-targets.

The first canonical run completed all 50 summary blocks with 2,394 passed, one failed, and 11 existing explicit ignores. Its single failure was this worker's missing Slack task in WS8's exact scheduler inventory; it was corrected by adding the required ordered 30-second entry, not by filtering tasks or weakening equality. The focused scheduler test and a second complete workspace run passed. No failure was called inherited or waived. The host libvips/ffmpeg prerequisite is resolved in the canonical image: the strict media pipeline test passes.

The corrected full run exited 0 with **2,395 passed, 0 failed, 11 existing explicit ignores** across 50 raw summaries, including doctests. App: 992 passed/2 ignored. DB: 718 passed/4 ignored. All **73 Slack-owned tests** run (49 app + 24 db), with no owned ignores or seed skips; the credential export adds a separately passing repeat of one DB test. Clippy exited 0 for workspace/all-targets with -D warnings. The image has Rust 1.98.1, libvips 8.16.1 and ffmpeg 7.1.5; version checks are recorded below.

Commands below were actually rerun in this continuation. Unless stated otherwise, they start at the assigned worktree root. Logs are retained under `.scratch/`. The final report-only commit does not change tested Rust code or vectors.

### Docker and compiler wrappers

The following is the exact `.scratch/pinned-check.sh` used by the invocations below:

```sh
#!/bin/bash
set -euo pipefail
root=$(git rev-parse --show-toplevel)
name="$1"; shift
docker run --rm --name "$name" --network none --cpus 4 --user 1000:1000 \
  -v "$root/.scratch/canonical:/src" \
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
```

The mounted rustc wrapper is an unconditional copy of the host slot loop. It shares the host lock directory and slot-count file; no container bypasses that pool.

```sh
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

Version probes (vips is queried through the runtime library):

```sh
.scratch/pinned-check.sh ws16-version-rust rustc --version
.scratch/pinned-check.sh ws16-version-vips-ffi python3 -c 'import ctypes; lib=ctypes.CDLL("libvips.so.42"); lib.vips_version_string.restype=ctypes.c_char_p; print(lib.vips_version_string().decode())'
.scratch/pinned-check.sh ws16-version-ffmpeg ffmpeg -version
```

```text
rustc 1.98.1 (48a229cea 2026-09-01)
8.16.1
ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers
```
### Fresh-clone Rails seeds and actual Rails sequences

Seed command ran from `.scratch/canonical`:

```sh
PARITY_NAMESPACE=ws16-canonical PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" rust/parity/bin/seed build default first_run
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

The reference producers run actual Rails jobs in recorded, network-none runner containers. Both use a fresh first_run seed copy. Regenerated vectors retain actual ciphertext rather than forcing IVs.

```sh
PARITY_NAMESPACE=ws16-sequence PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/slack/sequence_vectors.rb
PARITY_NAMESPACE=ws16-sequence PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/slack/sequence_vectors.rb personal
python3 rust/reference-tools/slack/check_reference.py
python3 rust/reference-tools/slack/write_test_inventory.py
```

```text
Slack Rails sequence (workspace): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (personal): import -> undo -> reimport; 89 tables per snapshot; 18 recorded API requests
Slack Rails reference: 21 source files match d7c7de9264c63015be398001d7a1094e7695a6db; no Slack drift on origin/main
Slack Rails test inventory: 95 covered, 10 partial, 139 deferred; 244 total
```
### Security/correctness discrimination: deliberately failing implementations

These four scripts each exited 0 only after requiring genuine libtest runtime assertion failures. Source was restored before the passing canonical tests. All native builds use the host rustc wrapper, two Cargo jobs and at most eight libtest threads. Mutation failures are retained as failures, not included in the passing workspace totals.

```sh
TMPDIR="$PWD/.scratch/tmp" python3 rust/reference-tools/slack/check_lease_mutations.py
```

```text
cancelled lease ignored: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 721 filtered out; finished in 0.11s
fresh lease stolen: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 721 filtered out; finished in 0.24s
wrong token accepted: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 721 filtered out; finished in 0.14s
sweep duplicates queued job: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 721 filtered out; finished in 0.12s
plaintext credential write: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 721 filtered out; finished in 0.72s
Slack mutation guards: 5 broken implementations rejected; source restored
```

```sh
python3 rust/reference-tools/slack/check_runner_mutations.py
```

```text
Runner mutation rejected: lease_release -> slack_job_releases_before_durable_continuation_and_uses_serial_queue FAILED
Runner mutation rejected: retry_after -> slack_job_retry_after_commits_heartbeat_and_delayed_job_atomically FAILED
Runner mutation rejected: cancel_preservation -> slack_job_cancel_during_auth_error_preserves_cancel_and_disconnects FAILED
test result: FAILED. 2 passed; 7 failed; 0 ignored; 0 measured; 985 filtered out; finished in 32.97s
```

```sh
python3 rust/reference-tools/slack/check_writer_mutation.py
```

```text
Writer mutation rejected: ordinary message callbacks -> slack_writer_history_replies_pins_reactions_are_quiet_and_keep_microseconds FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 993 filtered out; finished in 1.07s
```

```sh
python3 rust/reference-tools/slack/check_undo_mutations.py
```

```text
Undo mutation rejected: kept_parent -> slack_undo_keeps_live_thread_parent_for_foreign_or_scheduled_replies FAILED
Undo mutation rejected: claimed_user -> slack_undo_keeps_claimed_placeholders_and_their_mapping FAILED
test result: FAILED. 5 passed; 2 failed; 0 ignored; 0 measured; 987 filtered out; finished in 6.37s
```
### Corrected fresh-clone owned tests and scheduler inventory

```sh
.scratch/pinned-check.sh ws16-canonical-slack cargo test --offline --locked -j 2 -p campfire --bin campfire integrations::slack -- --nocapture --test-threads=8
.scratch/pinned-check.sh ws16-canonical-db cargo test --offline --locked -j 2 -p campfire_db --lib slack_ -- --nocapture --test-threads=8
.scratch/pinned-check.sh ws16-canonical-scheduler cargo test --offline --locked -j 2 -p campfire --bin campfire ws8_periodic_tasks_match_rails_names_and_intervals -- --nocapture --test-threads=8
```

```text
Slack DB differential (workspace): import -> undo -> reimport; 89 tables x 3 snapshots; every row and field matched
Slack DB differential (personal): import -> undo -> reimport; 89 tables x 3 snapshots; every row and field matched
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 945 filtered out; finished in 6.84s
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 698 filtered out; finished in 2.47s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 993 filtered out; finished in 0.03s
```
### Reverse encrypted-row readback

The export directory `.scratch/canonical/.scratch/crypto-final/db` was created before this command. Export ran in the final fresh clone:

```sh
.scratch/pinned-check.sh ws16-canonical-crypto env SLACK_RUST_CRYPTO_OUTPUT=/src/.scratch/crypto-final/db/rust-slack-crypto.json cargo test --offline --locked -j 2 -p campfire_db --lib slack_credentials_create_encrypts_rows_and_exports_for_rails_readback -- --nocapture --test-threads=8
```

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 721 filtered out; finished in 1.13s
```

For readback, `.scratch/crypto-final/db/production.sqlite3` was reset to a copy of the clone's freshly built default seed, old WAL/SHM files removed, and the just-exported Rust envelope JSON copied to `.scratch/crypto-final/db/rust-slack-crypto.json`. Then:

```sh
PARITY_NAMESPACE=ws16-crypto-final PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" rust/parity/bin/reference runner --storage "$PWD/.scratch/crypto-final" rust/reference-tools/slack/verify_rust_crypto.rb
```

```text
Slack encryption readback: Rails decrypted and validated 2 Rust-written columns
```
### Full workspace and clippy

First complete run, before the scheduler expectation correction (exit 101):

```sh
docker run --rm --name ws16-canonical-suite --network none --cpus 4 --user 1000:1000 -v "$PWD/.scratch/canonical:/src" -v "$PWD/.scratch/rustc-wrapper.sh:/rustc-wrapper:ro" -v /tmp/rust-port-rustc-slots:/rustc-slots -v /home/riels/.cache/rust-port/rustc-slots:/slot-count:ro -v /home/riels/.cargo/registry:/src/rust/.cargo-home/registry:ro -w /src/rust --env CARGO_HOME=/src/rust/.cargo-home --env RUSTC_WRAPPER=/rustc-wrapper --env TMPDIR=/src/tmp-review --env CI=1 --env CARGO_BUILD_JOBS=2 --env CARGO_PROFILE_TEST_DEBUG=0 --env CARGO_PROFILE_DEV_DEBUG=0 --env RUST_TEST_THREADS=8 --env CABLE_TEST_PORT_RANGE=53300-53399 --env INTEGRATION_TEST_PORT_RANGE=53300-53399 sha256:80bed826ce3b998e8ba75b85b25d18055066760982413e4483dd9779c5f053b2 cargo test --offline --locked -j 2 --workspace --exclude html5ever --no-fail-fast -- --test-threads=8
```

```text
Pinned workspace totals: 50 summary blocks; 2394 passed; 1 failed; 11 ignored
test result: FAILED. 991 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out; finished in 409.27s
```

Its assertion showed the actual task array contained `{"name":"slack imports","seconds":30}` between Fizzy and retention, while the expected array omitted it. The correction retains exact array equality.

Corrected complete rerun (exit 0), then clippy (exit 0):

```sh
.scratch/pinned-check.sh ws16-canonical-suite-verified cargo test --offline --locked -j 2 --workspace --exclude html5ever --no-fail-fast -- --test-threads=8
.scratch/pinned-check.sh ws16-canonical-clippy cargo clippy --offline --locked -j 2 --workspace --all-targets -- -D warnings
```

Every raw libtest summary from `canonical-suite-verified.log`, in execution order:

```text
test result: ok. 992 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 318.85s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.69s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 718 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 50.70s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.35s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.96s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.75s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.26s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.56s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.49s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.14s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.81s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.49s
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.37s
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

Raw final clippy line:

```text
    Finished `dev` profile [unoptimized] target(s) in 56.63s
```

Workspace aggregate counter (`.scratch/verify-workspace.py`) and actual invocations:

```python
#!/usr/bin/env python3
from pathlib import Path
import re
import sys
log = Path(sys.argv[1]).read_text()
rows = re.findall(r"^test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", log, re.M)
assert len(rows) == 50, f"Expected 50 complete workspace summary blocks, got {len(rows)}"
passed, failed, ignored = (sum(int(row[i]) for row in rows) for i in range(3))
assert failed == int(sys.argv[2]), (failed, sys.argv[2])
assert ignored == 11, ignored
print(f"Pinned workspace totals: {len(rows)} summary blocks; {passed} passed; {failed} failed; {ignored} ignored")
```

```sh
python3 .scratch/verify-workspace.py .scratch/canonical-suite-final.log 1
python3 .scratch/verify-workspace.py .scratch/canonical-suite-verified.log 0
```

```text
Pinned workspace totals: 50 summary blocks; 2394 passed; 1 failed; 11 ignored
Pinned workspace totals: 50 summary blocks; 2395 passed; 0 failed; 11 ignored
```

All existing explicit ignores from the corrected full run (no Slack-owned ignore):

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
test crates/kit/src/error.rs - error::halt (line 91) ... ignored
test crates/kit/src/lib.rs - (line 7) ... ignored
```

## Cleanup and final readback

The fresh clone's Cargo target measured 11G immediately before removal. `.scratch/canonical/rust/target` was deleted after the suite, clippy, owned tests and encrypted-row export completed. Earlier scratch targets were already absent. All scratch Cargo targets are gone; primary `rust/target`, source clones, seeds, recorded outputs and logs are preserved. No WS16 cargo/rustc/test processes, Docker containers or listeners in ports 53300–53399 remain. A final fetch found origin/main still at `20dc8ea3`; the Slack source checker was rerun afterward and still reports no drift.

```text
Deleted WS16 scratch Cargo target: .scratch/canonical/rust/target
WS16 scratch Cargo targets: 0; primary rust/target preserved
WS16 cargo/rustc/test processes: 0
```

The committed report mirror is `rust/plans/ws16-wave4-report.md`; the requested external report has identical bytes. Final worker/fresh-clone/remote HEAD equality is checked after the report push; that report-only commit leaves all tested code and vectors unchanged.
