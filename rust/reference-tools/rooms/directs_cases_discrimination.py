#!/usr/bin/env python3
"""Compile wrong group authorization and unbounded SQL selection; restore sources."""
from pathlib import Path
import os
import subprocess
root = Path(__file__).resolve().parents[3]
source = root / 'rust/crates/campfire/src/controllers/rooms/directs.rs'
original = source.read_text()
mutants = [
    ('if group && !require_current_user(c)?.is_administrator()',
     'if group && false && !require_current_user(c)?.is_administrator()',
     'controllers::rooms::directs_rails_cases::a_member_cannot_delete_a_group_dm'),
    ('.take(campfire_db::models::direct_room::MAX_MEMBERS)', '.take(usize::MAX)',
     'controllers::rooms::directs_rails_cases::create_caps_user_ids_before_querying'),
]
env = dict(os.environ, CI='1', TMPDIR=str(root / '.scratch'), CARGO_TARGET_DIR=str(root / 'rust/target'))
try:
    for old, new, test in mutants:
        assert original.count(old) == 1, old
        source.write_text(original.replace(old, new))
        result = subprocess.run(['mise', 'exec', 'rust@1.98.1', '--', 'cargo', 'test', '--locked', '-j4',
            '--manifest-path', str(root / 'rust/Cargo.toml'), '-p', 'campfire', test, '--', '--exact'],
            cwd=root, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        summaries = [line for line in result.stdout.splitlines() if line.startswith('test result:')]
        assert result.returncode and any('FAILED. 0 passed; 1 failed;' in line for line in summaries), result.stdout
        print('\n'.join(summaries), flush=True)
finally:
    source.write_text(original)
print('Direct Rails case discrimination: group authorization and uncapped real SQL rejected; source restored')
