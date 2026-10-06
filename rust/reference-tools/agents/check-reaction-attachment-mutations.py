#!/usr/bin/env python3
"""Reject a removed current reaction grant and a prematurely kept staged file."""
from pathlib import Path
import os
import re
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root.parent / '.scratch/reaction-attachment-mutations'
scratch.mkdir(parents=True, exist_ok=True)
mutations = [
    ('reaction_grant', 'crates/campfire/src/controllers/agents/reactions.rs',
     'if !agent_access::capability_for_agent(', 'if false && !agent_access::capability_for_agent(',
     'agent_reaction_permission_transitions'),
    ('attachment_file_rollback', 'crates/campfire/src/messaging/operations.rs',
     'keep_after_commit(tx, staged);', 'staged.keep();',
     'agent_attachment_queue_failure'),
]
for name, file, before, after, test in mutations:
    path = root / file
    original = path.read_bytes()
    source = original.decode()
    assert source.count(before) == 1, (name, source.count(before))
    try:
        path.write_text(source.replace(before, after, 1))
        result = subprocess.run(
            ['cargo', 'test', '--locked', '-j2',
             '-p', 'campfire', '--bin', 'campfire', test, '--', '--nocapture'],
            cwd=root, env=dict(os.environ, CI='1', CARGO_BUILD_JOBS='2', TMPDIR=str(scratch),
                              CABLE_TEST_PORT_RANGE='52900-52919',
                              INTEGRATION_TEST_PORT_RANGE='52920-52949', MAIL_TEST_PORT_RANGE='52920-52949'),
            capture_output=True, text=True)
        output = result.stdout + result.stderr
        (scratch / (name + '.log')).write_text(output)
        summary = re.search(r'^test result: FAILED\..*$', output, re.M)
        assert result.returncode != 0 and summary and 'assertion' in output and 'error[E' not in output, output[-4000:]
        print('WS11-api ' + name + ' mutation: ' + summary.group(), flush=True)
    finally:
        path.write_bytes(original)
print('WS11-api reaction/attachment mutations: 2 broken guards rejected; sources restored', flush=True)
