#!/usr/bin/env python3
"""Reject real omitted-owner and stale-instance model mutants; restore source in finally."""
from pathlib import Path
import os
import subprocess

ROOT = Path(__file__).resolve().parents[3]
output = ROOT / '.scratch/ws8bm-next'
output.mkdir(parents=True, exist_ok=True)
source = ROOT / 'rust/crates/db/src/models/channel_thread/work.rs'
original = source.read_text()
variants = [
    ('omitted-owner', 'if (changes.owner_id.is_some() || before.work_status.is_some() != status.is_some())',
     'if changes.owner_id.is_some()', 'message_controller_model_refuses_untracking_with_owner_omitted_like_rails'),
    ('stale-history', 'let mut fresh = Self::find(tx.conn(), id)?;', 'let mut fresh = self.clone();',
     'message_controller_separate_stale_work_changes_match_rails_history'),
]
env = dict(os.environ, CARGO_BUILD_JOBS='2', RUST_TEST_THREADS='8')
try:
    for name, needle, replacement, test in variants:
        assert original.count(needle) == 1, (name, original.count(needle))
        source.write_text(original.replace(needle, replacement, 1))
        result = subprocess.run([
            'cargo', 'test', '--locked',
            '--manifest-path', 'rust/Cargo.toml', '-p', 'campfire_db', test, '--', '--test-threads=8',
        ], cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        (output / f'model-mutant-{name}.log').write_text(result.stdout)
        assert result.returncode and 'test result: FAILED. 0 passed; 1 failed;' in result.stdout, result.stdout[-6000:]
        print(f'WS8bm direct-model discrimination: {name}: real model mutant REJECTED (1 assertion failure)', flush=True)
finally:
    source.write_text(original)
