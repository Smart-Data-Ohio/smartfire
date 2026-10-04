# PR 243 mutation sample

Nineteen distinct new native tests: 19 behavior mutations killed at mapped assertion lines. No compile or setup failure counts. Production bytes restored and hashed. The temporary guards are reproducible with `reference-tools/cutover/mutation-check.py` and `b-mutations.json`; the command prefix must pass `CAMPFIRE_LEDGER_MUTATION` to the canonical container.

| Mutation | Native test | Actual discriminating panic | Raw summary |
|---|---|---|---|
| organizer_bot | `cutover_event_rejects_bot_and_nonmember_organizers` | `crates/db/src/tests/calendar_event_test/cutover_event_test.rs:87:5:` | `Summary [   0.171s] 1 test run: 0 passed, 1 failed, 5053 skipped` |
| deleted_venue | `cutover_event_rejects_soft_deleted_venue` | `crates/db/src/tests/calendar_event_test/cutover_event_test.rs:114:5:` | `Summary [   0.142s] 1 test run: 0 passed, 1 failed, 5053 skipped` |
| invisible_invites | `cutover_event_off_and_invisible_members_get_no_invitation` | `crates/db/src/tests/calendar_event_test/cutover_event_test.rs:130:5:` | `Summary [   0.173s] 1 test run: 0 passed, 1 failed, 5053 skipped` |
| bot_invites | `cutover_event_invitations_exclude_bots` | `crates/db/src/tests/calendar_event_test/cutover_event_test.rs:180:5:` | `Summary [   0.126s] 1 test run: 0 passed, 1 failed, 5053 skipped` |
| mentions_notify | `cutover_event_mentions_member_receives_update_and_cancel` | `crates/db/src/tests/calendar_event_test/cutover_event_test.rs:162:5:` | `Summary [   0.216s] 1 test run: 0 passed, 1 failed, 5053 skipped` |
| update_kind | `cutover_event_time_change_notifies_going_and_maybe_once` | `crates/db/src/tests/calendar_event_test/cutover_event_test.rs:193:9:` | `Summary [   0.172s] 1 test run: 0 passed, 1 failed, 5053 skipped` |
| cancel_kind | `cutover_event_cancel_notifies_going_maybe_and_handles_declined` | `crates/db/src/tests/calendar_event_test/cutover_event_test.rs:244:9:` | `Summary [   0.190s] 1 test run: 0 passed, 1 failed, 5053 skipped` |
| year_cap | `cutover_recurrence_head_plain_update_checks_one_year_cap` | `crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:52:5:` | `Summary [   0.187s] 1 test run: 0 passed, 1 failed, 5053 skipped` |
| occurrence_cap | `cutover_recurrence_rule_over_cap_rejects_and_preserves_original_series` | `crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:160:5:` | `Summary [   0.183s] 1 test run: 0 passed, 1 failed, 5053 skipped` |
| series_order | `cutover_recurrence_equal_time_orders_active_before_cancelled` | `crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:135:5:` | `Summary [   0.117s] 1 test run: 0 passed, 1 failed, 5053 skipped` |
| guard_reset | `cutover_recurrence_head_only_series_retimes_through_following` | `crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:220:5:` | `Summary [   0.214s] 1 test run: 0 passed, 1 failed, 5053 skipped` |
| description | `cutover_recurrence_head_description_edit_stays_local` | `crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:242:5:` | `Summary [   0.151s] 1 test run: 0 passed, 1 failed, 5053 skipped` |
| venue_change | `cutover_venue_following_propagates_change` | `crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:177:5:` | `Summary [   0.115s] 1 test run: 0 passed, 1 failed, 5053 skipped` |
| venue_clear | `cutover_venue_following_propagates_clear` | `crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:185:5:` | `Summary [   0.151s] 1 test run: 0 passed, 1 failed, 5053 skipped` |
| association_reader | `cutover_reference_edit_adds_the_exact_event` | `crates/db/src/tests/calendar_event_test/cutover_reference_test.rs:79:5:` | `Summary [   0.348s] 1 test run: 0 passed, 1 failed, 5053 skipped` |
| entry_destroy | `cutover_entry_destroy_enqueues_captured_remote_identity_after_commit` | `crates/db/src/tests/calendar_event_test/cutover_entry_test.rs:40:5:` | `Summary [   0.182s] 1 test run: 0 passed, 1 failed, 5053 skipped` |
| delete_callback | `cutover_entry_delete_skips_remote_delete` | `crates/db/src/tests/calendar_event_test/cutover_entry_test.rs:51:5:` | `Summary [   0.102s] 1 test run: 0 passed, 1 failed, 5053 skipped` |
| existing_zone | `cutover_events_update_uses_existing_zone_and_notifies_attendees` | `crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:348:5:` | `Summary [   1.127s] 1 test run: 0 passed, 1 failed, 5053 skipped` |
| preload_n1 | `cutover_events_index_query_count_is_independent_of_occurrence_count` | `crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:329:5:` | `Summary [   1.374s] 1 test run: 0 passed, 1 failed, 5053 skipped` |

Raw summaries are copied from the selected test runs. The sample receipt is `ledger-ws14-ws15-b-mutations.json`. The equality-end validation mutation was also observed to fail; its exploratory run is not included in this retained 19-test sample.
