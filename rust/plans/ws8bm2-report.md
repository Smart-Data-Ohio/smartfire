# WS8bm2 message features F — pin/poll, shared dates, named commands and queued-job checkpoint

Branch `rust/ws8bm2-message-features-f` is stacked on E `16d00231e915e4760b666043bbac69096a8e362d`. This worker did not change or rebase E; its owner subsequently merged main at `1584aef48` before #207 landed. Recovery WIP `a48e8a4ab` was committed without pushing, as requested. The owned process/container inspection found no leftover Rails runner, test binary or container to stop. No OOM cause is claimed for the earlier SIGKILL. No stash, Python model-server operation, deployment or PR creation occurred.

Source commits: `1dc950735` pin/poll batching; `956db8b3c` merges #196; `a48e8a4ab` recovery WIP; `762c14da6` shared parser/integrations/runtime proofs; `a1549ea32` merges main #205; `3ebce2f00` test-only strict-clippy cleanup; `437f53c4e` request-zone DND/OOO durations and exact queue-observer exclusion. Checkpoint merge `a4132b9c1` includes main #207 `12b812796` and has a tracked tree identical to `3ebce2f00`. Main advanced during verification; merge `798e4c0d50395077ae9af4187256bff32d5c9ea5` includes reviewed WS11 #210 `3ea778569`, without conflicts. Locked metadata passed after every main merge. `07fec984c` preserves UTC service dispatch separately from request-zone dispatch. Final executable/test/vector source is `07fec984c1353e4692c7ea106263410c4af252fe`. The final documentation-only commit and pushed SHA are in the reply.

This is a coherent PR checkpoint, **partial for cutover and not owner-blocked-only**. The remaining list below separates incomplete evidence from the deliberately held owner reader. Date work reused the merged #196 parser; no second Date._parse parser was built.

## Complete in F

- Pin lists batch messages, pinners/authors and first rich-text bodies through the actual controller/presenter path. Ordinary and populated pins match 12 exact Rails partials; physical reads are 8 at both 4 and 16 pins, compared with Rails 13/25 and the pre-fix Rust 18/66. Message/blob ID readers use one JSON bind. A limit-8 SQLite regression exercises the pin controller and 32,768 repeated message IDs without exceeding the variable limit.
- Standalone named/anonymous poll cards have their own physical SQL proof. A single ordered vote/user join preserves vote order and optional names. Rust reads 5/5 at 4/16 voters; Rails named cards read 9/21 and anonymous cards 5/5. Anonymous output remains byte-identical and exposes no voter IDs.
- #196's unchanged Date._parse grammar and zone-name TSV are promoted into `rails_compat`; Time.zone.parse assembly and the pinned TZInfo boundaries live in db and serve both WS8 and the existing WS11 bot casts. The original parser file is moved, not duplicated; the grammar rename is 99% identical, with public visibility and formatting changes only. #179 viewer-zone rendering is unchanged. The two callers retain their different rescue rules.
- 315 Calendar and 315 slash result/error comparisons cover five zones (UTC, New York, Chicago, Lord Howe, Apia), compact/ordinal/week/JIS/AD/BC forms, offsets/fractions/leap seconds, invalid and wide numeric components, and nine gap/fold/skip-day combinations with and without an explicit offset. The 64 actual HTTP comparisons cover array/hash/null/bool/integer inputs, ignored multiparameter fields, structured pager/link/file inputs and exact exception status/type/body. Direct service duration calls retain the ambient UTC behavior pinned by the original WS8 domain vectors; the HTTP adapter explicitly uses the invoker zone set by Rails' `SetTimeZone` callback. Reminders and `/dnd` propagate RangeError as Rails does; scheduling preserves the ArgumentError message on create and update while propagating other errors. Eight real `/dnd` and `/ooo` duration requests pin persisted expiry times in UTC, New York, Chicago and Lord Howe; day/week durations now preserve the request-zone civil clock across spring and half-hour autumn transitions.
- All 35 named built-in DispatcherTest comparisons have separate WS8 test names and 63 pinned observations. Built-in message posts now call the real attachment adapter once inside the writer. `/ooo` uses the real owner `UserStatusSettings::announce_ooo` badge/notice API; its two served WebSocket frames match Rails exactly. Existing legacy webhook and board/thread behavior is retained. The WS11 case-ports ledger was not edited.
- 26 Calendar InboundSync/SyncEntry parent jobs execute the owner consumers over real encrypted account rows and a whitelist-only recorded Google client. Four queued SyncEntry children actually run after their declining parent. Exchanges, attendance, entry existence and silent old-reference streams match Rails at 4/16 references. Covered outcomes are confirmed/cancelled/404/403/503, missing/disconnected account, departed user, locally declined attendance, update, POST-409-to-PUT convergence and deleted event. HTTP 503 is a permanent owner error here, not a retry proof.
- 20 generic/LinkedIn parent network jobs actually execute their 20 queued stale children through real local TLS/HTTP. All 400 frame strings, outbound calls, positive/negative cache state and TTLs match Rails. Parents and children each use 10 reads at 4 and 16 references. Opposite-provider and suppressed-only siblings remain unclaimed; eight outer/savepoint rollbacks remain silent and preserve metadata/claims/jobs.
- 12 additional real queued jobs cover four deletions before dispatch, four deletions between GET and the writer, and four fatal SQLite write rejections. A response gate, not a sleep, establishes the interleaving. Missing records are discarded and acknowledged; failed writes leave the exact parent/sibling rows unchanged, publish no frames and queue no children. Rails and Rust queue storage differ, so this compares terminal outcome and effects, not backend row bytes.
- Both formerly ignored Drive `/agents/events` HTTP polling tests are enabled and pass. The owner's Rails fixture independently replayed byte-identically; populated/empty Drive projections carry only file IDs/URLs. Served omission of the projection makes both checks fail.

## Changes and ownership

| Files (under `rust/crates/`; controller/integration paths omit `campfire/src/`) | Change |
| --- | --- |
| `controllers/rooms/pins.rs`, `controllers/presenters.rs`, `controllers/searches/preloads.rs`, `db/src/models/message_rendering.rs` | Minimal batched plaintext facts for pin rendering, preserving the first-body selection and production request cost. |
| `db/src/models/message.rs`, `storage/src/blob.rs`, `db/src/models/poll.rs`, `controllers/message_features.rs` | JSON-bound ID lookups and joined ordered standalone vote names; checked parser boundary. |
| `rails_compat/{Cargo.toml,src/date_parse.rs,src/date_zones.tsv,src/lib.rs}`, `rust/Cargo.lock` | Promote the single WS11 grammar and add its existing bnum dependency. |
| `db/src/slash_commands/{calendar.rs,time_parser.rs,date_zone_boundaries.json}`, `controllers/accounts/bots.rs`, `accounts/bots/input_casts.rs` | Share assembly/TZ boundaries and preserve the WS11 UI rescue adapter. |
| `controllers/{saved_items.rs,scheduled_messages.rs,rooms/slash_commands.rs}`, `db/src/slash_commands.rs` | Exception class/message parity, real post-attachment integration and owner OOO callbacks. |
| `db/src/tests/slash_commands_test.rs` | Decode the owner's typed badge/notice intents to the unchanged pinned logical callback descriptors. |
| `controllers/message_features/{pin_poll_scaling,exceptional_input,slash_named,older_calendar_execution,older_embed_children,older_embed_failure}_tests.rs` | Physical query, exact Rails, real socket/queue and negative-control proof. |
| `controllers/message_features/quote_integration_tests.rs`, `integrations/test_support.rs`, `integrations/agent_jobs/drive_attachment_cases.rs` | Fixture row import, fresh-cookie socket helper, deterministic response gate and re-enabled HTTP checks. |
| `rust/reference-tools/messaging/{pin_poll_scaling,exceptional_inputs,slash_named,older_calendar_execution,older_embed_children,older_embed_failures}.rb`, corresponding `rust/vectors/messaging/*.json`, `verify_oracles.py` | Six committed corpora and serial independent replay of all 45 owned fixtures. |

Production provider/job-owner files used in mutations were restored byte-for-byte. No own commit touches `presenters/boards.rs`, `channel_thread/board.rs`, `channel_thread/work.rs`, `board_posts.rs`, `work_threads.rs` or `reference-tools/agents/case-ports.json`. Their main changes arrive only through reviewed merges. Existing physical push, agents and huddles remain integrated. The typed AgentBudgetNotice reader seam is deliberately unchanged.

## Physical reads

Counts are executed SELECT/WITH statements. Pin/poll capture invokes the production controller list helper and standalone poll presenter, including presenter construction and detached rendering. Authentication and application-layout reads are outside these partial comparisons on both sides. Embed capture includes consumer reads, its writer and synchronous callback rendering. Calendar capture wraps the actual owner consumers; **only the exact `SELECT job_class,status,attempts,last_error ...` query from the test's queue observer is excluded**. Polling/acknowledgement occurs outside consumer work. All domain/cache/association reads remain counted. The first Calendar counter included that observer and varied 20/22; that was a harness error, not an app N+1 finding.

| Path | Rust before 4 → 16 | Rust final 4 → 16 | Rails 4 → 16 |
| --- | --- | --- | --- |
| Ordinary pin list | 18 → 66 | 8 → 8 | 13 → 25 |
| Populated pin list | Not measured before | 8 → 8 | 13 → 25 |
| Standalone named poll | 9 → 21 | 5 → 5 | 9 → 21 |
| Standalone anonymous poll | 9 at 4; baseline stopped there | 5 → 5 | 5 → 5 |
| Generic/LinkedIn parent fetch, all five outcomes | Already implemented | 10 → 10 | 14 → 26 |
| Generic/LinkedIn actual child fetch, all five outcomes | No previous execution proof | 10 → 10 | 10 → 22 |

| Actual Calendar consumer | Rust 4 → 16 | Rails 4 → 16 |
| --- | --- | --- |
| `inbound_confirmed` | 6 → 6 | 7 → 7 |
| `inbound_cancelled` | 18 → 18 | 12 → 12 |
| `inbound_deleted` | 18 → 18 | 12 → 12 |
| `inbound_forbidden` | 6 → 6 | 7 → 7 |
| `inbound_unavailable` | 6 → 6 | 7 → 7 |
| `inbound_no_account` | 2 → 2 | 2 → 2 |
| `inbound_disconnected` | 2 → 2 | 2 → 2 |
| `inbound_departed` | 5 → 5 | 6 → 6 |
| `inbound_local_declined` | 6 → 6 | 7 → 7 |
| `sync_update` | 8 → 8 | 7 → 7 |
| `sync_conflict` | 9 → 9 | 8 → 8 |
| `sync_unavailable` | 9 → 9 | 7 → 7 |
| `sync_deleted` | 2 → 2 | 2 → 2 |


## Failing-first and discrimination evidence

The pin/poll baseline uses the production source from stacked E `16d00231e`; both new scaling tests fail. The initial exceptional parser baseline reports 84 Calendar and 39 slash differences. Its six earlier HTTP differences were section-extraction harness errors and are not app-defect claims. After shared parsing, an expanded real HTTP baseline had six actual differences: reminder/scheduling RangeError, three scheduling ArgumentError messages, and `/dnd` RangeError; all 64 final requests now match. The duration extension then failed first against `a4132b9c1`: New York `/dnd 7d` and `/ooo 7d` both stored 16:00Z instead of Rails' 15:00Z. The final eight-request duration corpus also covers Chicago, Lord Howe and UTC; the fix uses the existing shared local-day resolver rather than elapsed seconds. A subsequent full run exposed two genuine service-context regressions (`slash_dispatch_and_rows_match_rails` and `slash_preexisting_user_validations_and_calendar_match_rails`): direct Rails Dispatcher calls keep ambient UTC, whereas requests execute `SetTimeZone#apply_user_time_zone`. `handlers.rb` is unchanged between the older domain-vector pin and d7c7de92. The request adapter now selects a scoped dispatch entry point; direct service behavior and both existing vectors remain unchanged. Seven domain tests plus all 38 named/parser/HTTP tests pass after the correction.

The first named corpus had nine real missing attachment calls and an expired seed-cookie harness error. A corrected fresh-cookie run still failed `/ooo` waiting for the legacy callback; using the owner API produces both exact frames. The final baseline count guard recognizes bound prepared SQL rather than expanded parameters. All 35 names pass against the final production adapter.

Already implemented owner paths use served negative controls: mutate command metadata/recognition/result kind, omit the actual Drive payload field, change Calendar payload text, mutate persisted embed title, and stop discarding missing queued records. The same tests reject **40/40**, with no changed expectations or timing thresholds. All five source backups then compare byte-for-byte with restored production files. These are discrimination controls, not a claim that the unchanged owners had all those defects.

Recovery/reference-generator mistakes (wrong fake resolver API, missing fake HTTP method/body support, origin configuration, section extraction and stale cookie) were corrected before pinning final vectors. They are not counted as Rust bugs. The two direct-duration mismatches were deterministic context defects and were corrected; neither vector was re-pinned. No timeout/retry threshold was widened and no ignore was added. A verification mistake overlapped a baseline build with a workspace test run using the same target. The linker removed the active test executable, and three environment-isolation checks could not respawn `current_exe()` (OS error 2): `huddle::tests::cleanup_background_queue_and_http_enqueue_rollback`, `integrations::fizzy::cards::tests::ws15e_fizzy_web_urls_follow_configured_host`, and `integrations::web_push::tests::ws17_pinned_delivery_ignores_all_proxy_environment_keys`. The archived campfire summary is `2647 passed; 3 failed; 5 ignored`. The owned process readlink proved it was a deleted linker temporary. That run is preserved as invalid verification, not attributed to huddle behavior or timing. Final Rust builds/tests are serialized; none of those tests or their timing was changed.

Raw selected receipts (ANSI color escapes removed only):

```text
pin-poll-before.log
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 2577 filtered out; finished in 1.42s

pin-poll-final.log
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 2577 filtered out; finished in 1.50s

exceptional-baseline.log
WS8bm2 exceptional slash: 276 matched; 39 differed
WS8bm2 exceptional calendar: 231 matched; 84 differed
WS8bm2 exceptional structured HTTP: 32 matched; 6 differed
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 2611 filtered out; finished in 25.63s

exceptional-after.log
WS8bm2 exceptional slash: 315 matched; 0 differed
WS8bm2 exceptional calendar: 315 matched; 0 differed
WS8bm2 exceptional structured HTTP: 38 matched; 0 differed
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 2646 filtered out; finished in 19.25s

exceptional-http-before.log
WS8bm2 exceptional structured HTTP: 50 matched; 6 differed
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2650 filtered out; finished in 41.88s

slash-named-fixed.log
test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 2614 filtered out; finished in 4.10s

drive-after.log
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 2644 filtered out; finished in 1.22s

embed-failure-positive.log
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2651 filtered out; finished in 10.37s

duration-before.log
WS8bm2 exceptional structured HTTP: 56 matched; 2 differed
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2654 filtered out; finished in 44.99s

final-focused.log
WS8bm2 exceptional calendar: 315 matched; 0 differed
WS8bm2 exceptional slash: 315 matched; 0 differed
WS8bm2 exceptional structured HTTP: 64 matched; 0 differed
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 2609 filtered out; finished in 45.59s

pre-main210-duration-context-final-workspace.log
test result: FAILED. 1293 passed; 2 failed; 4 ignored; 0 measured; 0 filtered out; finished in 186.71s

duration-ambient-db-positive.log
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 1299 filtered out; finished in 39.67s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

duration-ambient-http-positive.log
WS8bm2 exceptional slash: 315 matched; 0 differed
WS8bm2 exceptional calendar: 315 matched; 0 differed
WS8bm2 exceptional structured HTTP: 64 matched; 0 differed
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 2628 filtered out; finished in 35.03s

served-negative.log
test result: FAILED. 0 passed; 40 failed; 0 ignored; 0 measured; 2612 filtered out; finished in 13.91s
WS8bm2 served negative controls exit: 101
```

## All 35 named built-in ports

Each row is a separate passed WS8 test in `controllers/message_features/slash_named_tests.rs`; all 35 also fail the served command mutations. Their original names are pinned from `test/services/slash_commands/dispatcher_test.rb`, independently of WS11's ledger.

| Rails name | Rust test name |
| --- | --- |
| registry holds every shipped command with metadata | `builtin_registry_holds_every_shipped_command_with_metadata` |
| command_text? matches slash commands but not escapes or play passthrough | `builtin_command_text_matches_slash_commands_but_not_escapes_or_play_passthrough` |
| huddle starts a call when configured | `builtin_huddle_starts_a_call_when_configured` |
| huddle errors when unconfigured | `builtin_huddle_errors_when_unconfigured` |
| event opens the prefilled form url | `builtin_event_opens_the_prefilled_form_url` |
| event without a time prefills the title only | `builtin_event_without_a_time_prefills_the_title_only` |
| event rejects past times | `builtin_event_rejects_past_times` |
| bare event opens the blank form | `builtin_bare_event_opens_the_blank_form` |
| poll opens the builder in channels but not threads | `builtin_poll_opens_the_builder_in_channels_but_not_threads` |
| remind posts and saves with a reminder | `builtin_remind_posts_and_saves_with_a_reminder` |
| remind rejects unusable input without posting | `builtin_remind_rejects_unusable_input_without_posting` |
| status sets emoji and text until end of day | `builtin_status_sets_emoji_and_text_until_end_of_day` |
| status rejects blank arguments | `builtin_status_rejects_blank_arguments` |
| dnd toggles, takes durations, and turns off | `builtin_dnd_toggles_takes_durations_and_turns_off` |
| dnd rejects garbage durations | `builtin_dnd_rejects_garbage_durations` |
| ooo sets an end with a note, and off clears it | `builtin_ooo_sets_an_end_with_a_note_and_off_clears_it` |
| ooo takes week durations, dates, and datetimes | `builtin_ooo_takes_week_durations_dates_and_datetimes` |
| ooo bare tomorrow and weekdays run to the end of the day | `builtin_ooo_bare_tomorrow_and_weekdays_run_to_the_end_of_the_day` |
| ooo bare dates run to the end of the day | `builtin_ooo_bare_dates_run_to_the_end_of_the_day` |
| ooo bare month dates roll to next year when this year's passed | `builtin_ooo_bare_month_dates_roll_to_next_year_when_this_year_s_passed` |
| ooo day durations stay exact | `builtin_ooo_day_durations_stay_exact` |
| ooo broadcasts the badge and the notice | `builtin_ooo_broadcasts_the_badge_and_the_notice` |
| ooo off while calendar OOO covers says the calendar still shows it | `builtin_ooo_off_while_calendar_ooo_covers_says_the_calendar_still_shows_it` |
| ooo rejects blank arguments, garbage, past times, and long notes | `builtin_ooo_rejects_blank_arguments_garbage_past_times_and_long_notes` |
| shrug posts with the shrug | `builtin_shrug_posts_with_the_shrug` |
| posting commands in a board answer an error without posting | `builtin_posting_commands_in_a_board_answer_an_error_without_posting` |
| posting commands in a board thread still post | `builtin_posting_commands_in_a_board_thread_still_post` |
| slash posts in threads skip the legacy webhook fanout | `builtin_slash_posts_in_threads_skip_the_legacy_webhook_fanout` |
| slash posts in channels fan out to legacy webhooks | `builtin_slash_posts_in_channels_fan_out_to_legacy_webhooks` |
| slash posts in threads process attachments once | `builtin_slash_posts_in_threads_process_attachments_once` |
| me posts an action line | `builtin_me_posts_an_action_line` |
| me requires an action | `builtin_me_requires_an_action` |
| play posts through the normal message path | `builtin_play_posts_through_the_normal_message_path` |
| slash posts never start a stream | `builtin_slash_posts_never_start_a_stream` |
| unknown commands error with the available list | `builtin_unknown_commands_error_with_the_available_list` |

## Remaining for cutover, with owner

1. **WS8b-m2, with WS15e's queue contract: additional enqueue-rejection permutations.** The new fatal-writer tests reject metadata UPDATEs; they do not inject failure when an otherwise successful callback tries to enqueue a stale sibling. Parent/claim/queue commit boundaries under actual enqueue-adapter rejection still need a Rails differential. This is unblocked proof work, not an absent API or a completed behavior.
2. **WS8b-m2, with WS14g: Calendar InboundSync/SyncEntry transport/429 retry and exhaustion permutations.** The new matrix proves permanent 503 outcomes and queued child execution. Existing E MeetLink pending retries are retained; they do not substitute for these two consumers' transient retry/exhaustion proof.
3. **WS8b-m2: remaining exceptional slash input evidence.** The shared absolute-date grammar and the recorded coercion/DST corpus are covered. Wide relative-duration overflow, exceptional leading/trailing time-split inputs and unsampled top-level structured parameter containers outside that corpus still need Rails differentials; the legacy split helpers retain their Option interfaces. Do not infer exact exceptional parity for those unsampled paths from the shared fallback parser tests. This is unblocked WS8 evidence/implementation work, not a second-parser dependency.
4. **WS11-API: typed AgentBudgetNotice facts reader.** Per the explicit brief, keep the existing named, flagged read-only seam in `presenters/activity.rs`; no typed adapter replacement was attempted. WS17 physical sending, WS11 invocation/auth and WS13 huddles are already real integrations and are not flagged stand-ins.
5. **Cutover system-test phase (lead/system-test owner): final both-app browser run.** The prior accepted 45/45 behavior results on each app are retained below; no pixel checks or new browser harness were added or counted in F. These behaviors are ported, not newly unported.
6. **WS12: broader board/work parity.** The prohibited owner files were not changed by F. The inherited board/work N+1/bind-limit fixes are already on main; no new WS8 blocker is asserted.

The requested exceptional grammar/coercion and 35-name matrices are complete for their recorded corpus. No equivalence claim covers unsampled arbitrary Ruby date strings or failure interleavings. The only currently named owner-API seam is AgentBudgetNotice; because items 1–3 remain, this checkpoint is **not owner-blocked-only**.

## Retained Rails file coverage inventory

These are named Rails behaviors covered by grouped Rust tests/vectors, not a claim that the Rails controller suites were executed as suites in F. The full workspace suite reruns the Rust ports. Ordinary named controller coverage is **140/140**; the former single agent case is now integrated.

| File under `test/controllers/` | Named behaviors covered / total |
| --- | --- |
| `rooms/polls_controller_test.rb` | 16/16 |
| `messages/pins_controller_test.rb` | 6/6 |
| `rooms/pins_controller_test.rb` | 3/3 |
| `saved_items_controller_test.rb` | 12/12 |
| `scheduled_messages_controller_test.rb` | 19/19 |
| `searches_controller_test.rb` | 36/36 |
| `rooms/slash_commands_controller_test.rb` | 10/10 |
| `autocompletable/icons_controller_test.rb` | 6/6 |
| `autocompletable/slash_commands_controller_test.rb` | 7/7 |
| `autocompletable/users_controller_test.rb` | 5/5 |
| `rooms/message_links_controller_test.rb` | 12/12 |
| `rooms/files_controller_test.rb` | 8/8 |

| Browser file under `test/system/` | Prior accepted Rails/Rust behavior results |
| --- | --- |
| `polls_test.rb` | 4/4 each |
| `pins_saved_test.rb` | 7/7 each |
| `slash_commands_test.rb` | 26/26 each |
| `search_files_test.rb` | 4/4 each |
| `scheduled_messages_test.rb` | 4/4 each |
| Total | 45/45 each; not rerun in F |



## Fresh-clone gates, commands and raw summaries

A new `git clone --no-hardlinks --branch rust/ws8bm2-message-features-f . .scratch/ws8bm2-f/fresh-final` started at `a1549ea32`. Before tests it fast-forwarded to the test-only clippy cleanup `3ebce2f00`, then to tree-identical checkpoint merge `a4132b9c1`, and finally to `437f53c4e` with the DST duration fix, then to the #210 merge and context correction `07fec984c1353e4692c7ea106263410c4af252fe` before the final gates. Default, first-run and agent-UI seeds were rebuilt there. No untracked source, seed, vector or scratch fixture was copied. The earlier generated HTTP body/token outputs and fresh target output were removed before the final workspace run; their producers write outputs and do not read those files. Only registry/build cache and execution/resource configuration were reused. The earlier overlapping verification logs are archived separately and are not the final receipts. All 13 workspace packages are included, without html5ever exclusions. Four test threads, two Cargo workers and the existing machine rustc slots are retained. No new target directory was created for compilation; any scratch target output is removed after verification.

The execution bridge is `.scratch/ws8bm2-f/ci-env.sh`: CI=1, CARGO_BUILD_JOBS=2, the existing toolchain image, docker --cpus 4, the unchanged rustc-throttle/slot mounts, the owned native target, registry-cache volume, and allocated port ranges 52500–52549/52550–52599. Fresh-clone gates set CARGO_HOME=/cargo-cache and CARGO_TARGET_DIR=/native-target. These are cache/resource settings, not test inputs.

From the fresh clone, every command below was rerun and its output retained in the parent `.scratch/ws8bm2-f`:

```sh
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_RUNTIME=docker PARITY_IMAGE=ws8bm2-reference:d7c7de92 rust/parity/bin/seed build default first_run agents_ui > ../final-seeds.log 2>&1
bash rust/ci/cargo.sh clippy --locked --workspace --all-targets -j2 -- -D warnings > ../final-clippy.log 2>&1
WS8BM2_ORACLE_SCRATCH="$PWD/../fresh-oracles-post210" python3 rust/reference-tools/messaging/verify_oracles.py > ../final-fresh-oracles.log 2>&1
python3 rust/reference-tools/messaging/features-reference-check.py --self-test > ../final-reference-source.log 2>&1
bash rust/ci/cargo.sh test --locked --workspace --no-fail-fast -j2 -- --test-threads=4 --nocapture > ../final-workspace.log 2>&1
bash rust/ci/cargo.sh metadata --locked --format-version 1 > ../final-fresh-metadata.json 2> ../final-fresh-metadata.stderr
bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins > ../final-release.log 2>&1
```

Derived aggregate across 61 Cargo summaries: 4701 passed; 0 failed; 14 ignored. The following lines are copied verbatim from Cargo, with ANSI escapes removed only.

```text
test result: ok. 2661 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 1153.80s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.65s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.01s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1302 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 168.26s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.96s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.97s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.58s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.20s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 19.08s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.62s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.40s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.81s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.01s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.65s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
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
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.55s
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
WS8bm2 full fresh workspace exit: 0
```

Strict clippy

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 48s
WS8bm2 final fresh clippy exit: 0
```

Release-input build

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 07s
WS8bm2 release input build exit: 0
```

Locked metadata completed with exit 0; parsed JSON contains all 13 workspace members.

```text
WS8bm2 oracle replay: 45/45 independently replayed fixtures byte-identical
WS8bm2 reference source check: 118 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
```

All inherited ignored tests (F removes the two Drive ignores and adds none):

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test controllers::internal_huddle_tests::huddle_gateway_own_node_suite_against_rust_endpoints ... ignored, requires Node and the gateway pinned ws package; run explicitly with --ignored
test controllers::rooms::system_browser_tests::huddle_system_cases_in_real_browser ... ignored, requires Docker and the pinned Playwright image; run parity/system/ws13
test controllers::rooms::system_browser_tests::livekit_stage_system_cases_in_real_browser ... ignored, requires the project-local LiveKit server; run parity/system/ws13-livekit
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

The final report copies in the worktree and delegation directory are byte-identical. The 40K fresh-clone `rust/target` generated test-output directory was deleted after verification. Dependency source directories named `target` under registry caches were identified and retained; they are source inputs, not build targets. Final git diff/ownership/process checks are recorded alongside the raw receipts. Full logs remain as evidence; this report does not repeat earlier rounds' commands as newly executed gates.
