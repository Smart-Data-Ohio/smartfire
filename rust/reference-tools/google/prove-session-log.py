#!/usr/bin/env python3
"""The real callback log assertion must fail if its rejection warning disappears."""
import os, subprocess
from pathlib import Path
root = Path(__file__).resolve().parents[2]
scratch = root.parent / '.scratch/ws14g/mutations'
scratch.mkdir(parents=True, exist_ok=True)
path = root / 'crates/campfire/src/controllers/google_sign_in.rs'
source = path.read_text()
warning = '    tracing::warn!("Google sign-in rejected: {reason}");\n'
assert source.count(warning) == 1
try:
    path.write_text(source.replace(warning, '', 1))
    result = subprocess.run(['cargo', 'test', '--locked', '-j', '4', '-p', 'campfire', 'google_sessions_rejection_log_names_reason', '--', '--nocapture'], cwd=root,
        env=dict(os.environ, CI='1', TMPDIR=str(scratch), CARGO_PROFILE_TEST_DEBUG='0', CARGO_PROFILE_DEV_DEBUG='0'),
        text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    (scratch / 'session-log-warning.log').write_text(result.stdout)
    lines = [line for line in result.stdout.splitlines() if line.startswith('test result:')]
    assert result.returncode == 101 and 'rejection log assertion:' in result.stdout and any('FAILED' in line for line in lines), result.stdout
    print('\n'.join(lines))
    print('Google session log discrimination: 1 mutation rejected')
finally:
    path.write_text(source)
