#!/usr/bin/env python3
"""Prove lease, token and sweep tests reject broken implementations; restore every edit."""
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[2]
source = root / 'crates/db/src/models/slack_import.rs'
credential_source = root / 'crates/db/src/models/slack.rs'
original = source.read_text()
credential_original = credential_source.read_text()
env = dict(os.environ, CARGO_BUILD_JOBS='2')
mutations = [
    ('cancelled lease ignored', "status IN ('cancelled','failed')", "status IN ('never')", 'slack_import_cancelled_and_failed_leases_block_until_release_or_staleness'),
    ('fresh lease stolen', "(json_extract(state, '$.step_started_at') IS NULL OR json_extract(state, '$.step_started_at') <= ?)", '? IS NOT NULL', 'slack_import_two_workers_execute_a_step_once'),
    ('wrong token accepted', "json_extract(state, '$.step_lease_token') = ?", '? IS NOT NULL', 'slack_import_stale_takeover_preserves_progress_and_fences_old_holder'),
    ('sweep duplicates queued job', 'if oldest.step_job_pending(now)', 'if false && oldest.step_job_pending(now)', 'slack_import_overlapping_sweeps_only_kick_queued_run_once'),
]
try:
    for label, before, after, test in mutations:
        if before not in original:
            raise RuntimeError(f'mutation anchor missing: {label}')
        source.write_text(original.replace(before, after))
        result = subprocess.run(['mise', 'exec', 'rust@1.98.1', '--', 'cargo', 'test', '--manifest-path', str(root / 'Cargo.toml'), '-p', 'campfire_db', '--lib', test, '--', '--test-threads=8'], env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        summaries = [line for line in result.stdout.splitlines() if line.startswith('test result:')]
        if result.returncode == 0 or not summaries or '1 failed' not in summaries[-1]:
            print(result.stdout)
            raise RuntimeError(f'mutation did not produce an assertion failure: {label}')
        print(f'{label}: {summaries[-1]}', flush=True)
finally:
    source.write_text(original)
try:
    before = 'let ciphertext = encryption.encrypt(secret);'
    if before not in credential_original:
        raise RuntimeError('credential mutation anchor missing')
    credential_source.write_text(credential_original.replace(before, 'let ciphertext = secret.to_owned();'))
    result = subprocess.run(['mise', 'exec', 'rust@1.98.1', '--', 'cargo', 'test', '--manifest-path', str(root / 'Cargo.toml'), '-p', 'campfire_db', '--lib', 'slack_credentials_create_encrypts_rows_and_exports_for_rails_readback', '--', '--test-threads=8'], env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    summaries = [line for line in result.stdout.splitlines() if line.startswith('test result:')]
    if result.returncode == 0 or not summaries or '1 failed' not in summaries[-1]:
        print(result.stdout)
        raise RuntimeError('plaintext credential mutation did not produce an assertion failure')
    print('plaintext credential write: ' + summaries[-1], flush=True)
finally:
    credential_source.write_text(credential_original)
print('Slack mutation guards: 5 broken implementations rejected; source restored')
