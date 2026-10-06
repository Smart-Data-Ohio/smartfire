#!/usr/bin/env python3
"""Compile wrong closed-room creation/update permissions; restore the controller."""
from pathlib import Path
import os
import subprocess
root = Path(__file__).resolve().parents[3]
source = root / 'rust/crates/campfire/src/controllers/rooms/closeds.rs'
original = source.read_text()
mutants = [
    ('ensure_permission_to_create_rooms(c).await?;', '',
     'controllers::rooms::closeds_rails_cases::create_forbidden_by_non_admin_when_account_restricts_creation_to_admins'),
    ('ensure_can_administer(c, &room)?;', '',
     'controllers::rooms::closeds_rails_cases::only_admins_or_creators_can_update'),
]
env = dict(os.environ, CI='1', TMPDIR=str(root / '.scratch'), CARGO_TARGET_DIR=str(root / 'rust/target'), CABLE_TEST_PORT_RANGE='52100-52149')
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
print('Closed-room Rails case discrimination: creation and update authorization mutants rejected; source restored')
