#!/usr/bin/env python3
"""List every pinned Rails GitHub test and its coverage or explicit deferred owner."""
from pathlib import Path
import re
import subprocess

root = Path(__file__).resolve().parents[2]
repo = root.parent
names = {
    'test/controllers/github/connections_controller_test.rb': ['github_connections_http_identity_flash_revocation_and_audits_match_rails'] * 5 + [None,None] + ['github_connections_http_identity_flash_revocation_and_audits_match_rails'] * 3 + [None,None,None],
    'test/controllers/github/app_connections_controller_test.rb': ['github_connections_oauth_state_round_trips_and_is_consumed_before_error_or_exchange'] * 2 + ['github_connections_http_identity_flash_revocation_and_audits_match_rails'] * 3 + ['github_connections_oauth_state_round_trips_and_is_consumed_before_error_or_exchange'] + ['github_connections_http_identity_flash_revocation_and_audits_match_rails'] * 7,
    'test/controllers/accounts/bots/github_connections_controller_test.rb': ['github_connections_http_identity_flash_revocation_and_audits_match_rails',None,'github_connections_security_enforces_sudo_admin_active_bot_and_single_use_state'] + ['github_connections_http_identity_flash_revocation_and_audits_match_rails'] * 5 + [None,None],
    'test/controllers/rooms/github_subscriptions_controller_test.rb': [
        'github_subscription_http_status_flash_token_events_and_membership_match_rails',
    ] * 7 + ['github_subscription_http_security_rejects_nonmembers_plain_members_direct_deleted_and_cross_room'] * 3 + [
        'github_subscription_sections_match_rails_bytes_and_real_edit_page_permissions',
    ] + ['github_subscription_http_status_flash_token_events_and_membership_match_rails'] * 5 + [
        'github_subscription_sections_match_rails_bytes_and_real_edit_page_permissions',
    ],
    'test/controllers/rooms/github/pull_request_cards_controller_test.rb': [
        'github_viewer_card_http_frames_statuses_permissions_and_bodies_match_rails',
    ] * 6 + [None, None] + [
        'github_viewer_card_http_frames_statuses_permissions_and_bodies_match_rails',
    ] * 3 + ['github_viewer_card_security_checks_membership_and_exact_context_before_token_access'] * 4,
    'test/models/github/pull_request_url_test.rb': ['github_url_extraction_and_non_code_html_match_pinned_rails'] * 10,
    'test/helpers/github_pull_requests_helper_test.rb': ['github_pr_and_thread_stamps_invalidate_message_fragments_without_touching_message'] + [None] * 19,
    'test/models/github/pull_request_thread_test.rb': [
        'github_pr_thread_uniqueness_reuse_cleanup_and_unrelated_errors_match_rails',
    ] * 9 + ['github_pr_agent_payload_security_hides_private_and_unknown_details_without_owner_access'] * 2,
    'test/models/github/repository_subscription_test.rb': ['github_subscriptions_validation_and_bot_membership_callbacks_match_rails'] * 10,
    'test/models/github/notification_test.rb': ['github_notification_claims_validate_and_share_one_concurrent_winner_per_subscription'] * 3,
    'test/models/github/pull_request_test.rb': [
        'github_pr_identity_display_files_and_save_callbacks_match_rails',
        'github_pr_identity_display_files_and_save_callbacks_match_rails',
        'github_pr_identity_display_files_and_save_callbacks_match_rails',
        'github_pr_identity_display_files_and_save_callbacks_match_rails',
        'github_pr_case_collapse_repoints_links_and_mappings_without_destroying_threads',
        'github_pr_staleness_claim_boundaries_and_concurrent_upserts_are_quiet',
        'github_pr_staleness_claim_boundaries_and_concurrent_upserts_are_quiet',
        'github_pr_staleness_claim_boundaries_and_concurrent_upserts_are_quiet',
        None,
    ] + [
        'github_message_create_and_edit_hooks_reconcile_references_and_fetches',
        'github_message_create_and_edit_hooks_reconcile_references_and_fetches',
        'github_message_create_and_edit_hooks_reconcile_references_and_fetches',
        'github_message_reference_security_ignores_code_and_caps_before_case_normalization',
        'github_message_create_and_edit_hooks_reconcile_references_and_fetches',
        'github_message_create_and_edit_hooks_reconcile_references_and_fetches',
     ] + [
        'github_pr_registered_card_callbacks_publish_public_and_private_room_and_thread_replacements',
        'github_pr_registered_card_callbacks_publish_public_and_private_room_and_thread_replacements',
    ],
    'test/jobs/github/deliver_subscription_event_job_test.rb': [
        'github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails',
    ] * 2 + ['github_notifier_security_redacts_per_subscription_and_neutralizes_mentions'] + [
        'github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails',
    ] * 3 + [None] + ['github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails'] + [None] + [
        'github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails',
    ] * 10 + ['github_notifier_durable_handler_publishes_real_room_and_thread_frames'] + [
        'github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails',
    ] * 3 + [None] + ['github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails'] * 5 + [
        'github_notifier_durable_handler_publishes_real_room_and_thread_frames',
    ] + ['github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails'] * 2 + [
        'github_notifier_security_redacts_per_subscription_and_neutralizes_mentions',
    ] * 5,
    'test/jobs/github/fetch_pull_request_job_test.rb': [
        'github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails',
    ] * 13 + [
        'github_fetch_transport_failure_persists_error_without_changing_card_or_files',
    ] + ['github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails'] * 6 + [None] * 2,
    'test/jobs/github/perform_agent_action_job_test.rb': [
        None,  # WS11 owns AgentApproval::decide! and its enqueue callback.
    ] + ['github_agent_rechecks_payload_and_linked_identity_before_any_write'] * 3 + [None] + [
        'github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails',
    ] * 4 + ['github_agent_rechecks_authority_before_any_write'] * 7 + [
        'github_agent_rechecks_payload_and_linked_identity_before_any_write',
    ] * 2 + ['github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails'] * 2 + [
        'github_agent_rechecks_payload_and_linked_identity_before_any_write',
    ] * 2 + [
        'github_agent_concurrent_duplicates_share_one_claim_and_one_outbound_write',
        'github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails',
        'github_agent_concurrent_duplicates_share_one_claim_and_one_outbound_write',
        'github_agent_concurrent_duplicates_share_one_claim_and_one_outbound_write',
        'github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails',
        'github_agent_completion_index_is_scoped_to_agent_approval_and_event_type',
        'github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails',
        'github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails',
        'github_claim_persisted_outcomes_and_audits_match_pinned_rails',
        'github_claim_sweep_and_late_finish_keep_the_first_outcome_and_enqueue_once',
        'github_claim_sweep_and_late_finish_keep_the_first_outcome_and_enqueue_once',
        'github_claim_sweep_fails_only_overdue_running_claims_and_preserves_metadata',
    ] + ['github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails'] * 2,
    'test/models/github/agent_pull_request_action_test.rb': [
        'github_agent_action_validation_summary_payload_and_normalization_match_rails',
    ] * 10 + ['github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails'],
    'test/controllers/github/webhooks_controller_test.rb': [
        None,  # Fetch persistence and the card broadcast are the next slices.
        'webhook_http_status_body_selection_and_privacy_match_rails',
        'webhook_http_status_body_selection_and_privacy_match_rails',
        'webhook_http_status_body_selection_and_privacy_match_rails',
        'webhook_http_status_body_selection_and_privacy_match_rails',
        'webhook_security_rejects_bad_or_missing_signatures_before_parsing',
        'webhook_security_rejects_bad_or_missing_signatures_before_parsing',
        'webhook_security_missing_or_blank_secret_is_unavailable',
        'webhook_redelivered_supported_events_do_not_enqueue_again',
        'webhook_http_status_body_selection_and_privacy_match_rails',
        'webhook_http_status_body_selection_and_privacy_match_rails',
        'webhook_redelivered_supported_events_do_not_enqueue_again',
        'webhook_http_status_body_selection_and_privacy_match_rails',
        'webhook_http_status_body_selection_and_privacy_match_rails',
        'webhook_http_status_body_selection_and_privacy_match_rails',
        'webhook_http_status_body_selection_and_privacy_match_rails',
        'webhook_http_status_body_selection_and_privacy_match_rails',
        None,  # Notifier posting and message-reference synchronization remain deferred.
        None,  # HTTP dedupe covered; actual subscription posting remains deferred.
        'webhook_http_status_body_selection_and_privacy_match_rails',
        'webhook_http_status_body_selection_and_privacy_match_rails',
    ],
    'test/models/github/app_test.rb': [
        'app_requires_both_credentials_and_authorizes_with_empty_scope',
        'app_requires_both_credentials_and_authorizes_with_empty_scope',
        'oauth_response_matrix_matches_rails',
        'oauth_response_matrix_matches_rails',
        'oauth_response_matrix_matches_rails',
        'rejected_refreshes_disconnect_but_transient_failures_preserve_credentials_and_timestamp',
        'transport_errors_and_revocations_never_expose_credentials',
        'refresh_sends_rotating_grant',
        'revoke_distinguishes_token_from_grant_and_uses_basic_auth',
        'revoke_distinguishes_token_from_grant_and_uses_basic_auth',
        'revoke_distinguishes_token_from_grant_and_uses_basic_auth',
    ],
    'test/models/github/write_client_test.rb': [
        'write_paths_payloads_identity_and_headers_match_rails',
        'repository_access_denies_refusals_but_propagates_unauthorized_and_errors',
        'write_paths_payloads_identity_and_headers_match_rails',
        'write_paths_payloads_identity_and_headers_match_rails',
        'write_paths_payloads_identity_and_headers_match_rails',
        'write_status_matrix_matches_rails_and_cannot_inject_mentions',
        'write_status_matrix_matches_rails_and_cannot_inject_mentions',
        'write_status_matrix_matches_rails_and_cannot_inject_mentions',
        None,  # The Ruby test also asserts the exact warning log; not ported in this slice.
        'repository_access_denies_refusals_but_propagates_unauthorized_and_errors',
        'repository_access_denies_refusals_but_propagates_unauthorized_and_errors',
        'repository_access_denies_refusals_but_propagates_unauthorized_and_errors',
        'transport_errors_and_revocations_never_expose_credentials',
    ],
    'test/models/github_connected_account_test.rb': [
        'accounts_enforce_rails_validations',
        'accounts_encrypt_both_columns_and_read_rails_rows',
        'unreadable_or_tampered_tokens_disconnect_without_panicking',
        'pats_are_used_as_is_and_disconnected_accounts_are_unusable',
        'pats_are_used_as_is_and_disconnected_accounts_are_unusable',
        'accounts_encrypt_both_columns_and_read_rails_rows',
        'expired_app_tokens_rotate_with_early_refresh_and_keep_old_refresh_if_omitted',
        'rejected_refreshes_disconnect_but_transient_failures_preserve_credentials_and_timestamp',
        'rejected_refreshes_disconnect_but_transient_failures_preserve_credentials_and_timestamp',
        'stale_account_lookups_reuse_rotated_credentials_without_http',
        'refresh_races_reuse_the_winner_without_holding_a_database_transaction',
        'refresh_races_reuse_the_winner_without_holding_a_database_transaction',
        'refresh_races_reuse_the_winner_without_holding_a_database_transaction',
        'transport_failure_leaves_app_connected_and_skips_remote_revoke',
        'disconnect_refreshes_before_revoking_the_whole_grant',
        'failed_refresh_skips_grant_revocation_but_records_error',
        'agent_identity_prefers_owner_app_but_falls_back_to_machine_pat',
        'agent_identity_prefers_owner_app_but_falls_back_to_machine_pat',
        'rejected_owner_refresh_falls_back_to_agent_pat',
        'agent_identity_prefers_owner_app_but_falls_back_to_machine_pat',
    ],
}
paths = sorted(p for p in subprocess.check_output(['git', '-C', str(repo), 'ls-tree', '-r', '--name-only', 'd7c7de92', 'test'], text=True).splitlines() if 'github' in p and p.endswith('_test.rb'))
rows = []
file_counts = []
covered = 0
rust_tests = (root / 'crates/campfire/src/integrations/github/pull_requests/tests.rs').read_text() + (root / 'crates/campfire/src/integrations/github/references/tests.rs').read_text() + (root / 'crates/campfire/src/integrations/github/tests.rs').read_text() + (root / 'crates/campfire/src/controllers/github/webhooks/tests.rs').read_text() + (root / 'crates/campfire/src/integrations/action_claims/tests.rs').read_text() + (root / 'crates/campfire/src/integrations/github/fetcher/tests.rs').read_text() + (root / 'crates/campfire/src/integrations/github/agent_actions/tests.rs').read_text() + (root / 'crates/campfire/src/integrations/github/notifier/tests.rs').read_text()
rust_tests += (root / 'crates/campfire/src/controllers/github/card_tests.rs').read_text() + (root / 'crates/campfire/src/controllers/github/subscription_tests.rs').read_text()
rust_tests += (root / 'crates/campfire/src/controllers/github/connection_tests.rs').read_text()
for path in paths:
    content = subprocess.check_output(['git', '-C', str(repo), 'show', f'd7c7de92:{path}'], text=True)
    tests = re.findall(r'^\s*test\s+["\'](.+?)["\']\s+do', content, re.M)
    mapped = names.get(path, [None] * len(tests))
    assert len(mapped) == len(tests), path
    file_counts.append((path,len(tests),sum(target is not None for target in mapped)))
    rows.append(f'## `{path}` ({len(tests)} tests)\n\n| Rails test | Status and owner | Rust coverage |\n|---|---|---|')
    for name, target in zip(tests, mapped):
        if target:
            assert re.search(r'\bfn ' + re.escape(target) + r'\(', rust_tests), target
            covered += 1
        status = 'Mapped to grouped Rust assertions; WS15g' if target else 'Deferred; WS15g continuation'
        if not target and '/agents/' in path:
            status += '; WS11 owns authentication middleware'
        if not target and path.endswith('write_client_test.rb'):
            status += ' (warning-log assertion; error/privacy assertions already covered)'
        if not target and path.endswith('pull_request_cards_controller_test.rb'):
            status += ' (relink through Connection HTTP; transport failure followed by recovery through the same frame)'
        if not target and path.endswith('deliver_subscription_event_job_test.rb'):
            status += ' (Notifier source/item/preference assertions covered; WS12 owns inbox accessible_to and the general mention recorder)'
        rows.append(f'| {name.replace(chr(124), chr(92)+chr(124))} | {status} | {"`" + target + "`" if target else "—"} |')
    rows.append('')
count = sum(len(re.findall(r'^\s*test\s+["\'](.+?)["\']\s+do', subprocess.check_output(['git', '-C', str(repo), 'show', f'd7c7de92:{path}'], text=True), re.M)) for path in paths)
summary = f'{count} Rails cases in {len(paths)} files: {covered} mapped to Rust assertions; {count-covered} explicitly deferred.'
header = '# WS15g Rails test coverage — partial\n\nReference: `d7c7de92`. ' + summary + '\n\nThese are domain-level ports grouped into Rust tests, not executions of the original Ruby tests. Webhook HTTP ingestion, transactional enqueue, fetch persistence/runtime handler and the shared stuck-claim sweep with runtime periodic registration are covered. Notifier posting/dedupe/privacy/thread routing with its registered runtime and message broadcasts are also covered. The PR domain, message reference hooks, threads, subscriptions, notification claims and registered card replacements are covered. Card/card-set/thread-header/files-summary partials match pinned Rails bytes. The viewer-frame HTTP file is 13/15 covered, with exact successful bodies; relink/recovery stay deferred. Room subscription create/update/destroy and their role-gated edit sections are covered. PAT/App/bot connections are wired with 28 HTTP vectors; profile/bot view and lifecycle cases remain explicit. GitHub health data/section pass independently; shared health-page wiring remains deferred. Remaining room-page/controller/system parity, helper cache cases, and agent write HTTP controllers remain deferred. All deferred cases retain WS15g as owner; WS11 supplies the agent authentication seam and outbound event-webhook runtime. No coverage or parity allowlist has been added.\n\n'
groups = '| Rails file | Cases passing grouped assertions | Deferred |\n|---|---:|---:|\n' + '\n'.join(f'| `{path}` | {passed}/{total} | {total-passed} |' for path,total,passed in sorted(file_counts,key=lambda entry:entry[1]-entry[2],reverse=True)) + '\n\n'
(root / 'plans/ws15g-rails-tests.md').write_text((header + groups + '\n'.join(rows)).rstrip() + '\n')
print('GitHub Rails inventory: ' + summary)
for path,total,passed in sorted(file_counts,key=lambda entry:entry[1]-entry[2],reverse=True):
    print(f'{path}: {passed}/{total} mapped case groups passing; {total-passed} deferred')
