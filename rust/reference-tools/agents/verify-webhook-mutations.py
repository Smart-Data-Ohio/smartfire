#!/usr/bin/env python3
"""Require compiled assertion failures for the webhook and posting security contracts."""
from pathlib import Path
import os
import re
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root.parent / '.scratch' / 'ws11-webhook-mutations'
scratch.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, TMPDIR=str(root.parent / '.scratch'), CARGO_TARGET_DIR=str(root / 'target'),
           CABLE_TEST_PORT_RANGE='52200-52249', MAIL_TEST_PORT_RANGE='52200-52249',
           INTEGRATION_TEST_PORT_RANGE='52250-52299')
mutations = [
    ('private-guard', 'campfire', 'crates/campfire/src/integrations/webhook.rs',
     'crate::integrations::net::guard::resolve_webhook(&*net.resolver, &host).await?', '"10.0.0.7".parse().unwrap()',
     'ws11_blocks_private_webhooks_before_connecting'),
    ('dns-pinning', 'campfire', 'crates/campfire/src/integrations/webhook.rs',
     'pinned_ip: Some(address)', 'pinned_ip: None', 'ws11_pins_public_dns_answer_and_sends_timestamp'),
    ('signature', 'campfire', 'crates/campfire/src/integrations/webhook.rs',
     'smartfire_headers(secret, payload.as_bytes(), now)', 'smartfire_headers(None, payload.as_bytes(), now)',
     'ws11_signed_requests_match_rails_vectors'),
    ('signature-body', 'campfire', 'crates/campfire/src/integrations/webhook.rs',
     'smartfire_headers(secret, payload.as_bytes(), now)', 'smartfire_headers(secret, b"{}", now)',
     'ws11_signed_requests_match_rails_vectors'),
    ('pinned-surfguard-version', 'campfire', 'crates/campfire/src/integrations/webhook.rs',
     'guard::resolve_webhook(&*net.resolver', 'guard::resolve(&*net.resolver', 'ws11_http_guard_matches_rails_vectors'),
    ('2xx-only', 'campfire', 'crates/campfire/src/integrations/webhook.rs',
     'if !(200..300).contains(&status)', 'if false', 'ws11_error_responses_never_become_replies'),
    ('agent-timeout', 'campfire', 'crates/campfire/src/integrations/webhook.rs',
     'if !agent =>', 'if true =>', 'ws11_agent_timeouts_propagate_while_legacy_gets_a_root_reply'),
    ('legacy-transient-source', 'campfire', 'crates/campfire/src/integrations/webhook.rs',
     'Http(#[source] HttpError)', 'Http(HttpError)', 'ws11_legacy_webhook_retry_policy_keeps_transient_sources'),
    ('agent-fanout-exclusion', 'campfire', 'crates/db/src/models/bot_webhook_fanout.rs',
     'SELECT 1 FROM agents WHERE user_id = ?', 'SELECT 1 FROM agents WHERE user_id = ? AND 1=0',
     'ws11_agent_backed_bots_never_receive_legacy_webhook_jobs'),
    ('hop-limit', 'campfire_db', 'crates/db/src/models/bot_webhook_fanout.rs',
     'hop_for_message(tx, message)? >= HOP_LIMIT', 'hop_for_message(tx, message)? >= 99',
     'ws11_fanout_hops_follow_authorized_agent_events_and_legacy_reply_sources'),
    ('secret-stale-read', 'campfire', 'crates/db/src/models/webhook.rs',
     '*self = query_one(tx.conn(), "SELECT webhooks.* FROM webhooks WHERE id = ? LIMIT 1", [self.id], Self::from_row)?.ok_or(Error::RecordNotFound("Webhook"))?;', '',
     'ws11_webhook_secrets_reload_encrypt_and_rotate'),
    ('secret-encoding', 'campfire', 'crates/db/src/models/webhook.rs',
     'encryption.encrypt_with_encoding(secret.as_bytes(), "US-ASCII")', 'encryption.encrypt_with_encoding(secret.as_bytes(), "UTF-8")',
     'ws11_webhook_secrets_reload_encrypt_and_rotate'),
    ('sync-reply-link', 'campfire', 'crates/campfire/src/integrations/jobs.rs',
     'attributes.reply_to_message_id = trigger.map(|message| message.id);', 'attributes.reply_to_message_id = None;',
     'ws11_sync_replies_use_root_links_threads_boards_and_locking'),
    ('sync-reply-thread', 'campfire', 'crates/campfire/src/integrations/jobs.rs',
     'trigger.and_then(|message| message.thread_id)', 'trigger.and_then(|_| None)',
     'ws11_sync_replies_use_root_links_threads_boards_and_locking'),
    ('posting-budget', 'campfire', 'crates/db/src/models/agent_posting.rs',
     'if usage < limit {', 'if true {', 'ws11_bot_key_message_budget_overflow_notifies_once'),
    ('posting-replay', 'campfire', 'crates/db/src/models/agent_posting.rs',
     '.filter(|id| !campfire_richtext::ruby::is_blank(id))', '.filter(|_| false)',
     'ws11_bot_key_idempotency_replay_precedes_budget'),
    ('notice-once', 'campfire', 'crates/db/src/models/agent_posting.rs',
     'ON CONFLICT(agent_id,cap,day) DO NOTHING RETURNING id', 'ON CONFLICT(agent_id,cap,day) DO UPDATE SET updated_at=excluded.updated_at RETURNING id',
     'ws11_bot_key_message_budget_overflow_notifies_once'),
    ('board-opener-budget', 'campfire_db', 'crates/db/src/models/agent_posting.rs',
     ' AND board_post_opener=0', '', 'ws11_message_budget_excludes_board_openers_and_counts_local_days'),
    ('budget-time-zone', 'campfire_db', 'crates/db/src/models/agent_posting.rs',
     'Ok(time_parser::zone(name.as_deref().unwrap_or("UTC")))', 'Ok(TimeZone::UTC)',
     'ws11_message_budget_excludes_board_openers_and_counts_local_days'),
]
clock_path = root / 'crates/campfire/src/integrations/webhook.rs'
clock_source = clock_path.read_text()
clock_start = clock_source.index('    let address = crate::integrations::net::guard::resolve_webhook')
clock_end = clock_source.index('    let headers = rails_compat::webhook::smartfire_headers', clock_start)
clock_before = clock_source[clock_start:clock_end]
clock_after = '    let now = now();\n' + clock_before.replace('    let now = now();\n', '')
posting_source = (root / 'crates/campfire/src/controllers/messages/by_bots.rs').read_text()
posting_start = posting_source.index('    let preflight = ')
posting_end = posting_source.index('    let mut attributes = message_params(c)?;', posting_start)
mutations.extend([
    ('timestamp-after-dns', 'campfire', 'crates/campfire/src/integrations/webhook.rs', clock_before, clock_after, 'ws11_timestamp_is_sampled_after_resolution'),
    ('posting-before-attachment', 'campfire', 'crates/campfire/src/controllers/messages/by_bots.rs', posting_source[posting_start:posting_end], '', 'ws11_replay_and_budget_precede_attachment_validation'),
])
for name, package, relative, before, after, test in mutations:
    path = root / relative
    original = path.read_text()
    if before not in original:
        raise RuntimeError(f'{name}: mutation anchor missing')
    try:
        path.write_text(original.replace(before, after))
        cmd = ['mise', 'exec', 'rust@1.98.1', '--', 'cargo', 'test', '--locked', '-j', '4', '-p', package, test, '--', '--nocapture']
        result = subprocess.run(cmd, cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        (scratch / f'{name}.log').write_text(result.stdout)
        summary = re.search(r'^test result: FAILED\..*$', result.stdout, re.M)
        if result.returncode != 101 or summary is None or f'{test} ... FAILED' not in result.stdout or 'error[E' in result.stdout:
            raise RuntimeError(f'{name}: regression was not detected; see {scratch/name}.log')
        print(f'{name}: {summary.group()}', flush=True)
    finally:
        path.write_text(original)
print(f'WS11 webhook/posting discrimination: {len(mutations)} compiled regressions detected; sources restored')
