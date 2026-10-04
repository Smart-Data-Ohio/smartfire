#!/usr/bin/env python3
"""Require the three full original sequences to reject actual writer defects."""
import argparse
from pathlib import Path
import subprocess

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary', type=Path, required=True)
p.add_argument('--test-host', type=Path, required=True)
p.add_argument('--scenario', choices=['all', 'inbox', 'inbox-filter', 'work'], default='all')
args = p.parse_args()
root = Path(__file__).resolve().parents[4]
runner = Path(__file__).with_name('system_behavior.py')
cases = [
    ('inbox', '--inject-inbox-handle', 'activity_inbox_test.rb: handles an item', '#activity-unread-count[hidden]'),
    ('inbox-filter', '--inject-inbox-preference', 'activity_inbox_test.rb: filters by type', 'event reminders'),
    ('work', '--inject-work-status', 'agent_work_assignment_test.rb:', 'committed work status remained planned'),
]
if args.scenario != 'all':
    cases = [case for case in cases if case[0] == args.scenario]
for scenario, injection, name, assertion in cases:
    for mutated in [False, True]:
        command = ['python3', str(runner), '--binary', str(args.binary.resolve()),
                   '--test-host', str(args.test_host.resolve()), '--scenario', scenario]
        if mutated:
            command.append(injection)
        result = subprocess.run(command, cwd=root, capture_output=True, text=True)
        output = result.stdout + result.stderr
        if 'Rust system behavior:' not in output:
            raise RuntimeError(f'Invalid {scenario} control: setup/transport failed\n{output}')
        rails, rust = output.split('Rust system behavior:', 1)
        failures = [line for line in output.splitlines() if line.startswith('FAIL ')]
        if 'Agent system behavior: 1 passed; 0 failed; 0 deferred' not in rails:
            raise RuntimeError(f'Invalid {scenario} control: Rails did not pass\n{output}')
        if not mutated:
            if result.returncode or failures or 'Agent system behavior: 1 passed; 0 failed; 0 deferred' not in rust:
                raise RuntimeError(f'Invalid {scenario} control: baseline did not pass\n{output}')
            print(f'Cutover {scenario}: Rails 1 passed; Rust 1 passed; 0 failures', flush=True)
        else:
            if (result.returncode != 1 or len(failures) != 1 or name not in failures[0]
                    or assertion not in failures[0]
                    or 'Agent system behavior: 0 passed; 1 failed; 0 deferred' not in rust):
                raise RuntimeError(f'Invalid {scenario} control: intended writer assertion did not fail\n{output}')
            print(f'Cutover {scenario}: intended writer defect rejected; Rails 1 passed; Rust 1 deliberate failure; waits unchanged', flush=True)
print(f'Cutover browser discrimination: {len(cases)} paired sequences passed; {len(cases)} writer defects rejected; 0 invalid controls')
