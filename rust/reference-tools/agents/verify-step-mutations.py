#!/usr/bin/env python3
"""Compile real regressions in parent ownership, grants, limits and broadcasts."""
from pathlib import Path
import os
import re
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root.parent / '.scratch' / 'ws11-step-mutations'
scratch.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, CI='1', TMPDIR=str(root.parent / '.scratch'),
           CARGO_TARGET_DIR=str(root / 'target'),
           INTEGRATION_TEST_PORT_RANGE='52250-52299',
           CABLE_TEST_PORT_RANGE='52200-52249', MAIL_TEST_PORT_RANGE='52200-52249')
mutations = [
    ('message-owner', 'Some(message.creator_id)!=user', 'false', 'ws11_step_model'),
    ('thread-owner', 'thread.work_owner_id!=user', 'false', 'ws11_step_model'),
    ('parent-cap', 'count>=50', 'count>=51', 'ws11_step_parent_limit'),
    ('create-grant', 'if !super::agent_access::capability_for_agent(tx.conn(),agent_id,capability,Some(room))?', 'if false', 'ws11_step_services'),
    ('update-grant', 'if !accessible', 'if false', 'ws11_step_services'),
    ('parent-broadcast', 'step.broadcast_parent(tx)?;', '', 'ws11_step_services'),
]
path = root / 'crates/db/src/models/agent_step.rs'
for name, before, after, test in mutations:
    original = path.read_text()
    pattern = r'\s*'.join(re.escape(c) for c in before if not c.isspace())
    if not re.search(pattern, original):
        raise RuntimeError(f'{name}: missing anchor')
    try:
        path.write_text(re.sub(pattern, lambda _: after, original))
        result = subprocess.run(['mise', 'exec', 'rust@1.98.1', '--', 'cargo',
                                 'test', '--locked', '-j4', '-p', 'campfire_db',
                                 test, '--', '--nocapture'], cwd=root, env=env,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        (scratch / f'{name}.log').write_text(result.stdout)
        summary = re.search(r'^test result: FAILED\..*$', result.stdout, re.M)
        if result.returncode != 101 or not summary or 'error[E' in result.stdout:
            raise RuntimeError(f'{name}: no compiled assertion failure')
        print(f'{name}: {summary.group()}', flush=True)
    finally:
        path.write_text(original)
print(f'WS11 step discrimination: {len(mutations)} compiled regressions detected; sources restored')
