#!/usr/bin/env python3
"""Reproduce the #244 HTTP branch controls, restoring every producer on exit.

Use --phase before only from the reviewed head with this tool/manifest supplied
externally. By default execute the new assertions. --exec-wrapper may select the
normal correctness Docker runner; it receives a command argument vector.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--phase', choices=('before', 'after'), default='after')
parser.add_argument('--exec-wrapper', type=Path)
parser.add_argument('--output', type=Path, default=Path('rust/target/review244-branch-controls'))
args = parser.parse_args()
root = Path(__file__).resolve().parents[3]
manifest = json.loads(Path(__file__).with_name('review244_branch_mutations.json').read_text())
args.output.mkdir(parents=True, exist_ok=True)
originals = {}
try:
    for path, anchor, replacement in manifest['patches']:
        producer = root / path
        source = producer.read_text()
        assert source.count(anchor) == 1, (path, 'invalid producer anchor')
        originals.setdefault(producer, producer.read_bytes())
        producer.write_text(source.replace(anchor, replacement))
    for case in manifest['controls']:
        expression = ' or '.join(f'test(={test})' for test in case[args.phase + '_tests'])
        command = ['env', 'WS244_R3_MUTATION=' + case['name'], 'cargo', 'nextest', 'run',
                   '--manifest-path', 'rust/Cargo.toml', '--locked', '-p', 'campfire',
                   '-j', '4', '--no-fail-fast', '-E', expression]
        if args.exec_wrapper:
            command = ['bash', str(args.exec_wrapper)] + command
        env = dict(os.environ, CI='true', RUST_TEST_THREADS='4')
        run = subprocess.run(command, cwd=root, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (args.output / (case['name'] + '.log')).write_text(run.stdout)
        expected = case[args.phase + '_exit']
        assert run.returncode == expected and 'Summary' in run.stdout, (case['name'], 'invalid or surviving control', run.stdout[-4000:])
        if expected == 100:
            assert 'panicked at' in run.stdout and re.search(r'^\s*FAIL\s+\[', run.stdout, re.M), (case['name'], 'no assertion failure')
            if args.phase == 'after':
                assert case['assertion_failure'] in run.stdout, (case['name'], 'wrong failure')
        print(case['name'] + ': ' + re.search(r'Summary.*', run.stdout)[0], flush=True)
finally:
    for path, source in originals.items():
        path.write_bytes(source)
