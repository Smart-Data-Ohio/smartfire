#!/usr/bin/env python3
"""Compile the three review regressions against the exact reviewed production source."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[3]
BASE = '6375ba3f5b996ad9bc27b5d9c0fd90ae2cc5bae2'
scratch = ROOT / '.scratch'
scratch.mkdir(exist_ok=True)
clone = Path(tempfile.mkdtemp(prefix='ws8bm-review-6375-', dir=scratch))
subprocess.run(['git', 'clone', '--quiet', '--shared', '--no-checkout', str(ROOT), str(clone)], check=True)
subprocess.run(['git', 'checkout', '--quiet', '--detach', BASE], cwd=clone, check=True)
for file in ['rust/crates/campfire/src/controllers/messages/review_tests.rs', 'rust/vectors/messaging/thread-review.json']:
    shutil.copyfile(ROOT / file, clone / file)
module = clone / 'rust/crates/campfire/src/controllers/messages.rs'
module.write_text(module.read_text().replace('mod upload_tests;', 'mod upload_tests;\n#[cfg(test)]\nmod review_tests;'))
assert subprocess.check_output(['git', 'diff', '--name-only'], cwd=clone, text=True).strip() == str(module.relative_to(clone))
(clone / '.scratch').mkdir()
env = dict(os.environ, CI='1', TMPDIR=str(clone / '.scratch'), CARGO_TARGET_DIR=str(ROOT / 'rust/target/review-6375'),
           CAMPFIRE_REFERENCE=str(clone), CABLE_TEST_PORT_RANGE='52000-52049', MAIL_TEST_PORT_RANGE='52000-52049',
           PARITY_IMAGE='triage-reference-d7c7de92', PARITY_NAMESPACE='ws8bm-review-base', PARITY_OWNER='ws8bm', PARITY_CPUS='2')
env.pop('RUST_TEST_THREADS', None)
print(f'WS8bm failing-first production revision: {BASE}; only regression module/vectors added; {clone}', flush=True)
commands = [('seeds', ['bash', 'rust/parity/bin/seed', 'build', 'default', 'first_run']),
            ('tests', ['mise', 'exec', 'rust@1.98.1', '--', 'cargo', 'test', '--locked', '-j4', '--manifest-path', 'rust/Cargo.toml',
                       '-p', 'campfire', '--bin', 'campfire', 'controllers::messages::review_tests::review_', '--', '--nocapture'])]
for name, command in commands:
    with (clone / '.scratch' / f'{name}.log').open('w') as log:
        result = subprocess.run(command, cwd=clone, env=env, stdout=log, stderr=subprocess.STDOUT)
    output = (clone / '.scratch' / f'{name}.log').read_text()
    for line in output.splitlines():
        if line.startswith(('seed:', 'test result:', '    Finished', 'WS8bm')) or (line.startswith('test ') and line.endswith('FAILED')):
            print(line, flush=True)
    if name == 'seeds':
        assert result.returncode == 0, output[-4000:]
    else:
        assert result.returncode == 101 and 'test result: FAILED. 0 passed; 3 failed;' in output, output[-4000:]
print('WS8bm failing-first: all 3 assigned regressions compiled and failed against 6375ba3f', flush=True)
