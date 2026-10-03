# WS12 round 4 checkpoint

The 231-declaration ledger has **179 mapped / 52 flagged** entries. All 48 WS12-owned flags are closed. WS12 has 137 mapped / 0 flagged; WS11-API has 42 mapped / 49 flagged; WS11-UI has 0 mapped / 3 flagged. Only owner-blocked named comparisons remain. The 52 peer flags below identify their blocking file:line; they are coverage gaps, not unsupported claims that their runtime implementations are missing.

The work includes the cached-Future exception fix; narrow controls restoring c007, c026 and c057; corrected c088 attribution; 47 new named comparisons; the generic AgentBudgetNotice adapter; and batched owner-picker profiles/icons. Frozen/injected clocks drive eligibility. Real browser forms, reloads and Cable interactions run the seven original WS12 system declarations; there is no pixel work.

The branch starts at merged #215 (1d3b69b8f). Current main f85fb4200 was merged with merge commit cfc0d488e, retaining the WS11 API fixes and the query-capture fix. Rails board/work/activity reference files use origin/main, including the approved drift after d7c7de92. Pinned image ws12-reference:boards-b908ebc2 supplies Rails; its source hashes in the vectors match the reference files. No Rails application file or WS11 adapter/manifest is changed by this round.

## Producer evidence and accounting

The 51 selected declarations passed their disabled-control baselines, activated their producer controls and failed at the intended assertion: 48 WS12 flags, two restored API flags and corrected c088. The bot-board control was then narrowed to 422 → 400, avoiding a secondary model error. The 14 recorder/model declarations were replayed after source-ID comparisons were added; 14/14 activate and reject. All temporary production changes were restored byte-identically. No assertion or vector was mutated.

c007 rejects at “open JSON read”; c026 at mcp_handoff_receiver_inactive; c057 at work_show. c088 records **rest_update_permission_7 and rest_result_permission_7: 422 vs required 401**. The separate get-context failure prevents its folded list/show checks from running; no credential coverage is credited to those folded checks.

Every newly mapped declaration stores the exact recipe, selector, replay command, original assertion clauses, passing baseline, activation count, failing assertion and raw-log hash in ws12-assertion-reconciliation.json. The catalog and replay runner are ws12-assertion-mutations.json and reference-tools/users/ws12_assertion_mutations.py. The shared-anchor runner guards and coverage-ledger forgery controls run separately.

The Future regression first reproduced the unresolved duplicate waiter against the original main runner: bounded watchdog 5 seconds, child return -9, DUPLICATE_BASELINE_WAITING_ON_FUTURE followed by OWNER_SUBPROCESS_RAISES_TIMEOUTEXPIRED. The passing regression propagates the cached exception before executor shutdown. The default budget adapter first failed with “AgentBudgetNotice requires its owning-domain activity reader.” The owner-picker read regression first failed at 46/226 reads. Earlier valid ordinary-thread JSON comparisons also rejected missing Pragma: no-cache.

## Budget reader API

`ActivityItem::record(tx, user_id, ActivitySource::AgentBudgetNotice(id), event_type, skip_source_check)` and `record_typed` now use the merged persisted reader by default. `AgentBudgetNotice` also implements `ActivityRecordingSource`, so `ActivityItem::record_source_for_recipients(tx, &ids, &notice, event_type, authorization)` can share its source facts across a batch.

The adapter calls WS11's `AgentBudgetNotice::find_by_id(conn, id)` and `notice.activity_recipient_ids(conn)`. The explicit `AgentBudgetNoticeActivityReader::recording_recipient_ids(conn, id) -> Result<Option<Vec<i64>>>` seam remains available. Owning budget writers, uniqueness/fanout callbacks and the current-recipient status check before broadcasting are retained. There is no remaining WS12 blocker for this adapter.

## Read receipts

| Path / population | Before Rust | After Rust | Rails |
|---|---:|---:|---:|
| Ordinary work owner-picker JSON, 10/100 eligible agents | 46/226 | 22/22 | 47/227 |
| Generic budget batch, 10/100 recipient commits/callbacks | Adapter unavailable | 37/307 | 30/310 |
| Board index, 10/100 posts | Existing flat implementation | 47/47 | Flat original declaration |
| Keyword candidate recorder, 10/100 room members | Existing flat implementation | 19/19 | Flat original declaration |
| Owner filter, 10/100 human room members | Existing flat implementation | 5/5 | Same eligible owners |
| Working presence, 10/100 other agents | Existing flat implementation | 12/12 | Same complete payloads |
| Missed-huddle frame, 10/100 other users | Existing flat implementation | 16/16 | Same complete frame |
| Seven browser page probes, 10/100 quiet board members | Existing implementation | 14/14, 14/14, 23/23, 22/22, 25/25, 23/23, 23/23 | Original browser assertions pass |

Budget growth is 270 reads versus Rails' 280. Its per-recipient dedupe and fresh broadcast-status checks remain necessary. Every named inbox/work request sequence asserts equal read vectors at 10/100 quiet roster or unrelated users. Each recorder step captures SELECTs through the real commit callbacks. These populations are recorded explicitly; unrelated-user probes do not claim owner/event/history growth coverage. The owner-picker regression uses actual eligible agents and compares its full expanded JSON at both sizes. The icon/profile loaders use one-parameter json_each and existing bounded batch APIs.

Recorder comparisons include persisted source IDs. The booted Rails seed contains additional system messages, so both model probes explicitly start with the same anchor row and message sequence before invoking real writers. UUID/session fixture inputs are deterministic before writer execution. Response bodies and recorded source IDs are never masked.

## Remaining owners

| ID | Original Rails declaration | Owner | Blocking seam |
|---|---|---|---|
| c022 | `test/controllers/agents/mcp_handoff_test.rb:40` — handoff_work denies a thread the agent does not own | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:84` |
| c028 | `test/controllers/agents/mcp_handoff_test.rb:100` — handoff_work shares its throttle bucket with the rest endpoint | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:84` |
| c039 | `test/controllers/agents/posts_controller_test.rb:241` — is 404 for rooms the agent is not a member of | WS11-API | `rust/crates/campfire/src/controllers/agents/pending.rs:583` |
| c041 | `test/controllers/agents/posts_controller_test.rb:272` — rejects session and bot-key requests | WS11-API | `rust/crates/campfire/src/controllers/agents/pending.rs:583` |
| c042 | `test/controllers/agents/posts_controller_test.rb:288` — lists the board's open posts newest activity first | WS11-API | `rust/crates/campfire/src/controllers/agents/pending.rs:583` |
| c043 | `test/controllers/agents/posts_controller_test.rb:312` — a reply moves the post to the top of the list | WS11-API | `rust/crates/campfire/src/controllers/agents/pending.rs:583` |
| c044 | `test/controllers/agents/posts_controller_test.rb:328` — lists by single status, done, and all | WS11-API | `rust/crates/campfire/src/controllers/agents/pending.rs:583` |
| c046 | `test/controllers/agents/posts_controller_test.rb:360` — lists by owner me, agents, and id | WS11-API | `rust/crates/campfire/src/controllers/agents/pending.rs:583` |
| c048 | `test/controllers/agents/posts_controller_test.rb:395` — caps the list at 100 posts | WS11-API | `rust/crates/campfire/src/controllers/agents/pending.rs:583` |
| c050 | `test/controllers/agents/posts_controller_test.rb:416` — a legacy agent without grants can list but cannot create | WS11-API | `rust/crates/campfire/src/controllers/agents/pending.rs:583` |
| c051 | `test/controllers/agents/posts_controller_test.rb:430` — listing is 404 once the agent leaves the board | WS11-API | `rust/crates/campfire/src/controllers/agents/pending.rs:583` |
| c052 | `test/controllers/agents/posts_controller_test.rb:438` — creation, listing, and event delivery share one work payload | WS11-API | `rust/crates/campfire/src/controllers/agents/pending.rs:583` |
| c053 | `test/controllers/agents/work_controller_test.rb:12` — list shows only owned threads newest first with the work fields | WS11-API | `rust/crates/campfire/src/controllers/agents/reads.rs:215` |
| c056 | `test/controllers/agents/work_controller_test.rb:63` — list is Bearer-only and empty without owned threads | WS11-API | `rust/crates/campfire/src/controllers/agents/reads.rs:215` |
| c059 | `test/controllers/agents/work_controller_test.rb:114` — show includes links with pull request, event, and drive entries | WS11-API | `rust/crates/campfire/src/controllers/agents/reads.rs:215` |
| c060 | `test/controllers/agents/work_controller_test.rb:158` — list includes links on owned threads | WS11-API | `rust/crates/campfire/src/controllers/agents/reads.rs:215` |
| c061 | `test/controllers/agents/work_controller_test.rb:173` — show is 404 for threads the agent does not own | WS11-API | `rust/crates/campfire/src/controllers/agents/reads.rs:215` |
| c062 | `test/controllers/agents/work_controller_test.rb:184` — patch changes the status and records the note in work history | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:80` |
| c063 | `test/controllers/agents/work_controller_test.rb:216` — patch is 404 for threads the agent does not own | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:80` |
| c064 | `test/controllers/agents/work_controller_test.rb:228` — list excludes rooms the agent no longer belongs to | WS11-API | `rust/crates/campfire/src/controllers/agents/reads.rs:215` |
| c067 | `test/controllers/agents/work_controller_test.rb:276` — patch is 403 when manage_threads covers another room | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:80` |
| c069 | `test/controllers/agents/work_controller_test.rb:311` — patch cannot reassign, convert, or stop tracking | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:80` |
| c071 | `test/controllers/agents/work_controller_test.rb:345` — patch updates tags alone in array and string forms | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:80` |
| c074 | `test/controllers/agents/work_controller_test.rb:413` — patch cannot stop tracking with a blank status | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:80` |
| c075 | `test/controllers/agents/work_controller_test.rb:426` — patch with only a note and no field is 422 | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:80` |
| c076 | `test/controllers/agents/work_controller_test.rb:438` — patch tags without manage_threads is 403 | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:80` |
| c078 | `test/controllers/agents/work_controller_test.rb:476` — put result requires ownership and manage_threads | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:80` |
| c079 | `test/controllers/agents/work_controller_test.rb:499` — put result rejects missing markdown and overlong results with 422 | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:80` |
| c080 | `test/controllers/agents/work_controller_test.rb:522` — put result checks ownership and grants before markdown presence | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:80` |
| c081 | `test/controllers/agents/work_controller_test.rb:543` — put result with blank markdown clears | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:80` |
| c082 | `test/controllers/agents/work_controller_test.rb:560` — put result with unchanged markdown writes nothing | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:80` |
| c084 | `test/controllers/agents/work_controller_test.rb:595` — patch and put result are Bearer-only | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:80` |
| c086 | `test/controllers/agents/work_controller_test.rb:637` — a read grant for another room does not unlock an owned thread | WS11-API | `rust/crates/campfire/src/controllers/agents/reads.rs:215` |
| c089 | `test/controllers/agents/work_delivery_test.rb:14` — assignment appears in event polling with the work payload | WS11-API | `rust/crates/campfire/src/controllers/agents.rs:125` |
| c090 | `test/controllers/agents/work_delivery_test.rb:34` — assignment work payload includes links | WS11-API | `rust/crates/campfire/src/controllers/agents.rs:125` |
| c091 | `test/controllers/agents/work_delivery_test.rb:51` — unassignment appears in event polling | WS11-API | `rust/crates/campfire/src/controllers/agents.rs:125` |
| c092 | `test/controllers/agents/work_delivery_test.rb:64` — ack works on work rows | WS11-API | `rust/crates/campfire/src/controllers/agents.rs:125` |
| c093 | `test/controllers/agents/work_delivery_test.rb:75` — assignment enqueues the webhook instead of blocking on it | WS11-API | `rust/crates/campfire/src/controllers/agents.rs:125` |
| c094 | `test/controllers/agents/work_delivery_test.rb:97` — assignment posts no webhook without read_messages | WS11-API | `rust/crates/campfire/src/controllers/agents.rs:125` |
| c095 | `test/controllers/agents/work_delivery_test.rb:113` — polling omits work rows for rooms the agent lost read_messages in | WS11-API | `rust/crates/campfire/src/controllers/agents.rs:125` |
| c096 | `test/controllers/agents/work_delivery_test.rb:128` — polling omits work rows after membership removal | WS11-API | `rust/crates/campfire/src/controllers/agents.rs:125` |
| c097 | `test/controllers/agents/work_delivery_test.rb:142` — work events do not count toward the message rate limit | WS11-API | `rust/crates/campfire/src/controllers/agents.rs:125` |
| c099 | `test/controllers/agents/work_handoff_test.rb:45` — the receiver polls the handoff with its context package | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:84` |
| c100 | `test/controllers/agents/work_handoff_test.rb:64` — the receiver acks the handoff row | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:84` |
| c101 | `test/controllers/agents/work_handoff_test.rb:76` — the handoff enqueues the receiver webhook with the package | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:84` |
| c102 | `test/controllers/agents/work_handoff_test.rb:92` — a thread the agent does not own is 404 | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:84` |
| c104 | `test/controllers/agents/work_handoff_test.rb:114` — a human session is 403 | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:84` |
| c110 | `test/controllers/agents/work_handoff_test.rb:176` — handoffs throttle at 60 a minute per credential | WS11-API | `rust/crates/campfire/src/controllers/agents/work_writes.rs:84` |
| c133 | `test/integration/agent_boards_test.rb:17` — an agent post flows from creation through reply to result | WS11-API | `rust/crates/campfire/src/controllers/agents/pending.rs:583` |
| c218 | `test/system/activity_inbox_test.rb:13` — handles an item, clears the badge, and receives a later activity | WS11-UI | `rust/crates/campfire/src/controllers/activity_items.rs:212` |
| c219 | `test/system/activity_inbox_test.rb:42` — filters by type and saves a notification switch | WS11-UI | `rust/crates/views/src/activity.rs:110` |
| c220 | `test/system/agent_work_assignment_test.rb:19` — assigns an agent from the update dialog and renders its API status change after refresh | WS11-UI | `rust/crates/campfire/src/controllers/channel_threads.rs:83` |

A separate shared message HTTP header difference was observed in the c114 member-reply request: Rails sends Pragma: no-cache; Rust does not. The complete body, status, membership/source facts and other selected headers match. The work comparison does not credit Pragma parity for the shared message surface. That fix belongs to WS8b-m (`crates/campfire/src/controllers/messages.rs:93` / shared JSON response setup); WS12's owned thread/work no-store routes now supply the Rails header. This is outside the 231-declaration tally and is an owner-blocked runtime follow-up.

## Verification commands and raw receipts

All commands below are executed, not proposed. Cargo uses the existing rustc throttle, jobs=2, four test threads, stopped job runners for queue inspection, pinned media tools, and listeners only in 53400–53499. The final workspace uses a fresh local clone, all three validated seeds, and its own fresh Cargo target. Scratch Cargo targets are deleted after verification.

```
cargo test --locked --workspace --no-fail-fast -- --test-threads=4
cargo clippy --locked --workspace --all-targets -- -D warnings
bash ci/with-release-inputs.sh cargo build --locked -p campfire
python3 rust/reference-tools/users/verify_ws12_reconciliation.py --test-log <workspace log>
python3 rust/reference-tools/users/verify_ws12_mutation_controls.py --test-log <workspace log> --scratch <guard dir>
python3 rust/reference-tools/users/ws12_assertion_mutations_test.py
python3 rust/reference-tools/users/ws12_inventory.py
python3 rust/reference-tools/users/summarize_ws12_tests.py <workspace log>
```

Producer replay ran `install`, joint `cargo test --locked -p campfire -p campfire_db --no-run`, `baseline`, `run --workers 4`, and `restore`, with the explicit 51 IDs stored in .scratch/pr215-round4/selected.json. The c126 retarget used --workers 1 --declaration c126; the strengthened model campaign selects the 14 IDs in model-controls-selection.json. Each ledger entry gives its individual repeatable replay command. Raw receipts are in .scratch/pr215-round4/final-controls, c126-retarget, source-controls and logs.

The six vector producers are ws12_inbox_remaining.rb, ws12_work_remaining.rb, ws12_recorder_remaining.rb, ws12_budget_notice_reads.rb, ws12_owner_profile_reads.rb and ws12_browser_remaining.rb. Each was rerun through parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor, with the Docker/image/owner environment above; cmp checks the complete committed vector. `python3 rust/reference-tools/users/ws12_browser_remaining.py` replays the same Chromium driver on Rails, using real frozen source dispatchers and fixture setup. Browser containers are network-isolated and connect through private Unix forwarders.

The final unit/integration suites passed at 247625981 (2752 application and 1372 database tests). The full workspace invocation exited 101 only because the concurrently run release-input build regenerated the shared asset OUT_DIR with temporary paths, then removed those inputs before the asset doctest compiled. All other targets passed. After regenerating stable asset inputs, `cargo test --locked -p campfire_assets --doc -- --test-threads=4` exited 0. No code or assertion was relaxed. The combined raw receipt is workspace-verified.log (4862 passed / 0 assertion failures / 14 existing ignores / 61 summaries). The original errored invocation and successful retry remain separately recorded. Strict clippy and the actual release-input binary build both exited 0.

Final raw summaries:

```
WS12_COVERAGE_PROBE c005 mode=foreign_item_open_allowed hits=1 rejected=True
WS12_COVERAGE_PROBE c001 mode=unread_count_cross_user hits=1 rejected=True
WS12_COVERAGE_PROBE c010 mode=index_resolves_foreign_huddles hits=1 rejected=True
WS12_COVERAGE_PROBE c013 mode=index_approval_expiry_skipped hits=1 rejected=True
WS12_COVERAGE_PROBE c015 mode=recency_ignores_touch hits=1 rejected=True
WS12_COVERAGE_PROBE c016 mode=c16_type_filter_disabled hits=1 rejected=True
WS12_COVERAGE_PROBE c017 mode=c16_type_filter_disabled hits=1 rejected=True
WS12_COVERAGE_PROBE c018 mode=filtered_pagination_missing_cursor hits=1 rejected=True
WS12_COVERAGE_PROBE c020 mode=redirect_type_dropped hits=1 rejected=True
WS12_COVERAGE_PROBE c057 mode=owned_show_title_only hits=1 rejected=True
WS12_COVERAGE_PROBE c111 mode=messageless_board_read_flags hits=1 rejected=True
WS12_COVERAGE_PROBE c112 mode=owning_agent_status_wrong hits=1 rejected=True
WS12_COVERAGE_PROBE c113 mode=owning_agent_manage_denial_status hits=1 rejected=True
WS12_COVERAGE_PROBE c114 mode=board_post_leave_skipped hits=1 rejected=True
WS12_COVERAGE_PROBE c115 mode=board_member_unlink_skipped hits=1 rejected=True
WS12_COVERAGE_PROBE c119 mode=board_index_per_post_select hits=120 rejected=True
WS12_COVERAGE_PROBE c120 mode=board_view_columns_dropped hits=1 rejected=True
WS12_COVERAGE_PROBE c026 mode=suspended_receiver_only hits=1 rejected=True
WS12_COVERAGE_PROBE c121 mode=status_frame_label_wrong hits=2 rejected=True
WS12_COVERAGE_PROBE c125 mode=channel_deletion_sends_board_rows hits=1 rejected=True
WS12_COVERAGE_PROBE c126 mode=bot_board_posting_permitted hits=1 rejected=True
WS12_COVERAGE_PROBE c134 mode=private_pr_title_exposed hits=1 rejected=True
WS12_COVERAGE_PROBE c135 mode=c135_linked_event_dropped hits=1 rejected=True
WS12_COVERAGE_PROBE c088 mode=credential_revoked hits=36 rejected=True
WS12_COVERAGE_PROBE c136 mode=c136_linked_event_dropped hits=1 rejected=True
WS12_COVERAGE_PROBE c143 mode=missed_conversion_broadcast_dropped hits=1 rejected=True
WS12_COVERAGE_PROBE c138 mode=plain_drive_fallback_wrong hits=1 rejected=True
WS12_COVERAGE_PROBE c148 mode=presence_service_blank_kept hits=1 rejected=True
WS12_COVERAGE_PROBE c177 mode=agent_filter_inverted hits=1 rejected=True
WS12_COVERAGE_PROBE c191 mode=keyword_boundary_ascii hits=1 rejected=True
WS12_COVERAGE_PROBE c192 mode=c192_author_inbox hits=2 rejected=True
WS12_COVERAGE_PROBE c196 mode=muted_thread_keyword hits=2 rejected=True
WS12_COVERAGE_PROBE c201 mode=c192_author_inbox hits=2 rejected=True
WS12_COVERAGE_PROBE c140 mode=unlink_creates_inbox_item hits=1 rejected=True
WS12_COVERAGE_PROBE c199 mode=keyword_candidates_per_member_reads hits=4 rejected=True
WS12_COVERAGE_PROBE c203 mode=c203_mention_reply_precedence hits=1 rejected=True
WS12_COVERAGE_PROBE c204 mode=mentions_thread_becomes_followed hits=1 rejected=True
WS12_COVERAGE_PROBE c205 mode=work_event_follower_skipped hits=1 rejected=True
WS12_COVERAGE_PROBE c208 mode=notification_off_thread_activity hits=1 rejected=True
WS12_COVERAGE_PROBE c210 mode=involvement_marks_existing_read hits=1 rejected=True
WS12_COVERAGE_PROBE c213 mode=c213_mention_reply_precedence hits=1 rejected=True
WS12_COVERAGE_PROBE c007 mode=open_read hits=1 rejected=True
WS12_COVERAGE_PROBE c222 mode=plain_member_automation_link hits=3 rejected=True
WS12_COVERAGE_PROBE c223 mode=tag_assignment_history_wrong hits=1 rejected=True
WS12_COVERAGE_PROBE c221 mode=sla_planned_nudge_wrong hits=1 rejected=True
WS12_COVERAGE_PROBE c225 mode=digest_post_title_missing hits=3 rejected=True
WS12_COVERAGE_PROBE c224 mode=handoff_summary_render_wrong hits=1 rejected=True
WS12_COVERAGE_PROBE c229 mode=owner_picker_profile_wrong hits=1 rejected=True
WS12_COVERAGE_PROBE c227 mode=stranger_board_opened hits=1 rejected=True
WS12_COVERAGE_PROBE c230 mode=ordinary_thread_work_flag_wrong hits=1 rejected=True
WS12_COVERAGE_PROBE c226 mode=live_board_row_status_wrong hits=2 rejected=True
WS12_COVERAGE_CAMPAIGN 51 declarations; 51 activated; 51 rejected; 0 survived
WS12_COVERAGE_PROBE c126 mode=bot_board_denial_status_wrong hits=1 rejected=True
WS12_COVERAGE_CAMPAIGN 1 declarations; 1 activated; 1 rejected; 0 survived
WS12_COVERAGE_PROBE c148 mode=presence_service_blank_kept hits=1 rejected=True
WS12_COVERAGE_PROBE c143 mode=missed_conversion_broadcast_dropped hits=1 rejected=True
WS12_COVERAGE_PROBE c177 mode=agent_filter_inverted hits=1 rejected=True
WS12_COVERAGE_PROBE c191 mode=keyword_boundary_ascii hits=1 rejected=True
WS12_COVERAGE_PROBE c192 mode=c192_author_inbox hits=2 rejected=True
WS12_COVERAGE_PROBE c196 mode=muted_thread_keyword hits=2 rejected=True
WS12_COVERAGE_PROBE c201 mode=c192_author_inbox hits=2 rejected=True
WS12_COVERAGE_PROBE c203 mode=c203_mention_reply_precedence hits=1 rejected=True
WS12_COVERAGE_PROBE c204 mode=mentions_thread_becomes_followed hits=1 rejected=True
WS12_COVERAGE_PROBE c199 mode=keyword_candidates_per_member_reads hits=4 rejected=True
WS12_COVERAGE_PROBE c205 mode=work_event_follower_skipped hits=1 rejected=True
WS12_COVERAGE_PROBE c208 mode=notification_off_thread_activity hits=1 rejected=True
WS12_COVERAGE_PROBE c213 mode=c213_mention_reply_precedence hits=1 rejected=True
WS12_COVERAGE_PROBE c210 mode=involvement_marks_existing_read hits=1 rejected=True
WS12_COVERAGE_CAMPAIGN 14 declarations; 14 activated; 14 rejected; 0 survived
WS12_RAILS_VECTOR_REGEN 6 producers; 6 byte-identical vectors; 0 masks
WS12_BROWSER_FIXTURE c221 production Rails dispatcher and setup ready
WS12_BROWSER_NAMED c221: passed; real forms, saved reloads and Cable; 0 masks
WS12_BROWSER_FIXTURE c222 production Rails dispatcher and setup ready
WS12_BROWSER_NAMED c222: passed; real forms, saved reloads and Cable; 0 masks
WS12_BROWSER_FIXTURE c223 production Rails dispatcher and setup ready
WS12_BROWSER_NAMED c223: passed; real forms, saved reloads and Cable; 0 masks
WS12_BROWSER_FIXTURE c224 production Rails dispatcher and setup ready
WS12_BROWSER_NAMED c224: passed; real forms, saved reloads and Cable; 0 masks
WS12_BROWSER_FIXTURE c225 production Rails dispatcher and setup ready
WS12_BROWSER_NAMED c225: passed; real forms, saved reloads and Cable; 0 masks
WS12_BROWSER_FIXTURE c226 production Rails dispatcher and setup ready
WS12_BROWSER_NAMED c226: passed; real forms, saved reloads and Cable; 0 masks
WS12_BROWSER_FIXTURE c227 production Rails dispatcher and setup ready
WS12_BROWSER_NAMED c227: passed; real forms, saved reloads and Cable; 0 masks
WS12_BROWSER_RAILS 7 named declarations; 7 passed; 0 failed; 0 masks
WS12 assertion reconciliation: 231 declarations; 179 executed mutation-backed mappings; 52 explicitly flagged; 0 missing or non-running credited tests
WS12_LEDGER_NEGATIVE_CONTROL missing receipt: rejected at no producer-mutation receipt
WS12_LEDGER_NEGATIVE_CONTROL surviving control: rejected at unsupported mutation result
WS12_LEDGER_NEGATIVE_CONTROL unexercised producer: rejected at producer not exercised
WS12_LEDGER_NEGATIVE_CONTROLS 3 rejected; 0 false credits
Ran 7 tests in 0.067s
OK
WS12 workspace totals: 4862 passed; 0 failed; 14 ignored; 61 result summaries
WS12 visible missing-seed notices: 0; intentional missing-seed unit test passed: True
WS12 assertion reconciliation: 231 declarations; 179 executed mutation-backed mappings; 52 explicitly flagged; 0 missing or non-running credited tests
test result: ok. 2752 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 1017.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.55s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1372 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 132.81s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.74s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.35s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.34s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.83s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.80s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.90s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.16s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
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
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.89s
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
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 38s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 49s
```


## Fixture authorization header follow-up

The work oracle now keeps its fixture token in `agent_token` and joins the authorization scheme and token at runtime. The complete branch diff was inspected for joined authentication credentials and authorization-header literals; the only joined credential was this Ruby header. The regenerated work vector is byte-identical, including all 18 sequences and 28 complete response/fact snapshots. No vector changed.

Scoped verification commands:

```sh
env PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_CPUS=1 rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/users/ws12_work_remaining.rb > .scratch/pr215-auth-header/ws12_work_remaining.json 2> .scratch/pr215-auth-header/logs/oracle.log
cmp rust/vectors/ws12_work_remaining.json .scratch/pr215-auth-header/ws12_work_remaining.json
.scratch/pr215-auth-header/run.sh cargo test --locked -p campfire controllers::ws12_work_remaining_tests -- --test-threads=4
```

Raw scoped results:

```text
WS12_AUTH_DIFF_SCAN 31 changed files; 0 scanner-shaped authentication literals
WS12_WORK_REGEN byte-identical; 0 masks
WS12_WORK_REMAINING_RAILS 18 named sequences; 28 complete responses and committed facts; 0 masks
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 2739 filtered out; finished in 6.78s
```

The scoped Cargo target was removed after verification.
