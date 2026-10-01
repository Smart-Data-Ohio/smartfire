#!/usr/bin/env python3
"""Prove the seed fragment checks reject missing UI and viewer-policy leaks."""
from pathlib import Path
import os
import subprocess
root = Path(__file__).resolve().parents[2]
scratch = root.parent / '.scratch/ws15g'
scratch.mkdir(parents=True, exist_ok=True)
env = os.environ | {'CI': '1', 'CABLE_TEST_PORT_RANGE': '51500-51549', 'TMPDIR': str(scratch), 'CARGO_TARGET_DIR': str(root / 'target')}
mutations = [
    ('thread-fragment-omitted', root / 'crates/campfire/src/controllers/presenters/github.rs',
     'Ok(campfire_views::github::thread_header(\n        ctx,\n        thread.room_id,\n        thread.id,\n        &data,\n    ))',
     'Ok({ let _ = (ctx, data); String::new() })', 'round2_seed_fragments_match', 1),
    ('bot-viewer-policy-bypassed', root / 'crates/views/src/github/connections.rs',
     'pub fn bot(data: &Connection, bot_id: i64, administrator: bool) -> String {',
     'pub fn bot(data: &Connection, bot_id: i64, administrator: bool) -> String {\n    let _ = administrator; let administrator = true;', 'round2_connection_fragments_load', 1),
]
for name, path, before, after, test, count in mutations:
    source = path.read_text()
    assert source.count(before) == 1, name
    try:
        path.write_text(source.replace(before, after))
        run = subprocess.run(['mise', 'exec', 'rust@1.98.1', '--', 'cargo', 'test', '--manifest-path', str(root / 'Cargo.toml'), '--locked', '-j4', '-p', 'campfire', test, '--', '--nocapture'], cwd=root.parent, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, timeout=300)
        (scratch / f'round2-mutation-{name}.log').write_text(run.stdout)
        summary = [line for line in run.stdout.splitlines() if line.startswith('test result:')]
        assert run.returncode != 0 and summary and f'{count} failed;' in summary[-1], run.stdout
        print(f'{name}: {summary[-1]}', flush=True)
    finally:
        path.write_text(source)
print('GitHub seed fragment mutations: 2 rejected; 0 survived', flush=True)
