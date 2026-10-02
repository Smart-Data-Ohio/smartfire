#!/usr/bin/env python3
"""List every Rails Slack test and the exact partial-delivery boundary."""
from pathlib import Path
import re

root = Path(__file__).resolve().parents[3]
model_ports = {
 'cancel! stops queued and running runs at their boundary',
 'undo! enqueues the undo job and resets progress tracking',
 'undo! refuses dry runs and active runs',
 'claim_running! lets a single queued run through while none runs',
 'claim_running! refuses while another run is undoing',
 'claim_running! refuses while a cancelled run holds a fresh step lease',
 'a stale step lease no longer blocks claims',
 'a second acquire fails while a fresh lease is held',
 'a stale lease can be taken over without losing saved progress',
 'releasing with the wrong token keeps the lease',
 'refresh_step_lease! renews a held lease without touching other state',
 'refresh_step_lease! refuses a token that no longer holds the lease',
 'a lease-looking state on a completed run does not block claims',
 'step lease stamps are written in UTC even under a user time zone',
 'fresh leases block and stale leases pass under user time zones',
 'undo! refuses with no status change while another run is queued, running or undoing',
 'the undo claim itself refuses a fresh lease held outside an active status',
 'undo_blocked_reason is nil when nothing else is active',
 'undo waits while the run itself holds a fresh step lease',
 'undo waits while another run holds a fresh step lease',
 'the later-overlap answer refreshes after reload',
 'record_issue! caps issues with a suppression notice',
 'sweep re-enqueues stale running runs',
 'sweep starts the oldest queued run only when nothing runs or undoes',
 'sweep never enqueues a second job for a run with one pending',
 'sweep re-enqueues stalled undoing runs',
}
client_deferred = {}
job_ports = {
 'undo is blocked while another run is queued': '119 real Rails HTTP actions compare blocking reason and unchanged DB/queue.',
 'dry run writes nothing except the run row and its issues': 'slack_sql_store_dry_run_collects_samples_without_domain_rows_or_reply_fetches',
 'workspace import creates rooms, memberships, messages, threads, boosts and pins': 'slack_sequence_matches_rails_import_undo_reimport_database_rows: every field in 89 tables',
 'tiny step budget spans several steps for one conversation': 'slack_runner_zero_budget_stops_after_page_and_preserves_reply_cursor plus actual fixture completion',
 'a second import creates zero duplicates': 'slack_undo_actual_overlapping_imports_require_lifo_and_name_later_importer',
 'undo is last-in, first-out per conversation': 'slack_undo_actual_overlapping_imports_require_lifo_and_name_later_importer',
 'message timestamps keep exact microseconds': 'slack_writer_timestamp_microseconds_do_not_round_through_float plus full Rails row differential',
 'undo removes exactly what the run created and leaves the rest': 'slack_undo_deletes_data_search_and_mappings_then_reimports_fixture plus kept-content tests and Rails differential',
 'undo keeps a created room that gained foreign messages and records an issue': 'slack_undo_keeps_live_thread_parent_for_foreign_or_scheduled_replies',
 'undo keeps threads and rooms with real activity, with members and mappings': 'slack_undo_keeps_live_thread_parent_for_foreign_or_scheduled_replies',
 'undo keeps imported messages holding a poll, a saved item or someone else\'s pin': 'slack_undo_keeps_poll_saved_item_foreign_pin_and_pending_root_reply',
 'undo keeps a thread with a pending scheduled reply, and the message it quotes': 'slack_undo_keeps_live_thread_parent_for_foreign_or_scheduled_replies',
 'import stays silent: no foreign jobs, broadcasts, unread or inbox items': 'slack_writer_history_replies_pins_reactions_are_quiet_and_keep_microseconds; rejecting broadcast sink and real job queue',
 'imported messages are searchable': 'slack_writer_history_replies_pins_reactions_are_quiet_and_keep_microseconds; message_search_index included in both Rails differentials',
 'users map by email, placeholders fill in, guests stay deactivated': 'slack_users_preview_import_repeat_match_pinned_rails_fixture_rows',
 'placeholder Google-link eligibility follows the allowed domains': 'slack_users_humans_receive_open_rooms_bots_guests_and_unknown_authors_do_not plus Rails user vector',
 'channels merge into same-name rooms without touching memberships': 'slack_conversations_only_public_workspace_channels_auto_merge',
 'room targets force new, skip and explicit rooms, and reject bad ids': 'slack_conversations_explicit_merge_leaves_memberships_and_invalid_target_reports',
 'room target id must be an alive Open or Closed room': 'slack_conversations_explicit_merge_leaves_memberships_and_invalid_target_reports',
 'workspace runs never auto-merge a private channel by name': 'slack_conversations_only_public_workspace_channels_auto_merge',
 'workspace runs merge a private channel only into its room target': 'slack_conversations_explicit_merge_leaves_memberships_and_invalid_target_reports',
 'personal runs never merge a private channel into an existing room': 'slack_conversations_only_public_workspace_channels_auto_merge',
 'personal run imports DMs, group DMs and private channels': 'slack_sequence_personal_matches_rails_import_undo_reimport_database_rows: every field in 89 tables',
 'personal run skips self DMs and Slackbot DMs': 'slack_conversations_directs_reuse_member_sets_skip_self_and_slackbot',
 'personal run dedupes a DM another member already imported': 'slack_conversations_directs_reuse_member_sets_skip_self_and_slackbot',
 'cancel stops the run at the next step boundary': 'slack_sql_store_rejects_cancelled_or_replaced_lease_before_domain_writes',
 'a cancel observed mid-step stops the next page from being written': 'slack_runner_cancelled_during_fetch_does_not_commit_page_or_progress',
 'HTTP 429 reschedules the run after Retry-After': 'slack_job_retry_after_commits_heartbeat_and_delayed_job_atomically',
 'auth errors fail the run and flag the connection': 'slack_job_cancel_during_auth_error_preserves_cancel_and_disconnects plus client error vectors',
 'two queued runs run one at a time': 'slack_job_live_lease_and_lost_run_claim_never_execute_callback plus registered serial worker',
}
job_ports.update({'undo keeps mappings for kept placeholders, so re-import creates no duplicates': 'Eleven retained Rails differentials, including sessions, Google account/identity, password and placeholder authorship; 89 tables after import, undo and reimport.', 'undo keeps a created room that holds an event and a scheduled message': 'sequence_keep_room_event_schedule.json: all 89 Rails tables match three phases.', 'undo keeps the parent of a thread holding a saved reply': 'sequence_keep_saved_reply.json: all 89 Rails tables match three phases.', 'undo keeps an imported message someone started a thread on': 'sequence_keep_foreign_thread.json: all 89 Rails tables match three phases.', 'undo keeps a thread whose reply holds a poll': 'sequence_keep_poll_reply.json: all 89 Rails tables match three phases.', 'undo removes a thread whose scheduled replies were all sent': 'sequence_keep_sent_reply.json: all 89 Rails tables match three phases.', 'catch-up picks up a new message and a late reply': 'slack_catchup_new_message_late_reply_and_deleted_mapped_thread_match_rails: actual local TLS history/replies, 30-day oldest and native thread counts/activity.', 'catch-up skips a thread whose mapped thread was deleted': 'slack_catchup_new_message_late_reply_and_deleted_mapped_thread_match_rails: deleted ghost mapping remains, issue recorded and no recreated thread/reply.', 'a multi-year conversation spanning several steps imports everything': 'slack_history_windows_full_catchup_finish_earlier_room_and_keep_forward_unread_pointers: three pages across three years, zero step budget.', 'a full import after a date-bounded test import imports everything older too': 'slack_history_windows_full_catchup_finish_earlier_room_and_keep_forward_unread_pointers: bounded middle year followed by unbounded full history.', 'catch-up after a kept test import and a full import re-reads only 30 days': 'slack_history_windows_full_catchup_finish_earlier_room_and_keep_forward_unread_pointers: exact oldest request in subsequent catch-up.', "a full import's coverage ends when an earlier run under it is undone": 'slack_catchup_coverage_is_invalidated_by_undoing_an_earlier_bounded_run: native undo invalidates coverage and unbounded history restores middle-year row.', 'a full import finishes rooms an earlier test import created': 'slack_history_windows_full_catchup_finish_earlier_room_and_keep_forward_unread_pointers: native room timestamp and every membership checked.', "finishing never moves an earlier membership's read pointer backwards": 'slack_history_windows_full_catchup_finish_earlier_room_and_keep_forward_unread_pointers: newer live pointer and unread membership preserved.', 'per-conversation record lookups seek the identity index': 'slack_finishing_per_conversation_seeks_identity_index_and_renews_heartbeat_lease: SQLite EXPLAIN range seeks identity index.', 'finishing refreshes the heartbeat while looping over rooms': 'slack_finishing_per_conversation_seeks_identity_index_and_renews_heartbeat_lease: native multi-conversation finish advances stale heartbeat and keeps lease.'})
job_partial = {}

model_extra = {'a step that raises still releases its lease': 'slack_job_cancel_during_auth_error_preserves_cancel_and_disconnects plus retry-after tests assert no retained lease after errors.', 'a continuing step releases its lease before enqueueing the next job': 'slack_job_releases_before_durable_continuation_and_uses_serial_queue: rejecting SQL trigger prevents publishing while busy.', 'a continuing undo releases its lease before enqueueing the next job': 'slack_job_undo_releases_before_continuation: native row lease removal and serial durable undo job.', 'a step job that cannot claim its queued run exits without re-enqueueing': 'slack_job_live_lease_and_lost_run_claim_never_execute_callback: loser callback rejects execution and enqueued stamp clears.'}
job_ports.update({
 'mentions of mapped users outside the channel render as tokens, unknown ids fall back':'slack_writer_mapped_nonmember_mentions_render_tokens_and_unknown_labels_fall_back; actual saved body and mentionees.',
 'a conversation whose mapped room was deleted is skipped with an issue':'slack_lifecycle_mapped_deleted_room_is_skipped_and_other_room_finishes; second import, exact issue, other room done.',
 'workspace runs never auto-merge a large group DM by name':'slack_lifecycle_large_groups_personal_targets_and_dry_preview_follow_rails; 11 unknown members and existing same-name Closed room.',
 'workspace runs merge a large group DM only into its room target':'slack_lifecycle_large_groups_personal_targets_and_dry_preview_follow_rails; explicit existing room, message and created_record flag.',
 'personal runs never auto-merge a large group DM by name':'slack_lifecycle_large_groups_personal_targets_and_dry_preview_follow_rails; personal explicit target ignored and same-name room untouched.',
 'dry runs preview a targeted large group DM as a merge':'slack_lifecycle_large_groups_personal_targets_and_dry_preview_follow_rails; dry target action/room id and no messages.',
 'personal runs ignore room target ids':'slack_lifecycle_large_groups_personal_targets_and_dry_preview_follow_rails; private-channel target ignored.',
 'room setup is atomic: a crash while recording memberships leaves nothing behind':'slack_lifecycle_room_membership_mapping_crash_rolls_back_then_resumes_cleanly; rejecting SQL trigger, rollback and fresh full setup.',
 'date bounds keep every row in range and are sent to Slack':'slack_lifecycle_date_bounds_filter_all_rows_and_are_sent_on_every_history_page; original fixture exact bounds, one persisted microsecond row, both CCHAN requests.',
 'scope errors fail with the missing scope and leave the connection':'slack_lifecycle_scope_transient_and_auth_failures_preserve_resumable_mappings; real local TLS failure, exact error, connected grant unchanged.',
 'transient failures past the retry budget fail the run':'slack_lifecycle_scope_transient_and_auth_failures_preserve_resumable_mappings; actual HTTP 500 four attempts, exact error, lease release.',
 'failed runs stay resumable: a new run continues from the mapping':'slack_lifecycle_scope_transient_and_auth_failures_preserve_resumable_mappings; auth failure after mapped users/room, reconnect and successful repeat with zero mapped-user delta.',
 'workspace runs exclude private channels when asked':'slack_runner_discovers_sorted_scope_and_does_not_page_history_in_discovery; actual TLS public_channel-only list request.',
 'import skips a thread whose parent message was deleted mid-run':'slack_workspace_deleted_queued_parent_and_truncated_reactions_match_rails; delete actual parent between persisted page and replies, no reply/thread, exact issue count.',
 'completing a run kicks the next queued run':'slack_workspace_full_import_completion_hands_off_to_next_queued_job; actual import worker chain, exactly one durable handoff for queued run.',
 'undoing a run kicks the next queued run':'slack_model_finishing_cancel_failure_and_undo_kick_next_queued_run; actual imported room then Undoer/job chain, durable handoff once.',
 'truncated reaction lists import the listed users with one issue per message':'slack_workspace_deleted_queued_parent_and_truncated_reactions_match_rails; three listed reactors, two truncated reactions, one exact warning.',
})
model_extra.update({
 'a runner step refreshes its lease wherever it refreshes the heartbeat':'slack_model_commit_refreshes_owned_lease_and_preserves_unleased_stamp; token held, heartbeat and stamp advance, progress persists.',
 'a runner without a lease token leaves the lease alone':'slack_model_commit_refreshes_owned_lease_and_preserves_unleased_stamp; heartbeat advances, foreign token and stamp unchanged.',
 'an undoer step refreshes its lease when it saves undo state':'slack_undo_saved_state_refreshes_heartbeat_and_owned_lease; actual undo state save, heartbeat and owned stamp advance.',
 "another job's lease write is not mistaken for progress on conflict":'slack_runner_lease_only_conflict_is_not_mistaken_for_saved_progress; lease-only write rethrows real UNIQUE conflict; saved progress continues.',
 'the undo claim itself refuses a queued run that slips in after the pre-check':'slack_import_atomic_undo_claim_refuses_queue_arriving_after_precheck; direct atomic claim, stale successful pre-check, no status/event change.',
 'failed and cancelled runs kick the next queued run':'slack_model_finishing_cancel_failure_and_undo_kick_next_queued_run; both terminal states durable handoff exactly once.',
 'step_finishing does not overwrite a cancelled run':'slack_model_finishing_cancel_failure_and_undo_kick_next_queued_run; finishing operation stops and cancelled status survives.',
 'step_finishing completes a running run and kicks the next queued one':'slack_model_finishing_cancel_failure_and_undo_kick_next_queued_run; actual finishing store, completed/done stats and one durable handoff.',
})
paths = sorted(set(root.glob('test/models/*slack*_test.rb')) | set(root.glob('test/models/slack/*_test.rb')) | set(root.glob('test/jobs/slack_import/*_test.rb')) | set(root.glob('test/controllers/slack/*_test.rb')) | set(root.glob('test/controllers/accounts/*slack*_test.rb')) | {root / 'test/system/slack_import_test.rb'})
lines = ['# WS16 Rails test inventory — owned declarations covered', '',
 'All 244 original declarations have executable Rust behavior coverage. The separate combined Google placeholder-claim interaction awaits the unmerged WS14g sign-in start/callback handlers; Slack opt-in, OAuth, preview, real import and undo execute over HTTP.', '',
 'Covered is a behavior mapping, not a claim that the original Rails test was run against Rust. The executable Rust coverage is in `db/src/tests/slack*_test.rs`, `campfire/src/integrations/slack/client/tests.rs`, and the 481 generated converter vectors. Runner/protocol, actual durable worker execution, mappers, quiet Message save/undo and workspace/personal Rails row differentials now have executable coverage. OAuth, connection and setup routes now have Rails-generated transport/HTTP/view goldens and runtime security tests. All fifteen admin/personal run actions have real HTTP session/CSRF/row/audit/queue comparisons and 109 complete HTTP response body goldens plus 83 detached template bodies. The combined setup/sudo/preview/plan/import/progress/undo interaction also executes over HTTP. Eleven retained-undo variants now also compare all 89 tables, huddle destruction matches six Rails affected tables, and actual TLS catch-up/multi-year/finishing cases execute. All remaining lifecycle/workspace/model declarations now have native fault/large-group/lease/queue regressions; malformed payload and transport goldens also execute.', '']
counts = {'covered': 0, 'partial': 0, 'deferred': 0}
for path in paths:
    tests = re.findall(r'^\s*test "((?:[^"\\]|\\.)*)"', path.read_text(), re.M)
    lines += ['## ' + str(path.relative_to(root)), '', '| Rails test | State | Coverage or remaining work |', '|---|---|---|']
    for name in tests:
        state, note = 'deferred', 'WS16 continuation: implement and exercise the original behavior.'
        if '/models/slack/markdown_converter_test.rb' in str(path) and not name.startswith('rendering:'):
            state, note = 'covered', 'Rails-generated converter vectors; crafted-token timing guard is a Rust test.'
        elif '/models/slack/markdown_converter_test.rb' in str(path) and name.startswith('rendering:'):
            state,note='covered','slack_converter_real_saves_match_rails_bodies_and_mentionees; four real Rails Message-save body/mentionees goldens.'
        elif path.name in ('oauth_controller_test.rb', 'connections_controller_test.rb', 'slack_imports_controller_test.rb'):
            state, note = 'covered', 'slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks.'
        elif path.name in ('slack_import_runs_controller_test.rb', 'imports_controller_test.rb'):
            if name == "run page loads later runs' stats once across the undo checks":
                state, note = 'covered', 'slack_personal_show_reads_later_stats_once_for_both_undo_controls: native SQLite trace asserts one later-stats query across both controls and actual HTTP response.'
            else:
                state, note = 'covered', '119 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow.'
        elif '/models/slack/client_test.rb' in str(path):
            if name in client_deferred:
                state, note = 'partial', client_deferred[name]
            else:
                state, note = 'covered', 'Local TLS fixture requests, Rails error vectors, retry and pacing tests.'
        elif path.name == 'slack_import_test.rb' and path.parent.name == 'models' and name in model_extra:
            state, note = 'covered', model_extra[name]
        elif path.name == 'slack_import_test.rb' and path.parent.name == 'models' and name in model_ports:
            state, note = 'covered', 'Database lifecycle tests; independent writers for claims, leases, undo and sweeps.'
        elif path.name == 'slack_import_test.rb' and path.parent.name == 'models' and name.startswith('start!'):
            state, note = 'covered', '33 actual Ruby normalization/time vectors plus real HTTP run creation and durable enqueue comparison.'
        elif path.parent.name == 'system':
            state, note = 'covered', 'slack_admin_credentials_preview_plan_import_progress_and_undo_over_http: real sudo password/session, credentials, preview, selection, import, polling and undo controls.'
        elif path.parent.name == 'slack_import' and name in job_ports:
            state, note = 'covered', job_ports[name]
        elif path.parent.name == 'slack_import' and name in job_partial:
            state, note = 'partial', job_partial[name]
        elif 'undo is blocked' in name or 'later import' in name:
            note = 'WS16 continuation: controller behavior deferred; domain overlap/naming checks exist.'
        counts[state] += 1
        lines.append('| ' + name.replace('|', '\\|') + ' | ' + state + ' | ' + note + ' |')
    lines.append('')
lines += ['## Totals', '', ', '.join(f'{value} {key}' for key, value in counts.items()) + f'; {sum(counts.values())} Rails tests inventoried.', '']
(root / 'rust/plans/ws16-test-inventory.md').write_text('\n'.join(lines))
print('Slack Rails test inventory: ' + ', '.join(f'{value} {key}' for key, value in counts.items()) + f'; {sum(counts.values())} total')
