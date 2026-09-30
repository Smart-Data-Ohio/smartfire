#!/usr/bin/env python3
"""List each unmapped pinned domain case with its owner; this never claims execution."""
from pathlib import Path
import json
import subprocess
ROOT = Path(__file__).resolve().parents[3]
subprocess.run(['python3', 'rust/reference-tools/agents/domain-case-inventory.py'], cwd=ROOT, check=True)
inventory = json.loads((ROOT / '.scratch/ws11-domain-case-inventory.json').read_text())
ports = json.loads((Path(__file__).parent / 'case-ports.json').read_text())
completed = {group['rails_file']: {case['rails'] for case in group['cases']} for group in ports['files']}
shared = {
    'test/jobs/agent/event_webhook_job_test.rb': 'WS11 delivery; WS15g live repository reader',
    'test/models/agent_approval_test.rb': 'WS11 domain; WS11-ui human approval controller/pages',
    'test/models/agent_kill_switch_test.rb': 'WS11; WS15g/WS15e approved-action execution callbacks',
    'test/models/agents/work_payload_test.rb': 'WS11; WS8 message presenter; WS15g private repository reader',
    'test/models/channel_thread_agent_assignment_test.rb': 'WS11 agent callbacks; WS12 mutation producers',
    'test/models/message_streaming_test.rb': 'WS11 finalization; WS12 activity; WS14/15 external reference sync',
    'test/models/message/bot_webhook_fanout_test.rb': 'WS11; WS16 import suppression integration',
    'test/models/user/bot_test.rb': 'WS11 bot domain/removal; WS11-api by-bot HTTP surface',
    'test/models/webhook_agent_key_test.rb': 'WS11; WS8 presenter; WS15g live private repository access',
    'test/services/slash_commands/dispatcher_test.rb': 'WS11 agent dispatch; WS8 built-in commands',
}
remaining = []
for group in inventory:
    done = completed.get(group['path'], set())
    assert done.issubset(set(group['names'])), group['path']
    names = [name for name in group['names'] if name not in done]
    if names:
        remaining.append({'rails_file': group['path'], 'owner': shared.get(group['path'], 'WS11 domain'), 'remaining_named_case_ports': names})
output = {'reference_pin': ports['reference_pin'], 'status': 'partial', 'meaning': 'Named case ports only. Consolidated Rust checks and Rails contract vectors are additional evidence, not file closure.', 'boundary_deferred': ports['boundary_deferred'], 'files': remaining}
(Path(__file__).parent / 'deferred-domain-cases.json').write_text(json.dumps(output, indent=2) + '\n')
print(f"WS11 deferred case inventory: {len(remaining)} pinned files; {sum(len(g['remaining_named_case_ports']) for g in remaining)} named source cases; owners recorded per file")
