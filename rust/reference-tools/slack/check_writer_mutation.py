#!/usr/bin/env python3
"""Prove the page writer test rejects ordinary per-message callbacks during imports."""
from pathlib import Path
import os
import subprocess

root = Path(__file__).resolve().parents[2]
source = root / 'crates/db/src/models/message.rs'
original = source.read_text()
old = 'let importing = imported_time.is_some();'
assert old in original
scratch = root.parent / '.scratch' / 'writer-mutation'
scratch.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, CARGO_BUILD_JOBS='2', CI='1', INTEGRATION_TEST_PORT_RANGE='53300-53399', CABLE_TEST_PORT_RANGE='53300-53399', TMPDIR=str(root.parent / '.scratch' / 'tmp'))
try:
    source.write_text(original.replace(old, 'let importing = false;'))
    result = subprocess.run(['cargo', 'test', '--locked', '--manifest-path', str(root / 'Cargo.toml'), '-p', 'campfire', 'slack_writer_history_replies_pins_reactions_are_quiet_and_keep_microseconds', '--', '--test-threads=8'], cwd=root.parent, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    (scratch / 'broken.log').write_text(result.stdout)
    assert result.returncode != 0 and 'test result: FAILED.' in result.stdout, result.stdout[-4000:]
    assert 'slack_writer_history_replies_pins_reactions_are_quiet_and_keep_microseconds ... FAILED' in result.stdout
    print('Writer mutation rejected: ordinary message callbacks -> slack_writer_history_replies_pins_reactions_are_quiet_and_keep_microseconds FAILED')
    for line in result.stdout.splitlines():
        if line.startswith('test result:'): print(line)
finally:
    source.write_text(original)
