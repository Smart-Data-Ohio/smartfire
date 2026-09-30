#!/usr/bin/env python3
"""Compare fresh pinned Rails outputs, allowing only the randomized encrypted secret."""
import json
from pathlib import Path

root = Path(__file__).resolve().parents[3]
for fresh, vector in [('bot-contract-followup-final', 'agents_bot_contract'),
                      ('posting-budget-contract-final', 'agents_posting_budget_contract')]:
    assert (root / '.scratch' / f'{fresh}.json').read_bytes() == (root / 'rust/vectors' / f'{vector}.json').read_bytes()
fresh = json.loads((root / '.scratch/webhook-contract-final.json').read_text())
vector = json.loads((root / 'rust/vectors/agents_webhook_contract.json').read_text())
assert {k: v for k, v in fresh.items() if k != 'secret'} == {k: v for k, v in vector.items() if k != 'secret'}
assert fresh['secret']['encoding'] == 'US-ASCII' and fresh['secret']['repeated']
assert len(fresh['secret']['plaintext']) == 64
print('WS11 bot/posting Rails oracles: 2 byte-identical contract files')
print(f"WS11 webhook Rails oracle: {len(fresh['guards'])} numeric hosts; {len(fresh['dns'])} DNS cases; {len(fresh['signatures'])} signatures; {len(fresh['payloads'])} payloads matched; randomized AR secret regenerated")
