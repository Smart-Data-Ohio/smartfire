#!/usr/bin/env python3
"""Compile wrong administrator and membership gates; restore the actual room controller."""
from pathlib import Path
import os
import subprocess
root = Path(__file__).resolve().parents[3]
source = root / 'rust/crates/campfire/src/controllers/rooms.rs'
original = source.read_text()
mutants = [
    ('if !allowed {', 'if false && !allowed {',
     'controllers::rooms::rooms_rails_cases::destroy_only_allowed_for_creators_or_those_who_can_administer'),
    ('Room::find_for_user(conn, user_id, id)', 'Room::find_by_id(conn, id)',
     'controllers::rooms::rooms_rails_cases::show_still_redirects_non_members_of_private_rooms'),
]
env = dict(os.environ, CI='1', TMPDIR=str(root / '.scratch'), CARGO_TARGET_DIR=str(root / 'rust/target'))
try:
    for old, new, test in mutants:
        assert old in original, old
        source.write_text(original.replace(old, new))
        result = subprocess.run(['cargo', 'test', '--locked', '-j4',
            '--manifest-path', str(root / 'rust/Cargo.toml'), '-p', 'campfire', test, '--', '--exact'],
            cwd=root, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        summaries = [line for line in result.stdout.splitlines() if line.startswith('test result:')]
        assert result.returncode and any('FAILED. 0 passed; 1 failed;' in line for line in summaries), result.stdout
        print('\n'.join(summaries), flush=True)
finally:
    source.write_text(original)
print('Room Rails case discrimination: administrator and private-room membership mutants rejected; source restored')
