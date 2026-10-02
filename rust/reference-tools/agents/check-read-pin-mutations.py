#!/usr/bin/env python3
"""Compile broken room visibility and pin grant guards; reject and restore each."""
from pathlib import Path
import os
import re
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root.parent / '.scratch/read-pin-mutations'
scratch.mkdir(parents=True, exist_ok=True)
mutations = [
    ('room_visibility', 'crates/campfire/src/controllers/agents/reads.rs',
     'if !agent.legacy_capabilities(conn)? {',
     'if false && !agent.legacy_capabilities(conn)? {', 'agent_reads_rooms_bytes'),
    ('pin_grant', 'crates/campfire/src/controllers/agents/pending.rs',
     'if !allowed(tx, &agent, cap, Some(message.room_id))? {',
     'if !matches!(op, "pin_message" | "unpin_message") && !allowed(tx, &agent, cap, Some(message.room_id))? {',
     'agent_pins_rest_bytes'),
]
for name, file, before, after, test in mutations:
    path = root / file
    original = path.read_bytes()
    source = original.decode()
    assert source.count(before) == 1, (name, source.count(before))
    try:
        path.write_text(source.replace(before, after, 1))
        result = subprocess.run(
            ['mise', 'exec', 'rust@1.98.1', '--', 'cargo', 'test', '--locked', '-j2',
             '-p', 'campfire', '--bin', 'campfire', test, '--', '--nocapture'],
            cwd=root, env=dict(os.environ, CI='1', CARGO_BUILD_JOBS='2', TMPDIR=str(scratch),
                              INTEGRATION_TEST_PORT_RANGE='52920-52949'),
            capture_output=True, text=True)
        output = result.stdout + result.stderr
        (scratch / (name + '.log')).write_text(output)
        summary = re.search(r'^test result: FAILED\..*$', output, re.M)
        assert result.returncode != 0 and summary and 'assertion' in output and 'error[E' not in output, output[-4000:]
        print('WS11-api ' + name + ' mutation: ' + summary.group(), flush=True)
    finally:
        path.write_bytes(original)
print('WS11-api read/pin mutations: 2 broken guards rejected; sources restored', flush=True)
