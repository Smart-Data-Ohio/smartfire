#!/usr/bin/env python3
"""Verify the real huddle-source regressions reject a missing issuance callback."""
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[1]
source = root / 'crates/db/src/models/huddle_grant.rs'
before = source.read_text()
needle = 'crate::models::huddle_invitations::after_issued(tx, &issued, previous_issue)?;'
assert before.count(needle) == 1
command = ['cargo', 'nextest', 'run', '--locked', '--manifest-path', str(root/'Cargo.toml'),
           '-p', 'campfire_db', '-j', '4', '-E', 'test(huddle_cutover_test)']
env = dict(os.environ, CARGO_BUILD_JOBS='2')
baseline = subprocess.run(command, cwd=root.parent, env=env, capture_output=True, text=True)
assert baseline.returncode == 0, baseline.stdout + baseline.stderr
print('WS17 huddle cutover baseline: 2 passed; 0 failures', flush=True)
try:
    source.write_text(before.replace(needle, 'let _ = previous_issue; // deliberately missing source callback'))
    broken = subprocess.run(command, cwd=root.parent, env=env, capture_output=True, text=True)
    output = broken.stdout + broken.stderr
    assert broken.returncode != 0 and output.count("real issuance must create the recipient's invitation") == 2, output
    assert '2 failed' in output and 'Build failed' not in output, output
    print('WS17 huddle cutover discrimination: 2 missing-source failures rejected; 0 invalid controls', flush=True)
finally:
    source.write_text(before)
