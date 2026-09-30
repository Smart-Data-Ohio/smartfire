#!/usr/bin/env python3
"""Compile the unadapted owner boundaries; the Rails byte assertion must reject them."""
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[3]
paths = [root / 'rust/crates/campfire/src/controllers/presenters' / name
         for name in ['room_list.rs', 'room_native.rs']]
originals = {path: path.read_bytes() for path in paths}
scratch = root / '.scratch'
scratch.mkdir(exist_ok=True)
env = dict(os.environ, CI='1', TMPDIR=str(scratch), CARGO_PROFILE_TEST_DEBUG='0',
           CARGO_PROFILE_DEV_DEBUG='0', CABLE_TEST_PORT_RANGE='52100-52149',
           MAIL_TEST_PORT_RANGE='52100-52149')
try:
    path = paths[0]
    source = path.read_text()
    changed = source.replace('.render().map(|list| format!("\\n    \\n{list}"))', '.render()')
    assert source != changed
    path.write_text(changed)
    path = paths[1]
    source = path.read_text()
    source = source.replace('let footer=format!("  {}", inline.strip_prefix(\'\\n\').unwrap_or(&inline));', 'let footer=inline;')
    changed = source.replace('let template=format!("{}\\n",campfire_views::channel_threads::PendingTemplate{ctx,user}.render()?);',
                             'let template=campfire_views::channel_threads::PendingTemplate{ctx,user}.render()?;')
    assert path.read_text() != changed
    path.write_text(changed)
    run = subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j4',
        '--manifest-path','rust/Cargo.toml','-p','campfire','--bin','campfire',
        'native_component_capture_matches_rails_root_selection','--','--test-threads=4','--nocapture'],
        cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    (scratch / 'native-boundary-discrimination.log').write_text(run.stdout)
    for line in run.stdout.splitlines():
        if line.startswith('test result:'):
            print(line)
    assert run.returncode != 0 and 'test result: FAILED. 0 passed; 1 failed;' in run.stdout, run.stdout[-6000:]
finally:
    for path, original in originals.items():
        path.write_bytes(original)
print('Native boundary discrimination: compiled unadapted owner seams rejected; source restored')
