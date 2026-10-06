#!/usr/bin/env python3
"""Prove that the malformed-payload, transport and atomic undo regressions reject faults."""
from pathlib import Path
import os
import subprocess

root = Path(__file__).resolve().parents[2]
cargo = os.environ.get('WS16_CARGO', 'cargo').split()
env = dict(os.environ, CARGO_BUILD_JOBS='2', CABLE_TEST_PORT_RANGE='53300-53399',
           INTEGRATION_TEST_PORT_RANGE='53300-53399')
groups = [
    ('campfire', 'integrations::slack', [
        ('crates/campfire/src/integrations/slack/payload.rs',
         '_ => Err(no_method("[]", value)),', '_ => Ok(Value::Null),',
         'slack_malformed_payloads_match_actual_rails_classes_messages_and_results'),
        ('crates/campfire/src/integrations/slack/client.rs',
         'format!("Slack network error for {method}: {class}: {message}")',
         'format!("Slack network error for {method}: {message}")',
         'slack_client_transport_classes_messages_and_retryability_match_rails'),
    ]),
    ('campfire_db', 'slack_import_atomic_undo_claim', [
        ('crates/db/src/models/slack_import.rs',
         "status = 'queued' OR {BLOCKING}", "status = 'queued' AND {BLOCKING}",
         'slack_import_atomic_undo_claim_refuses_queue_arriving_after_precheck'),
    ]),
]
for package, filter_name, faults in groups:
    originals = {}
    try:
        for relative, before, after, _ in faults:
            path = root / relative
            source = path.read_text()
            originals[path] = source
            assert source.count(before) == 1, f'nonunique mutation anchor: {relative}'
            path.write_text(source.replace(before, after))
        result = subprocess.run(cargo + ['test', '--offline', '--locked', '--manifest-path',
            str(root / 'Cargo.toml'), '-p', package, filter_name, '--', '--test-threads=8'],
            env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        assert result.returncode != 0 and 'test result: FAILED.' in result.stdout, result.stdout[-4000:]
        for _, _, _, test in faults:
            failure = next((line for line in result.stdout.splitlines()
                if line.startswith('test ') and test + ' ... FAILED' in line), None)
            assert failure, f'mutation did not reach its assertion: {test}\n{result.stdout[-8000:]}'
            print(failure, flush=True)
        for line in result.stdout.splitlines():
            if line.startswith('test result:'): print(line, flush=True)
    finally:
        for path, source in originals.items(): path.write_text(source)
print('Slack lifecycle mutation guards: malformed root, erased transport class and stale undo precheck rejected; sources restored')
