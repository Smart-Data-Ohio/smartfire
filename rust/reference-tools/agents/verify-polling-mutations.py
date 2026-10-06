#!/usr/bin/env python3
"""Require real assertion failures for polling privacy and cursor regressions."""
from pathlib import Path
import os
import re
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root.parent / '.scratch' / 'ws11-polling-mutations'
scratch.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, CI='1', TMPDIR=str(root.parent / '.scratch'), CARGO_TARGET_DIR=str(root / 'target'))
path = root / 'crates/db/src/models/agent_event_polling.rs'
mutations = [
    ('cursor-dropped-page', 'events.last().map_or(since, |e| e.id)', 'since'),
    ('inactive-agent', 'self.active &&', 'true &&'),
    ('revoked-grant', 'self.legacy || self.grants.contains(&None)', 'true || self.grants.contains(&None)'),
    ('foreign-approval', 'a.id=? AND a.agent_id=?', 'a.id=? AND ? IS NOT NULL'),
    ('approval-metadata-only', 'e.metadata.get("approval_id").map(ruby_i64)', 'e.agent_approval_id.or_else(|| e.metadata.get("approval_id").map(ruby_i64))'),
    ('assigned-deleted-thread', 'e.event_type!="work_unassigned"', 'false'),
]
for name, before, after in mutations:
    original = path.read_text()
    pattern = r'\s*'.join(re.escape(char) for char in before if not char.isspace())
    if not re.search(pattern, original):
        raise RuntimeError(f'{name}: missing anchor')
    try:
        path.write_text(re.sub(pattern, lambda _: after, original))
        result = subprocess.run(['cargo', 'test', '--locked', '-j4', '-p', 'campfire_db', 'ws11_poll', '--', '--nocapture'], cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        (scratch / f'{name}.log').write_text(result.stdout)
        summary = re.search(r'^test result: FAILED\..*$', result.stdout, re.M)
        if result.returncode != 101 or not summary or 'error[E' in result.stdout:
            raise RuntimeError(f'{name}: no compiled assertion failure')
        print(f'{name}: {summary.group()}', flush=True)
    finally:
        path.write_text(original)
print(f'WS11 polling discrimination: {len(mutations)} compiled regressions detected; sources restored')
