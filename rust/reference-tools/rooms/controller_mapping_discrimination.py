#!/usr/bin/env python3
"""Reject individually mapped HTTP cases against compiled missing controller dispatches."""
import argparse
import os
from pathlib import Path
import re
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument('group', choices=['sidebars', 'involvements', 'reads', 'favorites', 'room_categories', 'categories', 'switchers'])
args = parser.parse_args()
root = Path(__file__).resolve().parents[3]
source = root / 'rust/crates/campfire/src/controllers.rs'
scratch = root / '.scratch'
scratch.mkdir(exist_ok=True)
controllers = {'sidebars': 'users/sidebars', 'involvements': 'rooms/involvements', 'reads': 'rooms/reads',
               'favorites': 'rooms/favorites', 'room_categories': 'room_categories', 'categories': 'rooms/categories', 'switchers': 'switchers'}
original = source.read_bytes()
env = dict(os.environ, CI='1', TMPDIR=str(scratch), CARGO_TARGET_DIR=str(root/'rust/target'),
           CABLE_TEST_PORT_RANGE='52100-52149', MAIL_TEST_PORT_RANGE='52100-52149')
try:
    changed = re.sub(r'^\s*"' + controllers[args.group] + r'#[^"]+" => arc\([^\n]+\),\n', '', original.decode(), flags=re.M)
    assert changed != original.decode(), 'missing dispatch mutation'
    source.write_text(changed)
    run = subprocess.run(['mise', 'exec', 'rust@1.98.1', '--', 'cargo', 'test', '--locked', '-j4', '--manifest-path', 'rust/Cargo.toml',
                          '-p', 'campfire', '--bin', 'campfire', f'controllers::rooms::{args.group}_rails_cases', '--', '--test-threads=4'],
                         cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    (scratch/f'{args.group}-case-discrimination.log').write_text(run.stdout)
    summaries = [line for line in run.stdout.splitlines() if line.startswith('test result:')]
    assert run.returncode == 101 and len(summaries) == 1 and '0 passed;' in summaries[0] and '0 failed;' not in summaries[0], run.stdout
    print(summaries[0], flush=True)
finally:
    source.write_bytes(original)
print(f'Controller mapping discrimination: {args.group} compiled dispatch removal rejected; source restored')
