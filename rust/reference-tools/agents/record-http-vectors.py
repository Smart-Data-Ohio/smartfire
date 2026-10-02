#!/usr/bin/env python3
"""Recapture all agent wire artifacts from private copies of the pinned Rails seed."""
from pathlib import Path
import os
import subprocess
import sys

root = Path(__file__).resolve().parents[3]
output = Path(sys.argv[1]).resolve()
output.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, PARITY_IMAGE='ws11api-reference:d7c7de92')
env.setdefault('PARITY_NAMESPACE', 'ws11api-wire')
for script, filename in [
    ('http_contract.rb', 'agent_http.json'),
    ('mcp_contract.rb', 'agent_mcp.json'),
    ('surface_contract.rb', 'agent_surface.json'),
    ('bot_http_contract.rb', 'agent_bot_http.json'),
    ('legacy_bot_http_contract.rb', 'agent_legacy_bot_http.json'),
    ('conversation_http_contract.rb', 'agent_conversation_http.json'),
    ('fizzy_http_contract.rb', 'agent_fizzy_http.json'),
    ('fizzy_action_http_contract.rb', 'agent_fizzy_action_http.json'),
    ('reads_http_contract.rb', 'agent_reads_http.json'),
    ('pins_http_contract.rb', 'agent_pins_http.json'),
    ('polls_http_contract.rb', 'agent_polls_http.json'),
    ('polling_http_contract.rb', 'agent_polling_http.json'),
    ('reactions_http_contract.rb', 'agent_reactions_http.json'),
    ('bot_reactions_http_contract.rb', 'agent_bot_reactions_http.json'),
    ('work_validation_http_contract.rb', 'agent_work_validation_http.json'),
    ('attachments_http_contract.rb', 'agent_attachments_http.json'),
    ('permissions_http_contract.rb', 'agent_permissions_http.json'),
    ('review192_contract.rb', 'agent_review192_http.json'),
    ('review192r2_attachment_diagnosis.rb', 'agent_review192r2_attachment.json'),
    ('review192r3_attachment_diagnosis.rb', 'agent_review192r3_attachment.json'),
]:
    with (output / filename).open('w') as stdout, (output / (script + '.log')).open('w') as stderr:
        result = subprocess.run([
            str(root / 'rust/parity/bin/reference'), 'exec', '--seed', 'default',
            'bin/rails', 'runner', '/work/reference-tools/agents/' + script,
        ], cwd=root, env=env, stdout=stdout, stderr=stderr)
    assert result.returncode == 0, (script, result.returncode)
    print('WS11-api captured ' + filename, flush=True)

subprocess.run([sys.executable, str(root / "rust/reference-tools/agents/record-r5-representations.py"), str(output)], cwd=root, env=env, check=True)
