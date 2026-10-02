#!/usr/bin/env python3
"""Prove the real HTTP oracle detects lost placeholder claims and forged Slack state.

Run in the pinned Rust/media image with fresh Rails seeds and the host rustc wrapper.
Only tracked source is temporarily mutated; finally always restores its exact bytes.
"""
from pathlib import Path
import os
import subprocess

ROOT = Path(__file__).resolve().parents[3]
LOGS = ROOT / '.scratch/google-claim-mutations'
LOGS.mkdir(parents=True, exist_ok=True)
COMMAND = ['cargo', 'test', '--offline', '--locked', '--manifest-path',
           str(ROOT / 'rust/Cargo.toml'), '-p', 'campfire',
           'google_callback_slack_opt_in_claim_matches_rails_over_real_http',
           '--', '--test-threads=2', '--nocapture']
ENV = dict(os.environ, CARGO_BUILD_JOBS='2', CI='1',
           CABLE_TEST_PORT_RANGE='53300-53399', INTEGRATION_TEST_PORT_RANGE='53300-53399')
MUTATIONS = [
    ('case_sensitive_claim', 'rust/crates/db/src/models/google_identity.rs',
     'SELECT * FROM users WHERE LOWER(email_address)=?',
     'SELECT * FROM users WHERE email_address=?', 'google-claim:', None),
    ('forged_slack_state', 'rust/crates/campfire/src/controllers/slack.rs',
     'if !oauth::valid_state(', 'if false && !oauth::valid_state(', 'slack-forged-state:', None),
    ('workspace_audit_label', 'rust/crates/campfire/src/controllers/slack/setup.rs',
     'label: Some(format!("SlackWorkspace #{}", workspace.id)),',
     'label: None,', 'setup_save', 'slack_connections_http_persistence_audits_and_requests_match_rails'),
]
for name, relative, before, after, assertion, test in MUTATIONS:
    path = ROOT / relative
    original = path.read_bytes()
    assert original.count(before.encode()) == 1, (name, 'mutation anchor drifted')
    try:
        path.write_bytes(original.replace(before.encode(), after.encode(), 1))
        command = COMMAND.copy()
        if test:
            command[command.index('google_callback_slack_opt_in_claim_matches_rails_over_real_http')] = test
        result = subprocess.run(command, cwd=ROOT / 'rust', env=ENV,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (LOGS / f'{name}.log').write_bytes(result.stdout)
        output = result.stdout.decode(errors='replace')
        assert result.returncode != 0 and assertion in output and 'test result: FAILED.' in output, name
        summary = next(line for line in output.splitlines() if line.startswith('test result:'))
        print(f'{name}: {summary}', flush=True)
    finally:
        path.write_bytes(original)
result = subprocess.run(COMMAND, cwd=ROOT / 'rust', env=ENV,
                        stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
(LOGS / 'restored.log').write_bytes(result.stdout)
assert result.returncode == 0, result.stdout.decode(errors='replace')
for line in result.stdout.decode().splitlines():
    if line.startswith(('Google →', 'test result:')):
        print(line)
print('Google → Slack claim mutations: 3 rejected; restored control passed; source restored')
