# PR #215 review fixes — WS12 checkpoint

Branch: `rust/ws12-board-automations-3`. Verified code SHA: `5d08b771a00863f2acded1ffce114a1d0c8bb43f`.
Reviewed baseline: `746d69cb`. Main `15653de60` (#206) was merged with merge
commit `66167c656` before fixes. Main later moved to `1ccf68733` (#212);
merge commit `6a41a12f7` preserved both sides. Main then moved to `0d8da877e` (#214);
merge commit `5d08b771a` retained the reviewed API/reader implementations, and a third
fresh clone ran the final gates. Earlier full green receipts are retained separately.
Main then moved to `941a5ef4a` (#217); merge `91a8e827d` changes only Rails search tests
and documentation. An exact Git diff verified zero changes to tested Rust/Cargo and
explicit asset-build inputs; those completed checks remain applicable.
No pushed history was rewritten. The final report/control-tool-only commit preserves this tested Rust code.

All four requested review fixes are complete. Broader WS12 named coverage remains partial:
176 of the 231 reconciled declarations have executed mappings, and 55 are explicitly flagged.
This is **not** an owner-blocked-only checkpoint: 25 flags remain WS12-owned, 27 belong
to WS11-API and three to WS11-UI. The budget-notice generic-recorder adapter is now WS12-owned:
#214 supplies the typed reader and has already replaced all three presenter seams.

## Changes and failing-first evidence

- Recorder: `crates/db/src/models/activity_item/recorder.rs` restores the current-user lookup
  before broadcasts from independently committed WorkThreadEvent recipient writes.
  Authorized recording snapshots remain unchanged: deactivated Kevin still receives the
  persisted item, but receives zero activity frames. `ws12_recorder_race_test.rs` uses a
  real second SQLite connection after Jason's committed frame, with no sleeps. Both the
  old shortcut control and the fresh Rails producer reproduce the review schedule.
  Same-transaction board-opener and generic batches are retained. The SLA dispatcher
  rechecks recipient status/role in its source transaction, so its batch remains safe.
  Current-state authorization rechecks and existing locks were retained.
- Missed huddle: `controllers/ws12_huddle_copy_tests.rs` pins the original
  **You missed a huddle from Jason** comparison, plus Kevin's started-huddle copy.
  Both real HTML/JSON body producers and the served inbox article are asserted.
  Both previously credited tests pass with the missed-huddle producers corrupted;
  the new comparison fails on that exact control. Source guards and persisted Rails
  HuddleGrant inputs are in `reference-tools/users/huddle_copy.rb` and its vector.
- Unread stream: `crates/db/src/models/channel_thread/board.rs` now uses the shared
  Rails `user_<id>_unreads` helper. The existing DB test compares every observed
  unread stream and its entire payload against a freshly captured Rails creation.
  A new app test subscribes to the actual channel while creating posts through
  both unchanged WS11 REST/MCP adapters. These adapter files were not edited.
- SLA waiver: `controllers/rooms/board_automation_tests.rs` recognizes only PATCH
  SLA settings requests whose exact input is among 35 proven Rails crashes.
  Valid/unrelated 500s fail comparison. Nine missing/scalar roots all produce
  Rust 400, empty bodies, unchanged rules/audits and **zero write statements**
  across every connection. Rails returns 500 with unchanged facts. This is the
  maintainer-approved don't-replicate-crashes difference, documented in
  `ws12-approved-differences.md`. The production handler from main is retained.

The fresh Rails reference is d7c7de92 plus approved drift; board/work/activity files use
origin/main's Rails versions as required. Producers hash-guard the exact source files.
No Rails production file was changed. New vectors are actual Rails outputs, not Rust outputs.

Failing-first raw summaries (full logs under `.scratch/pr215-fixes/logs/`):

Recipient race, exact current test with the old `Some(user)` shortcut:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1352 filtered out; finished in 0.17s
```

Unread creation comparison before the stream fix:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1352 filtered out; finished in 0.06s
```

Missed-huddle producer corruption against the new original-copy comparison:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2648 filtered out; finished in 0.58s
```

Unrelated-500 waiver regression against the old blanket predicate:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2646 filtered out; finished in 0.00s
```

Controls changed only producer code; expected vectors and assertions were not mutated.
The reproducible control installer is `reference-tools/users/ws12_pr215_mutations.py`.
Its temporary changes to four production inputs were restored before final gates.
Mutation receipts are preserved in `ws12-pr215-assertion-audit.json` and scratch logs.

## Unread call-site audit against Rails

| Rust call site | Rails behavior | Resolution |
|---|---|---|
| `db/broadcasts.rs::unread_rooms_stream_name` | `UnreadRoomsChannel.stream_name_for` | Already `_unreads`; retained. |
| `channels/unread_rooms.rs` subscription | `UnreadRoomsChannel#subscribed` | Already the per-user helper; retained. |
| `channels/broadcasts.rs::mark_room_unread` | Per-user unread room channel | Already the helper; retained. |
| `channels/broadcasts.rs::unread_room` | `Message::Broadcasts#broadcast_unread_room` | Already the helper; retained. |
| `db/slash_commands.rs` posted-message unread notification | Imported message's `Message::Broadcasts` callback | Already the helper; retained. |
| `db/models/scheduled_message.rs` delivered-message unread notification | Dispatcher's persisted Message broadcast callback | Already the helper; retained. |
| `db/models/channel_thread/board.rs` creation | `ChannelThread#announce_board_post` | Sole wrong literal; replaced with the helper. |

## SELECT measurements through unchanged WS11 entrypoints

Counts are at **5 / 50 owned rows**. The reviewed-head column is Astra's read-only receipt,
not a claimed fresh full baseline build. Immediately after the #206 merge, the old-shortcut control and the fixed probe had
identical counts (163/163 REST create and 151/151 MCP create; other rows unchanged).
The current final column is measured after #212 and #214 as well. Changes to main's fixed rendering
cost are not attributed to the recipient-status fix.

| Path | Reviewed 746d69cb Rust | 15653de60, old shortcut | Fixed Rust | Rails |
|---|---:|---:|---:|---:|
| REST create | 168/168 | 163/163 | 143/143 | 60/60 |
| MCP create | 156/156 | 151/151 | 141/141 | 61/61 |
| REST update | 63/63 | 63/63 | 63/63 | 33/33 |
| MCP update | 63/63 | 63/63 | 63/63 | 34/34 |
| MCP board update | 63/63 | 63/63 | 63/63 | 34/34 |
| REST result | 25/25 | 25/25 | 25/25 | 26/26 |
| MCP result | 25/25 | 25/25 | 25/25 | 27/27 |
| REST handoff | 79/79 | 79/79 | 68/68 | 55/55 |
| MCP handoff | 79/79 | 79/79 | 68/68 | 56/56 |

The test asserts full response bytes, selected response headers and complete persisted
fact projections at both sizes. Main #214 reduced additional adapter costs (including handoff); these savings are
not attributed to the recorder safety fix. Fixed cost remains above Rails on some paths; the
approved growth bar passes, with equal counts at both sizes.

Final fresh-clone command:

```sh
.scratch/pr215-fixes/run-gate.sh cargo test --locked -p campfire ws12_agent_work_service_reads_are_reduced_and_flat_at_two_sizes -- --test-threads=1 --nocapture
```

```text
test controllers::ws12_agent_work_query_tests::ws12_agent_work_service_reads_are_reduced_and_flat_at_two_sizes ... WS12_AGENT_WORK_READS rest_create size=5/50 Rust=143/143 Rails=60/60
WS12_AGENT_WORK_READS mcp_create size=5/50 Rust=141/141 Rails=61/61
WS12_AGENT_WORK_READS rest_update size=5/50 Rust=63/63 Rails=33/33
WS12_AGENT_WORK_READS mcp_update size=5/50 Rust=63/63 Rails=34/34
WS12_AGENT_WORK_READS mcp_board_update size=5/50 Rust=63/63 Rails=34/34
WS12_AGENT_WORK_READS rest_result size=5/50 Rust=25/25 Rails=26/26
WS12_AGENT_WORK_READS mcp_result size=5/50 Rust=25/25 Rails=27/27
WS12_AGENT_WORK_READS rest_handoff size=5/50 Rust=68/68 Rails=55/55
WS12_AGENT_WORK_READS mcp_handoff size=5/50 Rust=68/68 Rails=56/56
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2667 filtered out; finished in 7.48s
```

## Assertion reconciliation and producer sweep

All **193** previously mapped declarations were inspected against their original assertion
starts and their Rust tests/vector response or fact projections, including the other **192**
requested declarations. The audit found **17 additional unsupported credits** and flagged them;
the original missed-huddle credit now points to the discriminating comparison.

Producer controls spot-checked **10 original mapped declarations**, including missed huddle
and **nine of the other 192**. Eleven producer modes ran in 23 test executions: the extra
HTML navigation and recorder race probes are not claimed as additional original declarations.
No claim is made that every inspected declaration was individually mutated.

| Original output checked | Producer control | Result |
|---|---|---|
| Missed-huddle body | Corrupt both HTML/JSON body arms | Both old credited tests survive; new comparison rejects. |
| Started-huddle body | Corrupt both body arms | Credited inbox corpus rejects. |
| Review-request label | Corrupt its label | Credited named helper rejects. |
| None-status body | Corrupt the None fallback | Credited inbox corpus rejects. |
| HTML type filtering | Remove filtered DOM rows | Both old credited tests survive; declaration flagged. |
| Positive done board results | Return no done posts | Credited tests survive; declaration flagged. |
| Newest board ordering and reply reordering (two declarations) | Sort by ID only | Credited tests survive; both declarations flagged. |
| Board-post cap | Return 99 instead of 100 | Credited tests survive; declaration flagged. |
| Work-list cap | Return 99 instead of 100 | Credited model comparison rejects. |

All 17 additional gaps, previous mappings, original assertion starts and reasons are in
`plans/ws12-pr215-assertion-audit.json`. The separate ledger retains all 231 names with
source hashes and either real passed mappings or explicit owners/gaps. Its receipt check
verifies execution, while the clause audit addresses the review's coverage issue.

```sh
python3 rust/reference-tools/users/ws12_inventory.py
python3 rust/reference-tools/users/summarize_ws12_tests.py .scratch/pr215-fixes/logs/workspace.log
python3 rust/reference-tools/users/verify_ws12_reconciliation.py --test-log .scratch/pr215-fixes/logs/workspace.log
```

```text
WS12 Rails inventory: 488 declarations; 55 deferred; 8 existing peer tests; 425 ported
```

```text
WS12 workspace totals: 4752 passed; 0 failed; 16 ignored; 61 result summaries
WS12 visible missing-seed notices: 0; intentional missing-seed unit test passed: True
```

```text
WS12 assertion reconciliation: 231 declarations; 176 executed mappings; 55 explicitly flagged; 0 missing or non-running credited tests
```

## Remaining declarations, each with an owner and reason

These are unverified **named assertion sets**, not claims that the installed production
features are absent. Browser behavior remains required; only pixel geometry is excluded.
The 38 former flags remain flagged. Seventeen unsupported credits were added to this list.

| Rails declaration | Owner | Remaining assertion / reason |
|---|---|---|
| `test/controllers/activity_items_controller_test.rb:32` — unread count is isolated by user and ignores read and handled items | WS12 | PR #215 assertion audit: The mapped model matrix checks multiple users and unread/read/handled subsets, but the credited HTTP count request signs in only David; the original second signed-in viewer count assertion is not executed. |
| `test/controllers/activity_items_controller_test.rb:110` — knowing another recipient's item id does not allow opening or changing it | WS12 | PR #215 assertion audit: The corpus checks 404 for the other recipient's read/handled URLs, but it then checks item 8400000000 rather than the foreign target 8400000099; no credited assertion checks the foreign item stayed unread or its open request. |
| `test/controllers/activity_items_controller_test.rb:164` — index resolves the current user's overdue invitations but no one else's | WS12 | The complete merged inbox HTTP corpus runs. This declaration still needs a clause-level projection covering its setup, expiry/recipient scope and state/assertions; an installed reader alone is not named-case proof. |
| `test/controllers/activity_items_controller_test.rb:228` — index expires overdue approvals and drops the decider's badge | WS12 | The complete merged inbox HTTP corpus runs. This declaration still needs a clause-level projection covering its setup, expiry/recipient scope and state/assertions; an installed reader alone is not named-case proof. |
| `test/controllers/activity_items_controller_test.rb:265` — index orders items by recency | WS12 | PR #215 assertion audit: The old corpus keeps ordering timestamps aligned with IDs and does not assert an existing older row moving after touch. |
| `test/controllers/activity_items_controller_test.rb:278` — type filters return only their event types as JSON | WS12 | PR #215 assertion audit: The JSON filter corpus has a subset of event types and no bogus type fallback; it does not execute the original all-event-types/default/bogus assertion set. |
| `test/controllers/activity_items_controller_test.rb:301` — type filters return only their event types as HTML | WS12 | PR #215 assertion audit: The credited HTTP corpus requests type filters only as JSON; its HTML requests are unfiltered. |
| `test/controllers/activity_items_controller_test.rb:317` — type filter survives pagination | WS12 | PR #215 assertion audit: All existing pagination requests are unfiltered; no events-filter cursor, older-link type parameter or follow-up disjointness assertion runs. |
| `test/controllers/activity_items_controller_test.rb:369` — state changes preserve the type filter | WS12 | PR #215 assertion audit: Existing read/handled redirects carry no type input; they cannot assert preservation of type=events or normalization of an invalid type. |
| `test/controllers/agents/mcp_handoff_test.rb:100` — handoff_work shares its throttle bucket with the rest endpoint | WS11-API | Production REST/MCP adapters and broad pinned bytes/state/queue vectors run, but a complete projection of this original declaration has not yet been individually audited; no broad vector is credited as its complete named assertion set. |
| `test/controllers/agents/posts_controller_test.rb:288` — lists the board's open posts newest activity first | WS11-API | PR #215 assertion audit: The credited board REST corpus has one planned post; its shared model cap fixture has planned/blocked posts. The mixed open/done newest-activity output is not asserted. |
| `test/controllers/agents/posts_controller_test.rb:312` — a reply moves the post to the top of the list | WS11-API | PR #215 assertion audit: No credited test writes a reply and then reads a multi-post list to assert reordering. |
| `test/controllers/agents/posts_controller_test.rb:328` — lists by single status, done, and all | WS11-API | PR #215 assertion audit: The REST done-filter probe is empty because there are no done posts; the original positive done/all/open membership assertions do not run. |
| `test/controllers/agents/posts_controller_test.rb:360` — lists by owner me, agents, and id | WS11-API | PR #215 assertion audit: The REST fixture has one agent-owned post and no second agent or human-owned row; negative human filtering is covered, but the complete positive owner selection assertion set is not. |
| `test/controllers/agents/posts_controller_test.rb:395` — caps the list at 100 posts | WS11-API | PR #215 assertion audit: The credited wire fixture has one board post. The shared model cap assertion loads 100 work-list rows, but its board-post query filters to three rows. Neither assertion exercises the board-post 100-row cap. |
| `test/controllers/agents/posts_controller_test.rb:416` — a legacy agent without grants can list but cannot create | WS11-API | PR #215 assertion audit: No credited HTTP test couples a legacy no-grant successful list with its forbidden create request. |
| `test/controllers/agents/posts_controller_test.rb:438` — creation, listing, and event delivery share one work payload | WS11-API | Production REST/MCP adapters and broad pinned bytes/state/queue vectors run, but a complete projection of this original declaration has not yet been individually audited; no broad vector is credited as its complete named assertion set. |
| `test/controllers/agents/work_controller_test.rb:12` — list shows only owned threads newest first with the work fields | WS11-API | PR #215 assertion audit: The wire list has one owned thread; the cap fixture orders same-timestamp rows by ID. Different timestamp newest-first ordering and the original multi-row payload are not asserted. |
| `test/controllers/agents/work_controller_test.rb:63` — list is Bearer-only and empty without owned threads | WS11-API | PR #215 assertion audit: The wire corpus does not issue the successful no-owned-threads list and the non-Bearer list request in this declaration. |
| `test/controllers/agents/work_controller_test.rb:114` — show includes links with pull request, event, and drive entries | WS11-API | Production REST/MCP adapters and broad pinned bytes/state/queue vectors run, but a complete projection of this original declaration has not yet been individually audited; no broad vector is credited as its complete named assertion set. |
| `test/controllers/agents/work_controller_test.rb:158` — list includes links on owned threads | WS11-API | Production REST/MCP adapters and broad pinned bytes/state/queue vectors run, but a complete projection of this original declaration has not yet been individually audited; no broad vector is credited as its complete named assertion set. |
| `test/controllers/agents/work_controller_test.rb:184` — patch changes the status and records the note in work history | WS11-API | PR #215 assertion audit: The work-write corpus asserts persisted note/history facts, but does not request JSON history and served human history after the same agent mutation. |
| `test/controllers/agents/work_delivery_test.rb:14` — assignment appears in event polling with the work payload | WS11-API | Production REST/MCP adapters and broad pinned bytes/state/queue vectors run, but a complete projection of this original declaration has not yet been individually audited; no broad vector is credited as its complete named assertion set. |
| `test/controllers/agents/work_delivery_test.rb:34` — assignment work payload includes links | WS11-API | Production REST/MCP adapters and broad pinned bytes/state/queue vectors run, but a complete projection of this original declaration has not yet been individually audited; no broad vector is credited as its complete named assertion set. |
| `test/controllers/agents/work_delivery_test.rb:51` — unassignment appears in event polling | WS11-API | Production REST/MCP adapters and broad pinned bytes/state/queue vectors run, but a complete projection of this original declaration has not yet been individually audited; no broad vector is credited as its complete named assertion set. |
| `test/controllers/agents/work_delivery_test.rb:64` — ack works on work rows | WS11-API | Production REST/MCP adapters and broad pinned bytes/state/queue vectors run, but a complete projection of this original declaration has not yet been individually audited; no broad vector is credited as its complete named assertion set. |
| `test/controllers/agents/work_delivery_test.rb:75` — assignment enqueues the webhook instead of blocking on it | WS11-API | Production REST/MCP adapters and broad pinned bytes/state/queue vectors run, but a complete projection of this original declaration has not yet been individually audited; no broad vector is credited as its complete named assertion set. |
| `test/controllers/agents/work_delivery_test.rb:97` — assignment posts no webhook without read_messages | WS11-API | Production REST/MCP adapters and broad pinned bytes/state/queue vectors run, but a complete projection of this original declaration has not yet been individually audited; no broad vector is credited as its complete named assertion set. |
| `test/controllers/agents/work_delivery_test.rb:113` — polling omits work rows for rooms the agent lost read_messages in | WS11-API | Production REST/MCP adapters and broad pinned bytes/state/queue vectors run, but a complete projection of this original declaration has not yet been individually audited; no broad vector is credited as its complete named assertion set. |
| `test/controllers/agents/work_delivery_test.rb:128` — polling omits work rows after membership removal | WS11-API | Production REST/MCP adapters and broad pinned bytes/state/queue vectors run, but a complete projection of this original declaration has not yet been individually audited; no broad vector is credited as its complete named assertion set. |
| `test/controllers/agents/work_delivery_test.rb:142` — work events do not count toward the message rate limit | WS11-API | Production REST/MCP adapters and broad pinned bytes/state/queue vectors run, but a complete projection of this original declaration has not yet been individually audited; no broad vector is credited as its complete named assertion set. |
| `test/controllers/agents/work_handoff_test.rb:45` — the receiver polls the handoff with its context package | WS11-API | Production REST/MCP adapters and broad pinned bytes/state/queue vectors run, but a complete projection of this original declaration has not yet been individually audited; no broad vector is credited as its complete named assertion set. |
| `test/controllers/agents/work_handoff_test.rb:64` — the receiver acks the handoff row | WS11-API | Production REST/MCP adapters and broad pinned bytes/state/queue vectors run, but a complete projection of this original declaration has not yet been individually audited; no broad vector is credited as its complete named assertion set. |
| `test/controllers/agents/work_handoff_test.rb:76` — the handoff enqueues the receiver webhook with the package | WS11-API | Production REST/MCP adapters and broad pinned bytes/state/queue vectors run, but a complete projection of this original declaration has not yet been individually audited; no broad vector is credited as its complete named assertion set. |
| `test/controllers/agents/work_handoff_test.rb:176` — handoffs throttle at 60 a minute per credential | WS11-API | Production REST/MCP adapters and broad pinned bytes/state/queue vectors run, but a complete projection of this original declaration has not yet been individually audited; no broad vector is credited as its complete named assertion set. |
| `test/controllers/channel_threads_board_test.rb:215` — the owning agent can change the status but cannot reassign | WS12 | Existing production/models and broader vectors are retained. Complete original assertion clauses still require an audited mapping or new discriminating test. |
| `test/controllers/channel_threads_board_test.rb:232` — the owning agent without manage_threads cannot change the status | WS12 | Existing production/models and broader vectors are retained. Complete original assertion clauses still require an audited mapping or new discriminating test. |
| `test/controllers/channel_threads_board_test.rb:334` — members can reply in a post and join or leave it from the page | WS12 | Existing production/models and broader vectors are retained. Complete original assertion clauses still require an audited mapping or new discriminating test. |
| `test/controllers/channel_threads_board_test.rb:358` — any member can link and unlink but non-members cannot | WS12 | Existing production/models and broader vectors are retained. Complete original assertion clauses still require an audited mapping or new discriminating test. |
| `test/controllers/channel_threads_board_test.rb:546` — a status change replaces the list row and moves the column row | WS12 | Existing production/models and broader vectors are retained. Complete original assertion clauses still require an audited mapping or new discriminating test. |
| `test/controllers/channel_threads_board_test.rb:620` — deleting a channel thread broadcasts no board row remove | WS12 | Existing production/models and broader vectors are retained. Complete original assertion clauses still require an audited mapping or new discriminating test. |
| `test/controllers/channel_threads_board_test.rb:632` — bot posting API returns 422 in a board | WS12 | Existing production/models and broader vectors are retained. Complete original assertion clauses still require an audited mapping or new discriminating test. |
| `test/integration/agent_boards_test.rb:17` — an agent post flows from creation through reply to result | WS11-API | Production REST/MCP adapters and broad pinned bytes/state/queue vectors run, but a complete projection of this original declaration has not yet been individually audited; no broad vector is credited as its complete named assertion set. |
| `test/models/channel_thread_board_test.rb:274` — agent owner filter matches update-work eligibility | WS12 | PR #215 assertion audit: Both credited model tests assert candidates and availability; neither calls the agents owner filter that the declaration asserts. |
| `test/system/activity_inbox_test.rb:13` — handles an item, clears the badge, and receives a later activity | WS11-UI | Server bytes, domain writes and Cable vectors run; this complete named browser sequence has not been re-executed at this checkpoint. Pixel geometry is excluded, browser behavior is not. |
| `test/system/activity_inbox_test.rb:42` — filters by type and saves a notification switch | WS11-UI | Server bytes, domain writes and Cable vectors run; this complete named browser sequence has not been re-executed at this checkpoint. Pixel geometry is excluded, browser behavior is not. |
| `test/system/agent_work_assignment_test.rb:19` — assigns an agent from the update dialog and renders its API status change after refresh | WS11-UI | Server bytes, domain writes and Cable vectors run; this complete named browser sequence has not been re-executed at this checkpoint. Pixel geometry is excluded, browser behavior is not. |
| `test/system/board_automations_test.rb:17` — the board creator configures tag rules and sla timers | WS12 | Server bytes, domain writes and Cable vectors run; this complete named browser sequence has not been re-executed at this checkpoint. Pixel geometry is excluded, browser behavior is not. |
| `test/system/board_automations_test.rb:39` — a plain member sees no automations link and is forbidden directly | WS12 | Server bytes, domain writes and Cable vectors run; this complete named browser sequence has not been re-executed at this checkpoint. Pixel geometry is excluded, browser behavior is not. |
| `test/system/board_automations_test.rb:51` — a tagged post auto-assigns and the history shows it | WS12 | Server bytes, domain writes and Cable vectors run; this complete named browser sequence has not been re-executed at this checkpoint. Pixel geometry is excluded, browser behavior is not. |
| `test/system/board_automations_test.rb:64` — a person hands a post to an agent with context | WS12 | Server bytes, domain writes and Cable vectors run; this complete named browser sequence has not been re-executed at this checkpoint. Pixel geometry is excluded, browser behavior is not. |
| `test/system/board_automations_test.rb:83` — the stale-work digest renders on the board | WS12 | Server bytes, domain writes and Cable vectors run; this complete named browser sequence has not been re-executed at this checkpoint. Pixel geometry is excluded, browser behavior is not. |
| `test/system/boards_test.rb:12` — boards carry posts from creation to result with live rows | WS12 | Server bytes, domain writes and Cable vectors run; this complete named browser sequence has not been re-executed at this checkpoint. Pixel geometry is excluded, browser behavior is not. |
| `test/system/boards_test.rb:86` — a non-member cannot open the board | WS12 | Server bytes, domain writes and Cable vectors run; this complete named browser sequence has not been re-executed at this checkpoint. Pixel geometry is excluded, browser behavior is not. |
| `test/controllers/channel_threads_controller_test.rb:433` — the owner picker lists eligible agents with profiles and excludes ineligible ones | WS12 | Existing production/models and broader vectors are retained. Complete original assertion clauses still require an audited mapping or new discriminating test. |

Additional production integration: **WS12** still needs the generic-recorder adapter
consuming the typed `AgentBudgetNotice` API now supplied by main #214. The owning
reader, bounded preload and three formerly flagged HTML/path/JSON presenter seams
are complete on main; they no longer await WS11. The existing narrow recorder seam
is `recording_recipient_ids(&self, conn: &Connection, notice_id: i64)
-> Result<Option<Vec<i64>>>`. It can delegate to `AgentBudgetNotice::find_by_id` and
`activity_recipient_ids`, preserving None for an absent source. This checkpoint adds
no new feature work beyond the four requested review fixes.

## Final checks and raw summaries

Tests ran from a fresh `git clone --no-hardlinks --single-branch` under
`.scratch/pr215-fixes-clean/source`, at the verified SHA above. Only validated default,
first_run and agents_ui seeds plus the pinned reference environment were copied in.
The three seed verifiers ran with the frozen time 2026-03-02T16:00:00Z and reported
29/0, 4/0 and 40/0 respectively. CI=1 prevents silent missing-seed parity skips.

The gate environment uses CARGO_BUILD_JOBS=2 with the existing rustc throttle;
CARGO_TARGET_DIR is our sole `.scratch/pr215-fixes/target`. Test workers are at most four;
focused repeats use one. Pinned FFmpeg/libvips are loaded from our existing Rails-media
scratch inputs. Cable ports are 53420–53449, mail 53400–53419 and GitHub 53450–53499.
No model server was touched.

Commands, all executed (the wrapper changes to the fresh clone and exports that environment):

```sh
.scratch/pr215-fixes/run-gate.sh cargo test --locked --workspace --no-fail-fast -- --test-threads=4
.scratch/pr215-fixes/run-gate.sh cargo clippy --locked --workspace --all-targets -- -D warnings
.scratch/pr215-fixes/run-gate.sh bash ci/with-release-inputs.sh cargo build --locked -p campfire
.scratch/pr215-fixes/run-gate.sh cargo test --locked -p campfire ws12_ -- --test-threads=1 --nocapture
.scratch/pr215-fixes/run-gate.sh cargo test --locked -p campfire_db ws12_recorder_rechecks_broadcast_recipient_status_between_commits -- --test-threads=1 --nocapture
```

Full workspace raw Cargo summaries (including intentionally ignored tests):

```text
test result: ok. 2661 passed; 0 failed; 7 ignored; 0 measured; 0 filtered out; finished in 1119.78s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.18s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1353 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 158.08s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.12s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.47s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.28s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.30s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.70s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.28s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.91s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.24s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.95s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.85s
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

Strict clippy:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 15s
```

Release inputs only:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 56s
```

Focused application review/coverage receipts:

```text
test controllers::rooms::board_automation_tests::ws12_sla_missing_and_scalar_shapes_return_empty_400_without_writes ... WS12_SLA_REJECT "missing-scalar-0" Rust=400 body_bytes=0 writes=0 Rails=500
WS12_SLA_REJECT "missing-scalar-1" Rust=400 body_bytes=0 writes=0 Rails=500
WS12_SLA_REJECT "missing-scalar-2" Rust=400 body_bytes=0 writes=0 Rails=500
WS12_SLA_REJECT "missing-scalar-3" Rust=400 body_bytes=0 writes=0 Rails=500
WS12_SLA_REJECT "missing-scalar-4" Rust=400 body_bytes=0 writes=0 Rails=500
WS12_SLA_REJECT "missing-scalar-5" Rust=400 body_bytes=0 writes=0 Rails=500
WS12_SLA_REJECT "missing-scalar-6" Rust=400 body_bytes=0 writes=0 Rails=500
WS12_SLA_REJECT "missing-scalar-7" Rust=400 body_bytes=0 writes=0 Rails=500
WS12_SLA_REJECT "missing-scalar-8" Rust=400 body_bytes=0 writes=0 Rails=500
test controllers::ws12_agent_work_query_tests::ws12_agent_work_service_reads_are_reduced_and_flat_at_two_sizes ... WS12_AGENT_WORK_READS rest_create size=5/50 Rust=143/143 Rails=60/60
WS12_AGENT_WORK_READS mcp_create size=5/50 Rust=141/141 Rails=61/61
WS12_AGENT_WORK_READS rest_update size=5/50 Rust=63/63 Rails=33/33
WS12_AGENT_WORK_READS mcp_update size=5/50 Rust=63/63 Rails=34/34
WS12_AGENT_WORK_READS mcp_board_update size=5/50 Rust=63/63 Rails=34/34
WS12_AGENT_WORK_READS rest_result size=5/50 Rust=25/25 Rails=26/26
WS12_AGENT_WORK_READS mcp_result size=5/50 Rust=25/25 Rails=27/27
WS12_AGENT_WORK_READS rest_handoff size=5/50 Rust=68/68 Rails=55/55
WS12_AGENT_WORK_READS mcp_handoff size=5/50 Rust=68/68 Rails=56/56
test controllers::ws12_huddle_copy_tests::ws12_started_and_missed_huddles_assert_the_original_rails_copy ... WS12_HUDDLE_COPY 2 persisted sources; HTML presenter, JSON producer and served inbox DOM agree with Rails
test controllers::ws12_unread_broadcast_tests::ws12_agent_board_creation_reaches_the_rails_unread_stream_over_both_transports ... WS12_UNREAD_STREAM rest_create_queries_5: user_149087659_unreads; complete payload agrees with Rails
WS12_UNREAD_STREAM mcp_create_queries_5: user_149087659_unreads; complete payload agrees with Rails
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 2642 filtered out; finished in 21.20s
```

Final exact Jason/Kevin recipient schedule:

```text
test tests::ws12_recorder_race_test::ws12_recorder_rechecks_broadcast_recipient_status_between_commits ... PR215_RECORDER_RACE later_recipient_active=false persisted_items=1 activity_frames=0 expected_main_rails_frames=0
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1356 filtered out; finished in 0.11s
```

Fresh Rails producers used the existing ws12 Docker runtime (`ws12-reference:boards-b908ebc2`, one CPU), the default seed and injected/frozen time in each producer:

```sh
PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_CPUS=1 rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/users/recorder_broadcast_race.rb
PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_CPUS=1 rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/users/huddle_copy.rb
PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_CPUS=1 rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/boards/unread_streams.rb
PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_CPUS=1 rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/board_automations/review/missing-scalars.rb
```

Fresh Rails producer receipts (no masks):

```text
PR215_RAILS_RECORDER_RACE later_recipient_active=false persisted_items=1 activity_frames=0 second_connection=true
```

```text
WS12_HUDDLE_COPY_RAILS 2 persisted-source comparisons; 0 masks
```

```text
WS12_BOARD_UNREAD_RAILS 1 exact stream/payload comparisons
```

```text
Rails board automation settings oracle: 9 complete responses and rule/tag/audit facts; 0 masks
```

The first fresh-clone workspace (before #212) passed with no failures:

```text
WS12 workspace totals: 4729 passed; 0 failed; 16 ignored; 61 result summaries
WS12 visible missing-seed notices: 0; intentional missing-seed unit test passed: True
```

The initial combined app filter found a fixture-allowlist generation issue (Python treated
false and zero as equal). It was corrected before final gates by deduplicating canonical
JSON representations. It was not a production Rust behavior failure; both inputs remain
separate in the final 35-input list. The final combined focused suite is the credited receipt.

The report/control-tool-only commit changes no Rust code. The mutation tool refuses
restoring old pre-merge backups over clean production inputs, preserving merged changes. The fresh clone has zero tracked changes.
Deleted the sole 33 GiB Cargo target `.scratch/pr215-fixes/target` and the three
40 KiB test-receipt `rust/target` directories in the fresh clones. A final target-directory
scan under our scratch root returns zero. Source fixtures, negative-control receipts
and logs are retained. Push and remote-head equality are checked before the reply.
