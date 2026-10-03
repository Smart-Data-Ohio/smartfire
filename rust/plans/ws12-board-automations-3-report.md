# WS12 service reads and generic recorder checkpoint

Branch: `rust/ws12-board-automations-3`, stacked on #206 at `ae99f082b`.
Verified Rust source head: `ba6386dbe8fac7fdb22ee8dc4a2a4e1d584ccbcc`.
Publication head is the reporting commit given in the final reply.

This is a partial validation checkpoint. The service read reductions, sixteen requested
owner/mutation comparisons and generic recorder API are implemented. All 231 formerly
deferred declarations are individually reconciled: 193 have explicit Rust assertion mappings;
38 retain a named owner and evidence gap. They receive no new coverage credit. The full
488-declaration inventory therefore reports 442 ported, 8 existing peer tests and 38 deferred.
WS12 still owns named assertion work; this is not an owner-blocked-only stop.

## Service reads

All measurements use the installed WS11 REST/MCP entrypoints at 5 and 50 owned rows,
with identical request fixtures and reader+writer SELECT capture before and after.
The full status, selected headers, literal response body and committed message/history/
ledger/audit/handoff/job facts match the 244-case Rails corpus. The regenerated corpus is
identical to the committed one; no response fields are masked.

The stacked baseline is measured at ae99f082b (201/189/85/35/113), rather than substituting
Astra's earlier 198/186/83/33/105. The middle column records a740ff9ca before the main merge; its sender-name shortcut was
subsequently removed by the stale-profile correction. The final Rust column includes that
correction and reviewed WS11 adapter changes.

| Surface | Before 5/50 | Service-only 5/50 | Final merged 5/50 | Rails 5/50 |
| --- | --- | --- | --- | --- |
| REST create | 201/201 | 180/180 | 168/168 | 60/60 |
| MCP create | 189/189 | 168/168 | 156/156 | 61/61 |
| REST update | 85/85 | 65/65 | 63/63 | 33/33 |
| MCP update | 85/85 | 65/65 | 63/63 | 34/34 |
| MCP board update | 85/85 | 65/65 | 63/63 | 34/34 |
| REST result | 35/35 | 27/27 | 25/25 | 26/26 |
| MCP result | 35/35 | 27/27 | 25/25 | 27/27 |
| REST handoff | 113/113 | 86/86 | 79/79 | 55/55 |
| MCP handoff | 113/113 | 86/86 | 79/79 | 56/56 |

`agent_work.rs` combines the fresh thread/membership lookup, uses the public current-capability
batch API, and returns the operation's persisted thread instead of reloading it again.
`channel_thread/board.rs` joins owner users, room memberships and agent associations in one
bounded json_each query, followed by the public live capability query. Model owner validation
and handoff receiver policy retain current user/agent/grant checks. `channel_thread.rs` and
`thread_tag.rs` reuse known inserted rows; the private validated save avoids a second validation
only while the same writer lock remains held and no authorization fact has changed.
The handoff audit retains a fresh previous-owner read. A separate Rails-pinned regression
changes the sender profile after loading its object and proves the audit keeps the current
previous-owner name while preserving the supplied actor snapshot.
Existing locks, fresh ownership/policy rechecks, callback order and every board broadcast
remain. The full fixed gap is not claimed eliminated; callback reads and stricter rechecks remain.
No `controllers/agents/*` file was edited by this branch. Main #196/#205/#207/#210/#208/#211 is merged with
merge commits; #206's branch/history was neither merged as a PR nor rebased.

The two old availability regressions required exactly four reads. Actual availability is now
2/2 at 10/200 owners with either explicit or legacy posting access. They now require flat
counts at or below Rails' four-read ceiling, preserving all owner/result assertions.

## Sixteen requested owner/mutation comparisons

`work/agent_named.rb`, source hashes and `ws12_agent_named.json` project every assertion from
these actual Rails operations, including complete errors, persisted thread fields, history
metadata and inbox recipient/source/unread facts. All sixteen reject the production control.
The two previously credited legacy/suspended declarations are retained; the other fourteen
are additional reconciled declarations. The WS11-owned case-ports.json remains untouched.

The test names to route into WS11's manifest are:

- `ws12_agent_named_active_member_post_owner`
- `ws12_agent_named_legacy_post_owner`
- `ws12_agent_named_suspended_owner_validation`
- `ws12_agent_named_nonmember_owner_validation`
- `ws12_agent_named_missing_post_owner_validation`
- `ws12_agent_named_outside_human_owner_validation`
- `ws12_agent_named_bot_without_agent_validation`
- `ws12_agent_named_owner_unavailable_after_access_change`
- `ws12_agent_named_status_event_with_note`
- `ws12_agent_named_status_inbox_human_path`
- `ws12_agent_named_tag_set_preserves_status`
- `ws12_agent_named_tags_run_url_missing_fields_validation`
- `ws12_agent_named_result_event_agent_actor`
- `ws12_agent_named_result_requires_ownership`
- `ws12_agent_named_status_requires_ownership`
- `ws12_agent_named_status_and_note_validation`

They live in `crates/db/src/tests/ws12_agent_named_test.rs`. Independent validation attempts
use fresh Rails instances, like separate HTTP requests; stale Rails validation error objects
are not carried from one attempted request to another. No saved state or error is masked.

## Generic recorder and WS11 reader seam

`activity_item/recording_source.rs` exports ActivityEventType, SourceAuthorization,
ActivityRecordingFacts, ActivityRecordingSource and AgentBudgetNoticeActivityReader.
The existing `ActivityItem::record` signature remains a parser facade over `record_typed`.
`record_from_source` covers additional persisted polymorphic sources; the batch entrypoint
loads source and active-human facts once per writer transaction using bounded loaders.

The handled source types are Message, WorkThreadEvent, BoardSlaNudge, SavedItem,
CalendarEvent (Rails Event), HuddleGrant, AgentApproval, AgentBudgetNotice through the reader,
ScheduledMessage, Session and TwoFactorCredential. CallerAuthorized is the explicit
skip_source_check contract. SavedItem/Event/ScheduledMessage/Session/TwoFactorCredential
have no Rails activity-recipient hook: inbox visibility alone does not authorize generic
recording. HuddleGrant uses its current direct-room audience; Approval its current deciders.
Only active humans record, creator exclusion remains under caller authorization, and only
Message/WorkThreadEvent may group. The first non-grouped event and handled/read timestamps
remain idempotent. Reviewed dirty-column writes and operation-snapshot broadcasts are reused.
Source, recipient and broadcast rollback is tested; owning-domain writers/callbacks stay intact.

The **AgentBudgetNoticeActivityReader** seam for WS11 is:

```rust
fn recording_recipient_ids(
    &self, conn: &Connection, notice_id: i64,
) -> Result<Option<Vec<i64>>>;
```

None means unsaved/deleted. Some verifies current persistence and returns the current owner,
or active human administrators when the owner association is absent. The generic recorder
applies its active-human filter. Consume it with
`ActivityItem::record_with_budget_notice_reader(tx, user_id,
ActivitySource::AgentBudgetNotice(id), event_type, authorization, &reader)`.
The plain budget entrypoint requires this explicit owning reader. WS12 provides a test-side
consumer using real rows; it does not duplicate WS11's notice writer or production reader.
The three flagged budget reads in the inbox presenter still await WS11's typed facts reader
(and subsequent WS12/WS11-UI adapter replacement). Details are in ws12-generic-recorder-api.md.

The Rails producer has 135 complete source/recipient/auth cases plus caller-authorized
Message grouping and handled-source idempotency. Seven DB regressions cover the complete
matrix, rollback, grouping, idempotency, required reader seam and 10/100-recipient batching.
At both sizes, source reads are 1 and user reads are 1. The direct-room NULL-involvement case
fails against the original NOT IN predicate and passes with the explicit NULL clause,
matching Rails' exclusion of only explicit nothing/invisible values.

## Reconciliation and additional exact evidence

`ws12-assertion-reconciliation.json` retains every one of the 231 original declarations,
its source SHA, exact title/line, Rust file/test mappings or an explicit owner and gap.
`verify_ws12_reconciliation.py --test-log LOG` requires real passed test receipts as well as
source/function presence; ignored, failed, missing or non-running assertions earn no credit.
It rejects the first fresh-clone receipt because the two availability tests failed there.
The inventory generator applies only these explicit mappings.

Additional pinned producers/tests cover all eleven human handoff declarations plus the stale-profile audit case and all four
inbox helper declarations. The handoff producer queries persisted rows after failed writes,
avoiding Rails' rolled-back association cache. It asserts human actor/from/to metadata,
package arrays, exact audit targets, webhook enqueue/status and denial/rollback facts.
The helpers assert review labels, reminder venue/no-venue bodies and lockout label/body/title/path.
Both suites reject their production controls (11/11 and 4/4); the extra stale-profile regression fails against the unsafe sender-name shortcut.

Main #210 supplies four previously flagged callback comparisons: hop-limit suppression,
human root hop=0, deletion without an agent owner, and queued deleted-owner snapshot delivery.
Their complete ledger/thread/history and queue/request/HMAC vectors are regenerated unchanged
from Rails. The final workspace receipt must execute their installed WS11 tests before these
four mappings receive credit; the owning manifest is inherited from main, not edited here.

## Remaining named assertions and owners

The 38 flags comprise 17 WS12 named assertions, 18 WS11-API named assertions and three
WS11-UI browser assertions. All entries below are validation/evidence gaps, not a claim
that the installed endpoint or production feature is absent. They remain flagged; broad HTTP or model vectors alone are
not credited as their complete original assertion sets.

| Rails declaration | Remaining owner | Reason |
| --- | --- | --- |
| `test/controllers/activity_items_controller_test.rb:164` — index resolves the current user's overdue invitations but no one else's | WS12 with WS11-UI | Index-triggered expiry and viewer/decider badge scope still need a complete named projection. |
| `test/controllers/activity_items_controller_test.rb:228` — index expires overdue approvals and drops the decider's badge | WS12 with WS11-UI | Index-triggered expiry and viewer/decider badge scope still need a complete named projection. |
| `test/controllers/agents/mcp_handoff_test.rb:100` — handoff_work shares its throttle bucket with the rest endpoint | WS11-API (WS12 service preserved) | Complete named REST/MCP/poll/ack/webhook/throttle or cross-surface projection remains; adapter files are peer-owned. |
| `test/controllers/agents/posts_controller_test.rb:438` — creation, listing, and event delivery share one work payload | WS11-API (WS12 service preserved) | Complete named REST/MCP/poll/ack/webhook/throttle or cross-surface projection remains; adapter files are peer-owned. |
| `test/controllers/agents/work_controller_test.rb:114` — show includes links with pull request, event, and drive entries | WS11-API (WS12 service preserved) | Complete named REST/MCP/poll/ack/webhook/throttle or cross-surface projection remains; adapter files are peer-owned. |
| `test/controllers/agents/work_controller_test.rb:158` — list includes links on owned threads | WS11-API (WS12 service preserved) | Complete named REST/MCP/poll/ack/webhook/throttle or cross-surface projection remains; adapter files are peer-owned. |
| `test/controllers/agents/work_delivery_test.rb:14` — assignment appears in event polling with the work payload | WS11-API (WS12 service preserved) | Complete named REST/MCP/poll/ack/webhook/throttle or cross-surface projection remains; adapter files are peer-owned. |
| `test/controllers/agents/work_delivery_test.rb:34` — assignment work payload includes links | WS11-API (WS12 service preserved) | Complete named REST/MCP/poll/ack/webhook/throttle or cross-surface projection remains; adapter files are peer-owned. |
| `test/controllers/agents/work_delivery_test.rb:51` — unassignment appears in event polling | WS11-API (WS12 service preserved) | Complete named REST/MCP/poll/ack/webhook/throttle or cross-surface projection remains; adapter files are peer-owned. |
| `test/controllers/agents/work_delivery_test.rb:64` — ack works on work rows | WS11-API (WS12 service preserved) | Complete named REST/MCP/poll/ack/webhook/throttle or cross-surface projection remains; adapter files are peer-owned. |
| `test/controllers/agents/work_delivery_test.rb:75` — assignment enqueues the webhook instead of blocking on it | WS11-API (WS12 service preserved) | Complete named REST/MCP/poll/ack/webhook/throttle or cross-surface projection remains; adapter files are peer-owned. |
| `test/controllers/agents/work_delivery_test.rb:97` — assignment posts no webhook without read_messages | WS11-API (WS12 service preserved) | Complete named REST/MCP/poll/ack/webhook/throttle or cross-surface projection remains; adapter files are peer-owned. |
| `test/controllers/agents/work_delivery_test.rb:113` — polling omits work rows for rooms the agent lost read_messages in | WS11-API (WS12 service preserved) | Complete named REST/MCP/poll/ack/webhook/throttle or cross-surface projection remains; adapter files are peer-owned. |
| `test/controllers/agents/work_delivery_test.rb:128` — polling omits work rows after membership removal | WS11-API (WS12 service preserved) | Complete named REST/MCP/poll/ack/webhook/throttle or cross-surface projection remains; adapter files are peer-owned. |
| `test/controllers/agents/work_delivery_test.rb:142` — work events do not count toward the message rate limit | WS11-API (WS12 service preserved) | Complete named REST/MCP/poll/ack/webhook/throttle or cross-surface projection remains; adapter files are peer-owned. |
| `test/controllers/agents/work_handoff_test.rb:45` — the receiver polls the handoff with its context package | WS11-API (WS12 service preserved) | Complete named REST/MCP/poll/ack/webhook/throttle or cross-surface projection remains; adapter files are peer-owned. |
| `test/controllers/agents/work_handoff_test.rb:64` — the receiver acks the handoff row | WS11-API (WS12 service preserved) | Complete named REST/MCP/poll/ack/webhook/throttle or cross-surface projection remains; adapter files are peer-owned. |
| `test/controllers/agents/work_handoff_test.rb:76` — the handoff enqueues the receiver webhook with the package | WS11-API (WS12 service preserved) | Complete named REST/MCP/poll/ack/webhook/throttle or cross-surface projection remains; adapter files are peer-owned. |
| `test/controllers/agents/work_handoff_test.rb:176` — handoffs throttle at 60 a minute per credential | WS11-API (WS12 service preserved) | Complete named REST/MCP/poll/ack/webhook/throttle or cross-surface projection remains; adapter files are peer-owned. |
| `test/controllers/channel_threads_board_test.rb:215` — the owning agent can change the status but cannot reassign | WS12 | Complete original controller/picker/Cable assertion sequence still needs its own audited projection. |
| `test/controllers/channel_threads_board_test.rb:232` — the owning agent without manage_threads cannot change the status | WS12 | Complete original controller/picker/Cable assertion sequence still needs its own audited projection. |
| `test/controllers/channel_threads_board_test.rb:334` — members can reply in a post and join or leave it from the page | WS12 | Complete original controller/picker/Cable assertion sequence still needs its own audited projection. |
| `test/controllers/channel_threads_board_test.rb:358` — any member can link and unlink but non-members cannot | WS12 | Complete original controller/picker/Cable assertion sequence still needs its own audited projection. |
| `test/controllers/channel_threads_board_test.rb:546` — a status change replaces the list row and moves the column row | WS12 | Complete original controller/picker/Cable assertion sequence still needs its own audited projection. |
| `test/controllers/channel_threads_board_test.rb:620` — deleting a channel thread broadcasts no board row remove | WS12 | Complete original controller/picker/Cable assertion sequence still needs its own audited projection. |
| `test/controllers/channel_threads_board_test.rb:632` — bot posting API returns 422 in a board | WS12 | Complete original controller/picker/Cable assertion sequence still needs its own audited projection. |
| `test/integration/agent_boards_test.rb:17` — an agent post flows from creation through reply to result | WS11-API (WS12 service preserved) | Complete named REST/MCP/poll/ack/webhook/throttle or cross-surface projection remains; adapter files are peer-owned. |
| `test/system/activity_inbox_test.rb:13` — handles an item, clears the badge, and receives a later activity | WS11-UI | Complete named browser sequence still needs execution; server/domain/Cable bytes are already covered. |
| `test/system/activity_inbox_test.rb:42` — filters by type and saves a notification switch | WS11-UI | Complete named browser sequence still needs execution; server/domain/Cable bytes are already covered. |
| `test/system/agent_work_assignment_test.rb:19` — assigns an agent from the update dialog and renders its API status change after refresh | WS11-UI | Complete named browser sequence still needs execution; server/domain/Cable bytes are already covered. |
| `test/system/board_automations_test.rb:17` — the board creator configures tag rules and sla timers | WS12 | Complete named browser sequence still needs execution; server/domain/Cable bytes are already covered. |
| `test/system/board_automations_test.rb:39` — a plain member sees no automations link and is forbidden directly | WS12 | Complete named browser sequence still needs execution; server/domain/Cable bytes are already covered. |
| `test/system/board_automations_test.rb:51` — a tagged post auto-assigns and the history shows it | WS12 | Complete named browser sequence still needs execution; server/domain/Cable bytes are already covered. |
| `test/system/board_automations_test.rb:64` — a person hands a post to an agent with context | WS12 | Complete named browser sequence still needs execution; server/domain/Cable bytes are already covered. |
| `test/system/board_automations_test.rb:83` — the stale-work digest renders on the board | WS12 | Complete named browser sequence still needs execution; server/domain/Cable bytes are already covered. |
| `test/system/boards_test.rb:12` — boards carry posts from creation to result with live rows | WS12 | Complete named browser sequence still needs execution; server/domain/Cable bytes are already covered. |
| `test/system/boards_test.rb:86` — a non-member cannot open the board | WS12 | Complete named browser sequence still needs execution; server/domain/Cable bytes are already covered. |
| `test/controllers/channel_threads_controller_test.rb:433` — the owner picker lists eligible agents with profiles and excludes ineligible ones | WS12 | Complete original controller/picker/Cable assertion sequence still needs its own audited projection. |

Remaining production integration: **WS11** supplies the typed budget-notice facts reader;
**WS12/WS11-UI** replace the three presenter reads once it is available. WS12 named checks
still remain, so this checkpoint is not owner-blocked-only. No other missing WS12 production
behaviour was identified in this round; the earlier full owned-file audit is retained in
ws12-board-automations-2-unported.md, with its generic-recorder deferral superseded above.

## Verification commands and raw receipts


All commands below run from the WS12 worktree. Cargo uses CARGO_BUILD_JOBS=2 and the existing
machine-wide rustc wrapper, with `.scratch/target` reused for the isolated clone. The suites also emit small
`rust/target` JSON receipt directories inside the owned clones; these are removed after
preserving their receipts. No other worktree target, model server or listener was touched.

Common verification environment:

```sh
export CARGO_TARGET_DIR="$PWD/.scratch/target" CARGO_BUILD_JOBS=2 CI=1
export LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs"
export PATH="$PWD/.scratch/rails-media/usr/bin:$PATH"
export CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499
export TMPDIR="$PWD/.scratch/board-automations-3-final-latest/source/.scratch/tmp"
```

Rails commands use the verified reference image and current-main Rails source SHA checks for
board/work/activity files, honoring the approved drift after d7c7de92:

```sh
export PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2
export PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_CPUS=1
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/work/agent_named.rb
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/users/generic_recorder.rb
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/work/handoff_named.rb
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/users/activity_helpers_named.rb
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/agents/work_writes_http_contract.rb
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/boards/write_views.rb
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/work/human_http.rb
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/work/link_model.rb
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/agents/next2_model_cases.rb hop_limit
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/agents/next2_model_cases.rb human_root
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/agents/next2_model_cases.rb delete_no_owner
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/agents/next2_job_cases.rb delete_owned
TMPDIR="$PWD/.scratch/board-automations-3/tmp" bash rust/reference-tools/views/agents_ui/inbox_http.sh --check
```

Raw Rails summary lines from those runs (stdout JSON and stderr receipts are retained under
`.scratch/board-automations-3/`; the 244/96/90/29 JSON files compare equal to the committed vectors):

```
Rails WS12 agent named oracle: 16 complete owner/mutation assertion sets; 0 masks
Rails generic recorder oracle: 135 source/recipient/authorization cases; preserved handled-source idempotency; 0 masks
Rails human handoff named oracle: 12 complete assertion sets; 0 masks
Rails activity helper named oracle: 4 complete assertion sets; 0 masks
Rails WS11 callback oracle: hop_limit; complete persisted/queued comparison; 0 masks; unchanged
Rails WS11 callback oracle: human_root; complete persisted/queued comparison; 0 masks; unchanged
Rails WS11 callback oracle: delete_no_owner; complete persisted/queued comparison; 0 masks; unchanged
Rails WS11 callback oracle: delete_owned; complete persisted/queued comparison; 0 masks; unchanged
Rails boards-write oracle: 96 complete comparisons; 0 masks; unchanged
Rails human-work oracle: 90 complete comparisons; 0 masks; unchanged
Rails work-links oracle: 29 complete comparisons; 0 masks; unchanged
Rails work-write oracle: 244 complete responses and persisted fact sets; 0 masks; unchanged
inbox Rails HTTP oracle: 86 responses; 11 source types; reference d7c7de92
inbox Rails HTTP fixtures: committed corpus matches regenerated bytes
```

The query regression ran before the service edit at ae99f082b, then against a740ff9ca, using
`cargo test --locked --manifest-path rust/Cargo.toml -p campfire ws12_agent_work_service_reads_are_reduced_and_flat_at_two_sizes -- --test-threads=2 --nocapture`.
The 16-case domain control disabled owner validation/availability and agent status/result writes;
the generic source control records nothing for new sources, followed by an authorization and
idempotency overwrite control. Handoff and helper controls return incorrect production results.
Every control was restored byte-for-byte before committing or running final gates.

Raw failing-first/negative-control summary lines:

```
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2576 filtered out; finished in 14.44s
test result: FAILED. 0 passed; 16 failed; 0 ignored; 0 measured; 1295 filtered out; finished in 2.01s
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 1323 filtered out; finished in 19.24s
test result: FAILED. 3 passed; 2 failed; 0 ignored; 0 measured; 1323 filtered out; finished in 12.71s
test result: FAILED. 0 passed; 11 failed; 0 ignored; 0 measured; 1329 filtered out; finished in 1.41s
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 2611 filtered out; finished in 1.67s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1340 filtered out; finished in 0.11s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1341 filtered out; finished in 0.07s
```

The NULL and stale-profile regressions then ran green against their corrections:

```
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 1334 filtered out; finished in 13.62s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 1330 filtered out; finished in 0.40s
```

Seeds default, first_run and agents_ui were checked with actual Rails verify_parity_seed.rb
at the frozen time before copying into the clone. Their raw JSON summary lines are:

```
  "passed": 29,
  "failed": 0

  "passed": 4,
  "failed": 0

  "passed": 40,
  "failed": 0
```

The final source is an isolated local clone with no tracked differences, checked out at the
verified code SHA above. Source paths/seeds are explicit; CI=1 prevents missing-seed parity
fixtures from being silently credited. One intentional missing-seed helper unit test still runs.

```sh
cargo test --locked --manifest-path .scratch/board-automations-3-final-latest/source/rust/Cargo.toml --workspace --no-fail-fast -- --test-threads=4
python3 rust/reference-tools/users/summarize_ws12_tests.py .scratch/board-automations-3/logs/workspace-latest.log
python3 rust/reference-tools/users/verify_ws12_reconciliation.py --test-log .scratch/board-automations-3/logs/workspace-latest.log
cargo test --locked --manifest-path .scratch/board-automations-3-final-latest/source/rust/Cargo.toml -p campfire ws12_agent_work_service_reads_are_reduced_and_flat_at_two_sizes -- --test-threads=1 --nocapture
cargo clippy --locked --manifest-path .scratch/board-automations-3-final-latest/source/rust/Cargo.toml --workspace --all-targets -- -D warnings
.scratch/board-automations-3-final-latest/source/rust/ci/with-release-inputs.sh cargo check --locked -p campfire
```

Raw workspace, reconciliation, standalone query, strict clippy and release-input receipts:

```
test result: ok. 2625 passed; 0 failed; 7 ignored; 0 measured; 0 filtered out; finished in 971.19s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.15s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1345 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 182.44s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.51s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.67s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.29s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 22.27s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.87s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.63s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.19s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.10s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.23s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.88s
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
WS12 workspace totals: 4708 passed; 0 failed; 16 ignored; 61 result summaries
WS12 visible missing-seed notices: 0; intentional missing-seed unit test passed: True
WS12 assertion reconciliation: 231 declarations; 193 executed mappings; 38 explicitly flagged; 0 missing or non-running credited tests

WS12_AGENT_WORK_READS rest_create size=5/50 Rust=168/168 Rails=60/60
WS12_AGENT_WORK_READS mcp_create size=5/50 Rust=156/156 Rails=61/61
WS12_AGENT_WORK_READS rest_update size=5/50 Rust=63/63 Rails=33/33
WS12_AGENT_WORK_READS mcp_update size=5/50 Rust=63/63 Rails=34/34
WS12_AGENT_WORK_READS mcp_board_update size=5/50 Rust=63/63 Rails=34/34
WS12_AGENT_WORK_READS rest_result size=5/50 Rust=25/25 Rails=26/26
WS12_AGENT_WORK_READS mcp_result size=5/50 Rust=25/25 Rails=27/27
WS12_AGENT_WORK_READS rest_handoff size=5/50 Rust=79/79 Rails=55/55
WS12_AGENT_WORK_READS mcp_handoff size=5/50 Rust=79/79 Rails=56/56
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2631 filtered out; finished in 7.85s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 57s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 01s
```

An earlier completed clone run had exactly two availability-count assertion failures (2/2
reads against an obsolete exact 4/4 expectation); the flat-at-or-below-ceiling correction is
included. Two subsequent verification attempts were superseded by the NULL/profile corrections;
one link attempt was killed with signal 9. A complete 59f4b4525 workspace run passed
4,689 tests before #210 moved main; its receipt correctly refuses the four newly mapped,
then non-running callback tests. The first #210 clone run lost its app process to
SIGKILL with no preceding assertion failure. Its remaining crate runs are not credited
as a complete workspace gate. The final retry uses four test threads and the same
compiler throttle; no kernel cause was available for the kill. None of those older
runs is credited as the final gate. The four-thread a05da652e run subsequently passed
4,707 tests, all 193 mappings and the standalone service probe; main then advanced with
#208/#211. Its obsolete build checks were stopped, and the final merged code was verified
in another fresh clone. The unrelated Twitter Rails changes do not alter the WS12-owned
Rails source hashes used by the comparisons above.
The final successful receipts above are from the corrected code. Target cleanup and the final
remote-head verification are recorded before publication.


Raw cleanup receipt (small JSON outputs are preserved under `.scratch/board-automations-3/target-receipts/`):

```
WS12 target cleanup: 5 target directories removed; 0 target directories remain in .scratch
```
