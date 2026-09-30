#!/usr/bin/env python3
"""List every pinned Rails GitHub test and its coverage or explicit deferred owner."""
from pathlib import Path
import re
import subprocess

root = Path(__file__).resolve().parents[2]
repo = root.parent
names = {
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
covered = 0
rust_tests = (root / 'crates/campfire/src/integrations/github/tests.rs').read_text()
for path in paths:
    content = subprocess.check_output(['git', '-C', str(repo), 'show', f'd7c7de92:{path}'], text=True)
    tests = re.findall(r'^\s*test\s+["\'](.+?)["\']\s+do', content, re.M)
    mapped = names.get(path, [None] * len(tests))
    assert len(mapped) == len(tests), path
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
        rows.append(f'| {name.replace(chr(124), chr(92)+chr(124))} | {status} | {"`" + target + "`" if target else "—"} |')
    rows.append('')
count = sum(len(re.findall(r'^\s*test\s+["\'](.+?)["\']\s+do', subprocess.check_output(['git', '-C', str(repo), 'show', f'd7c7de92:{path}'], text=True), re.M)) for path in paths)
summary = f'{count} Rails cases in {len(paths)} files: {covered} mapped to Rust assertions; {count-covered} explicitly deferred.'
header = '# WS15g Rails test coverage — partial\n\nReference: `d7c7de92`. ' + summary + '\n\nThese are domain-level ports grouped into Rust tests, not executions of the original Ruby tests. HTTP routes, view/system parity, webhook ingestion, PR persistence/fetch jobs, notifications/subscriptions, agent actions and sweep wiring remain deferred. All deferred cases retain WS15g as owner; WS11 supplies the agent authentication seam. No coverage or parity allowlist has been added.\n\n'
(root / 'plans/ws15g-rails-tests.md').write_text((header + '\n'.join(rows)).rstrip() + '\n')
print('GitHub Rails inventory: ' + summary)
