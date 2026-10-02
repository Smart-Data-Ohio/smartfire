#!/usr/bin/env python3
"""Discriminate the durable Slack protocol against three deliberately broken guards."""
from pathlib import Path
import os
import subprocess

root = Path(__file__).resolve().parents[2]
source = root / 'crates/campfire/src/integrations/slack/jobs.rs'
original = source.read_text()
mutations = [
    ('lease_release', 'SlackImport::release_step_lease(tx, id, &lease)', 'SlackImport::refresh_step_lease(tx, id, &lease)', 'slack_job_releases_before_durable_continuation_and_uses_serial_queue'),
    ('retry_after', '.wait(Duration::from_secs(delay))', '.wait(Duration::ZERO)', 'slack_job_retry_after_commits_heartbeat_and_delayed_job_atomically'),
    ('cancel_preservation', 'if run.status == "running" {', 'if true {', 'slack_job_cancel_during_auth_error_preserves_cancel_and_disconnects'),
]
env = dict(os.environ, CARGO_BUILD_JOBS='2', CI='1', INTEGRATION_TEST_PORT_RANGE='53300-53399', CABLE_TEST_PORT_RANGE='53300-53399')
scratch = root.parent / '.scratch' / 'runner-mutations'
scratch.mkdir(parents=True, exist_ok=True)
env['TMPDIR'] = str(root.parent / '.scratch' / 'tmp')
try:
    # One build deliberately breaks all three independent contracts. Each must fail at its
    # own runtime assertion; a compiler error or an unrelated test failure is not proof.
    broken = original
    for name, old, new, test in mutations:
        assert old in broken, f'mutation target missing: {name}'
        broken = broken.replace(old, new)
    source.write_text(broken)
    command = ['mise', 'exec', 'rust@1.98.1', '--', 'cargo', 'test', '--locked', '--manifest-path', str(root / 'Cargo.toml'), '-p', 'campfire', 'integrations::slack::jobs::tests', '--', '--test-threads=8']
    result = subprocess.run(command, env=env, cwd=root.parent, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    (scratch / 'broken.log').write_text(result.stdout)
    assert result.returncode != 0 and 'test result: FAILED.' in result.stdout, result.stdout[-3000:]
    for name, _, _, test in mutations:
        assert f'{test} ... FAILED' in result.stdout, f'{name} did not discriminate\n{result.stdout[-6000:]}'
        print(f'Runner mutation rejected: {name} -> {test} FAILED')
    for line in result.stdout.splitlines():
        if line.startswith('test result:'): print(line)
finally:
    source.write_text(original)
