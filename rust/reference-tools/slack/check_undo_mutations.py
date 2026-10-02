#!/usr/bin/env python3
"""Require the destructive undo tests to reject lost parents and claimed-user deletion."""
from pathlib import Path
import os
import subprocess

root = Path(__file__).resolve().parents[2]
source = root / 'crates/campfire/src/integrations/slack/undoer.rs'
original = source.read_text()
mutations = [
    ('kept_parent', 'self.keep_parents(tx, &batch)?;', '// deliberately omit kept-parent propagation',
     'slack_undo_keeps_live_thread_parent_for_foreign_or_scheduled_replies'),
    ('claimed_user', '.is_none_or(|s| s.trim().is_empty())', '.is_none_or(|_| true)',
     'slack_undo_keeps_claimed_placeholders_and_their_mapping'),
]
scratch = root.parent / '.scratch' / 'undo-mutations'
scratch.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, CARGO_BUILD_JOBS='2', CI='1',
           INTEGRATION_TEST_PORT_RANGE='53300-53399', CABLE_TEST_PORT_RANGE='53300-53399',
           TMPDIR=str(root.parent / '.scratch' / 'tmp'))
try:
    broken = original
    for name, old, new, test in mutations:
        assert broken.count(old) == 1, f'ambiguous or missing mutation: {name}'
        broken = broken.replace(old, new)
    source.write_text(broken)
    result = subprocess.run(
        ['mise', 'exec', 'rust@1.98.1', '--', 'cargo', 'test', '--locked',
         '--manifest-path', str(root / 'Cargo.toml'), '-p', 'campfire',
         'integrations::slack::undoer::tests', '--', '--test-threads=8'],
        cwd=root.parent, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    (scratch / 'broken.log').write_text(result.stdout)
    assert result.returncode != 0 and 'test result: FAILED.' in result.stdout, result.stdout[-4000:]
    for name, _, _, test in mutations:
        assert f'{test} ... FAILED' in result.stdout, f'{name} did not discriminate\n{result.stdout[-6000:]}'
        print(f'Undo mutation rejected: {name} -> {test} FAILED')
    for line in result.stdout.splitlines():
        if line.startswith('test result:'): print(line)
finally:
    source.write_text(original)
