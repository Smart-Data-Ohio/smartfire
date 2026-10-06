#!/usr/bin/env python3
"""Require compiled assertion failures for cleanup regressions, then restore sources."""
from pathlib import Path
import os
import re
import subprocess
ROOT = Path(__file__).resolve().parents[3]
SCRATCH = ROOT / '.scratch' / 'ws11-credential-case-mutations'
SCRATCH.mkdir(parents=True, exist_ok=True)
ENV = dict(os.environ, CI='1', TMPDIR=str(ROOT / '.scratch'), CARGO_TARGET_DIR=str(ROOT / 'rust/target'))
MUTATIONS = [
    ('revoked-auth', 'agent_credential.rs', 'AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at>?)', 'AND (expires_at IS NULL OR expires_at>?)', 'ws11_credential_case_revoked'),
    ('expired-auth', 'agent_credential.rs', '(expires_at IS NULL OR expires_at>?)', '(expires_at IS NULL OR expires_at<=?)', 'ws11_credential_case_expired'),
    ('use-throttle', 'agent_credential.rs', '(last_used_at IS NULL OR last_used_at<=?)', '(last_used_at IS NULL OR last_used_at IS NOT NULL OR last_used_at<=?)', 'ws11_credential_case_use_throttle'),
    ('grant-active-scope', 'agent_grant.rs', 'WHERE revoked_at IS NULL ORDER BY id', 'WHERE revoked_at IS NOT NULL ORDER BY id', 'ws11_grant_case_active_and_revoked'),
]
for name, filename, before, after, test in MUTATIONS:
    path = ROOT / 'rust/crates/db/src/models' / filename
    original = path.read_text()
    pattern = r'\s*'.join(re.escape(char) for char in before if not char.isspace())
    start = 0
    changed, count = re.subn(pattern, lambda _: after, original[start:], count=1)
    changed = original[:start] + changed
    assert count == 1, name
    try:
        path.write_text(changed)
        output = subprocess.run(['cargo', 'test', '--locked', '-j4', '--manifest-path', 'rust/Cargo.toml', '-p', 'campfire_db', test, '--', '--nocapture'], cwd=ROOT, env=ENV, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (SCRATCH / f'{name}.log').write_text(output.stdout)
        assert output.returncode != 0 and 'test result: FAILED.' in output.stdout and 'could not compile' not in output.stdout, name
        print(name + ': ' + next(line for line in output.stdout.splitlines() if line.startswith('test result: FAILED.')), flush=True)
    finally:
        path.write_text(original)
print('WS11 credential/grant case discrimination: 4 compiled regressions detected; sources restored')
