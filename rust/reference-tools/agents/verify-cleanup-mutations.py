#!/usr/bin/env python3
"""Require compiled assertion failures for cleanup regressions, then restore sources."""
from pathlib import Path
import os
import re
import subprocess
ROOT = Path(__file__).resolve().parents[3]
SCRATCH = ROOT / '.scratch' / 'ws11-cleanup-mutations'
SCRATCH.mkdir(parents=True, exist_ok=True)
ENV = dict(os.environ, CI='1', TMPDIR=str(ROOT / '.scratch'), CARGO_TARGET_DIR=str(ROOT / 'rust/target'))
MUTATIONS = [
    ('grant-duplicate', 'agent_grant.rs', 'a.revoked_at.is_none()', 'false', 'ws11_cleanup_credential'),
    ('credential-validation', 'agent_credential.rs', '.into_result()?;', ';', 'ws11_cleanup_credential'),
    ('agent-grants', 'agent.rs', '"agent_credentials", "agent_grants", "agent_slash_commands", "agent_events"', '"agent_credentials", "agent_slash_commands", "agent_events"', 'ws11_cleanup_agent'),
    ('approval-inbox', 'agent.rs', 'approval.destroy(tx)?;', 'let _ = approval;', 'ws11_cleanup_agent'),
    ('backfill-kind', 'agent.rs', "SELECT id,NULL,'workspace',CURRENT_TIMESTAMP", "SELECT id,NULL,'personal',CURRENT_TIMESTAMP", 'ws11_cleanup_backfill'),
]
for name, filename, before, after, test in MUTATIONS:
    path = ROOT / 'rust/crates/db/src/models' / filename
    original = path.read_text()
    pattern = r'\s*'.join(re.escape(char) for char in before if not char.isspace())
    start = original.index('pub fn update(') if name == 'credential-validation' else 0
    changed, count = re.subn(pattern, lambda _: after, original[start:], count=1)
    changed = original[:start] + changed
    assert count == 1, name
    try:
        path.write_text(changed)
        output = subprocess.run(['mise', 'exec', 'rust@1.98.1', '--', 'cargo', 'test', '--locked', '-j4', '--manifest-path', 'rust/Cargo.toml', '-p', 'campfire_db', test, '--', '--nocapture'], cwd=ROOT, env=ENV, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (SCRATCH / f'{name}.log').write_text(output.stdout)
        assert output.returncode != 0 and 'test result: FAILED.' in output.stdout and 'could not compile' not in output.stdout, name
        print(name + ': ' + next(line for line in output.stdout.splitlines() if line.startswith('test result: FAILED.')), flush=True)
    finally:
        path.write_text(original)
print('WS11 cleanup discrimination: 5 compiled regressions detected; sources restored')
