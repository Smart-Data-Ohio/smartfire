#!/usr/bin/env python3
"""Record every WS11 contract afresh from the pin and compare committed vectors."""
from pathlib import Path
import os
import subprocess
import sys

root = Path(__file__).resolve().parents[3]
scratch = root / '.scratch'
scratch.mkdir(exist_ok=True)
env = dict(os.environ, PARITY_NAMESPACE='ws11', PARITY_OWNER='ws11',
           PARITY_IMAGE='triage-reference-d7c7de92')
contracts = [
    ('bot_contract', 'bot-contract-followup-final', False),
    ('posting_budget_contract', 'posting-budget-contract-final', True),
    ('webhook_contract', 'webhook-contract-final', True),
    ('delivery_contract', 'delivery-contract-final', False),
    ('event_payload_contract', 'event-payload-contract-final', False),
    ('bot_gaps_contract', 'bot-gaps-contract-final', False),
    ('grant_credential_contract', 'grant-credential-contract-final', False),
    ('event_access_contract', 'event-access-contract-final', False),
    ('event_polling_contract', 'event-polling-contract-final', False),
    ('approval_contract', 'approval-contract-final', False),
    ('approvals_service_contract', 'approvals-service-contract-final', True),
    ('agent_record_contract', 'agent-record-contract-final', False),
    ('slash_command_contract', 'slash-command-contract-final', False),
    ('step_contract', 'step-contract-final', False),
    ('working_presence_contract', 'working-presence-contract-final', False),
    ('context_contract', 'context-contract-final', False),
]
for name, output, freeze in contracts:
    command = ['rust/parity/bin/reference', 'runner', '--seed', 'default']
    if freeze:
        command += ['--time', '2026-03-02T16:00:00Z', '--freeze']
    command += [f'rust/reference-tools/agents/{name}.rb']
    with (scratch / f'{output}.json').open('wb') as stdout, (scratch / f'{output}.log').open('wb') as stderr:
        subprocess.run(command, cwd=root, env=env, stdout=stdout, stderr=stderr, check=True)
    if name not in ('bot_contract', 'posting_budget_contract', 'webhook_contract'):
        assert (scratch / f'{output}.json').read_bytes() == (root / f'rust/vectors/agents_{name}.json').read_bytes(), name
subprocess.run([sys.executable, 'rust/reference-tools/agents/compare-contracts.py'], cwd=root, check=True)
print('WS11 domain Rails oracles: 13 byte-identical contract files; 16 contracts recorded in total')
