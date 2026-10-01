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
client_deferred = {
 'network errors retry then raise': 'Exact Ruby transport exception class/message remains unported; retry count and final RequestError are covered.',
 'a 5xx that recovers returns the payload': 'Fixture server sequence recovery still needed.',
 'history fixtures arrive newest-first like conversations.history': 'Fixture byte identity is covered; explicit ordering assertion still needed.',
 'on_request fires once per attempt for api call counts': 'Callback counts tested for failed retries; recovery sequence still needed.',
}
job_ports = {
 'undo is blocked while another run is queued': '109 real Rails HTTP actions compare blocking reason and unchanged DB/queue.',
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
job_partial = {
 'date bounds keep every row in range and are sent to Slack': 'Client query tests and 18 Rails boundary vectors cover this seam; exact original full bounded fixture scenario remains.',
 'scope errors fail with the missing scope and leave the connection': 'Client vector plus shared job failure branch; direct job scope-failure scenario remains.',
 'transient failures past the retry budget fail the run': 'Client exhaustion/protocol guards implemented; exact non-Slack Ruby exception class text remains.',
 'undo keeps mappings for kept placeholders, so re-import creates no duplicates': 'Claimed-user mapping and kept-root reimport covered; all original session/Google/authorship variants remain.',

}
paths = sorted(set(root.glob('test/models/*slack*_test.rb')) | set(root.glob('test/models/slack/*_test.rb')) | set(root.glob('test/jobs/slack_import/*_test.rb')) | set(root.glob('test/controllers/slack/*_test.rb')) | set(root.glob('test/controllers/accounts/*slack*_test.rb')) | {root / 'test/system/slack_import_test.rb'})
lines = ['# WS16 Rails test inventory — partial', '',
 'Owner for every deferred or partial row: **WS16 continuation**. No tests are reassigned to other workstreams.', '',
 'Covered is a behavior mapping, not a claim that the original Rails test was run against Rust. The executable Rust coverage is in `db/src/tests/slack*_test.rs`, `campfire/src/integrations/slack/client/tests.rs`, and the 481 generated converter vectors. Runner/protocol, actual durable worker execution, mappers, quiet Message save/undo and workspace/personal Rails row differentials now have executable coverage. OAuth, connection and setup routes now have Rails-generated transport/HTTP/view goldens and runtime security tests. All fifteen admin/personal run actions have real HTTP session/CSRF/row/audit/queue comparisons and 83 complete template body goldens. The combined setup/sudo/preview/plan/import/progress/undo interaction also executes over HTTP. Remaining fault/large-history cases are listed below.', '']
counts = {'covered': 0, 'partial': 0, 'deferred': 0}
for path in paths:
    tests = re.findall(r'^\s*test "((?:[^"\\]|\\.)*)"', path.read_text(), re.M)
    lines += ['## ' + str(path.relative_to(root)), '', '| Rails test | State | Coverage or remaining work |', '|---|---|---|']
    for name in tests:
        state, note = 'deferred', 'WS16 continuation: implement and exercise the original behavior.'
        if '/models/slack/markdown_converter_test.rb' in str(path) and not name.startswith('rendering:'):
            state, note = 'covered', 'Rails-generated converter vectors; crafted-token timing guard is a Rust test.'
        elif path.name in ('oauth_controller_test.rb', 'connections_controller_test.rb', 'slack_imports_controller_test.rb'):
            state, note = 'covered', 'slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks.'
        elif path.name in ('slack_import_runs_controller_test.rb', 'imports_controller_test.rb'):
            if name == "run page loads later runs' stats once across the undo checks":
                state, note = 'partial', 'One blocked-reason scan feeds the view; the original SQL-count regression still needs instrumentation.'
            else:
                state, note = 'covered', '109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow.'
        elif '/models/slack/client_test.rb' in str(path):
            if name in client_deferred:
                state, note = 'partial', client_deferred[name]
            else:
                state, note = 'covered', 'Local TLS fixture requests, Rails error vectors, retry and pacing tests.'
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
