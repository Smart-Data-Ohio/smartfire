#!/usr/bin/env python3
"""Run native tests normally; run storage vectors with the pinned media libraries.

Use as CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER. This runs already compiled
test executables, so the host's rustc wrapper and build limits stay in effect.
The storage version guard and every byte assertion remain enabled.
"""
import os
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parents[3]
binary = Path(sys.argv[1]).resolve()
assert binary.is_relative_to(root), 'test executable must belong to this worktree'
dependency_file = binary.with_name(binary.name + '.d')
if dependency_file.exists() and 'crates/storage/tests/vectors.rs' in dependency_file.read_text():
    command = [
        'docker', 'run', '--rm', '--cpus', '2', '--name', f'ws8br-media-{os.getpid()}',
        '--user', f'{os.getuid()}:{os.getgid()}', '--env', 'CI=1', '--env', 'RUSTC_BOOTSTRAP=1',
        '--env', f'TMPDIR={root}/.scratch', '-v', f'{root}:{root}', '-w', str(root),
        '--entrypoint', str(binary), os.environ.get('PARITY_IMAGE', 'campfire-reference'),
        *sys.argv[2:],
    ]
    raise SystemExit(subprocess.run(command).returncode)
os.execv(str(binary), [str(binary), *sys.argv[2:]])
