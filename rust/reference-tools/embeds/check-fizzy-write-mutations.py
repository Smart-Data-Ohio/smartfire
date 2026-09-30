#!/usr/bin/env python3
"""Compile write-flow guard defects; each child runs the real HTTP adapter and local server."""
from pathlib import Path
import os
import subprocess

root = Path(__file__).resolve().parents[2]
env = os.environ.copy()
env.update(CI='1', TMPDIR=str(root.parent / '.scratch'), CARGO_TARGET_DIR=str(root / '.scratch/target'), FIZZY_API_BASE_URL='http://127.0.0.1:51597')
mutants = [
    ('fizzy_connections.rs', 'concerns::require_sudo_mode(c)?;', '', 'ws15e_fizzy_connection_http_matrix', 'WS15E_FIZZY_CONNECTION_CASE', 'sudo'),
    ('fizzy_message_cards.rs', '.is_some_and(|t| t.locked_at.is_some())', '.is_some_and(|_| false)', 'ws15e_fizzy_message_creation_http_matrix', 'WS15E_FIZZY_MESSAGE_CASE', 'locked'),
]
for index, (filename, old, new, test, key, case) in enumerate(mutants, 1):
    path = root / 'crates/campfire/src/controllers' / filename
    original = path.read_text()
    assert old in original
    # The message matrix hands each isolated child an already-bound ephemeral listener.
    # Run that parent harness, including its locked case, rather than bypassing socket handoff.
    if key.endswith('CONNECTION_CASE'):
        env[key] = case
    try:
        path.write_text(original.replace(old, new, 1))
        run = subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','-j','2','-p','campfire',test,'--','--nocapture','--test-threads=8'],cwd=root,env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
        (root.parent / '.scratch' / f'fizzy-write-mutation-{index}.log').write_text(run.stdout)
        assert run.returncode and 'test result: FAILED' in run.stdout, run.stdout[-3000:]
        print(f'{index} {filename} {case}: ' + next(line for line in run.stdout.splitlines() if line.startswith('test result:')),flush=True)
    finally:
        path.write_text(original)
        env.pop(key, None)
print(f'WS15e Fizzy write mutation checks: {len(mutants)} detected, 0 survived')
