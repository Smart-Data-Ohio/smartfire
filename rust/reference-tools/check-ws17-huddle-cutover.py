#!/usr/bin/env python3
"""Reject missing issuance callbacks and policy-conditional missing invitations."""
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[1]
source = root / 'crates/db/src/models/huddle_grant.rs'
invitations = root / 'crates/db/src/models/huddle_invitations.rs'
before = source.read_text()
before_invitations = invitations.read_text()
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
needle = 'for recipient in recipients {'
assert before_invitations.count(needle) == 1
try:
    invitations.write_text(before_invitations.replace(needle, needle + '''
        let policy = UserStatusSettings::find(tx.conn(), recipient.id)?;
        if policy.dnd_enabled || policy.quiet_hours_enabled { continue; }
'''))
    broken = subprocess.run(command, cwd=root.parent, env=env, capture_output=True, text=True)
    output = broken.stdout + broken.stderr
    assert broken.returncode != 0 and output.count("real issuance must create the recipient's invitation") == 1, output
    assert '1 passed' in output and '1 failed' in output and 'Build failed' not in output, output
    print('WS17 huddle policy discrimination: 1 conditional missing-invitation failure rejected; DND-after-issuance still passes; 0 invalid controls', flush=True)
finally:
    invitations.write_text(before_invitations)
