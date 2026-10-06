#!/usr/bin/env python3
"""Require compiled failures for selection and acknowledgment access regressions."""
from pathlib import Path
import os
import re
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root.parent / '.scratch' / 'ws11-event-access-mutations'
scratch.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, CI='1', TMPDIR=str(root.parent / '.scratch'),
           CARGO_TARGET_DIR=str(root / 'target'), CABLE_TEST_PORT_RANGE='52200-52249',
           MAIL_TEST_PORT_RANGE='52200-52249', INTEGRATION_TEST_PORT_RANGE='52250-52299')
path = root / 'crates/db/src/models/agent_event_access.rs'
mutations = [
    ('selection-owner', 'e.agent_id=?1 AND', '?1 IS NOT NULL AND'),
    ('selection-revocation', 'g.revoked_at IS NULL', '1=1'),
    ('selection-membership',
     'AND EXISTS (SELECT 1 FROM memberships WHERE user_id=?3 AND room_id=m.room_id)',
     'AND (?3 IS NOT NULL)'),
    ('ack-owner', 'e.agent_id == agent_id', 'true'),
    ('ack-membership', 'if !membership {', 'if false {'),
    ('ack-webhook-isolation', "UPDATE agent_events SET outcome='acknowledged' WHERE id=?",
     "UPDATE agent_events SET outcome='acknowledged',webhook_status='delivered' WHERE id=?"),
]
for name, before, after in mutations:
    original = path.read_text()
    if before not in original:
        raise RuntimeError(f'{name}: missing anchor')
    try:
        path.write_text(original.replace(before, after))
        result = subprocess.run([
            'cargo', 'test', '--locked', '-j', '4',
            '-p', 'campfire_db', 'ws11_event_readability_and_ack_match_rails_access_matrix',
            '--', '--nocapture',
        ], cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        (scratch / f'{name}.log').write_text(result.stdout)
        summary = re.search(r'^test result: FAILED\..*$', result.stdout, re.M)
        if result.returncode != 101 or not summary or 'error[E' in result.stdout:
            raise RuntimeError(f'{name}: no compiled assertion failure')
        print(f'{name}: {summary.group()}', flush=True)
    finally:
        path.write_text(original)
print(f'WS11 event access discrimination: {len(mutations)} compiled regressions detected; sources restored')
