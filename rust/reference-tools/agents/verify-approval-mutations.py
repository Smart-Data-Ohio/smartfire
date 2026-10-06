#!/usr/bin/env python3
"""Discriminate approval authorization, expiry, size and settlement regressions."""
from pathlib import Path
import os
import re
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root.parent / '.scratch' / 'ws11-approval-mutations'
scratch.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, CI='1', TMPDIR=str(root.parent / '.scratch'), CARGO_TARGET_DIR=str(root / 'target'), INTEGRATION_TEST_PORT_RANGE='52250-52299', CABLE_TEST_PORT_RANGE='52200-52249', MAIL_TEST_PORT_RANGE='52200-52249')
mutations = [
    ('external-admin', 'agent_approval.rs', '!(self.github_action() || self.fizzy_action())', 'true', 'campfire_db', 'ws11_approval_authorization'),
    ('inactive-agent', 'agent_approval.rs', 'bot.status=0 AND', '1=1 AND', 'campfire_db', 'ws11_approval_decision_expiry'),
    ('settled-guard', 'agent_approval.rs', 'self.effective_status(now) != "pending"', 'false', 'campfire_db', 'ws11_approval_decision_expiry'),
    ('effective-expiry', 'agent_approval.rs', 'self.expires_at <= now', 'false', 'campfire_db', 'ws11_approval_decision_expiry'),
    ('payload-bytes', 'agent_approval.rs', 's.len() > 4096', 's.chars().count() > 4096', 'campfire_db', 'ws11_approval_validation'),
    ('decision-webhook', 'agent_approval.rs', 'enqueue_delivered_webhook(tx, &event);', '', 'campfire', 'ws11_approval_queue_failure'),
    ('service-grant', 'agent_approvals.rs', '!capability_for_agent(tx.conn(), agent_id, "external_action", request.room_id)?', 'false', 'campfire_db', 'ws11_approvals_service_replay'),
    ('service-replay', 'agent_approvals.rs', '!campfire_richtext::ruby::is_blank(external)', 'false', 'campfire_db', 'ws11_approvals_service_replay'),
    ('service-budget', 'agent_approvals.rs', 'check_budget(tx, agent_id, Cap::ExternalActions)?', 'None::<serde_json::Value>', 'campfire_db', 'ws11_approvals_service_replay'),
]
for name, filename, before, after, package, test in mutations:
    path = root / 'crates/db/src/models' / filename
    original = path.read_text()
    pattern = r'\s*'.join(re.escape(char) for char in before if not char.isspace())
    if not re.search(pattern, original):
        raise RuntimeError(f'{name}: missing anchor')
    try:
        path.write_text(re.sub(pattern, lambda _: after, original))
        result = subprocess.run(['cargo', 'test', '--locked', '-j4', '-p', package, test, '--', '--nocapture'], cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        (scratch / f'{name}.log').write_text(result.stdout)
        summary = re.search(r'^test result: FAILED\..*$', result.stdout, re.M)
        if result.returncode != 101 or not summary or 'error[E' in result.stdout:
            raise RuntimeError(f'{name}: no compiled assertion failure')
        print(f'{name}: {summary.group()}', flush=True)
    finally:
        path.write_text(original)
print(f'WS11 approval discrimination: {len(mutations)} compiled regressions detected; sources restored')
