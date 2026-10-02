#!/usr/bin/env python3
"""Compile and reject missing reader, fanout and concurrent budget guards."""
from pathlib import Path
import os
import re
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root.parent / '.scratch/legacy-security-mutations'
scratch.mkdir(parents=True, exist_ok=True)
mutations = [
    ('current_history_grant', 'crates/campfire/src/controllers/agents/reads.rs',
     'if !agent_access::capability_for_agent(p.conn,agent_id,"read_messages",Some(room.id))? {',
     'if false && !agent_access::capability_for_agent(p.conn,agent_id,"read_messages",Some(room.id))? {',
     'agent_permissions_mcp_after_success'),
    ('legacy_self_fanout', 'crates/db/src/models/bot_webhook_fanout.rs',
     'if bot.id != message.creator_id', 'if true', 'agent_legacy_bot_fanout_http'),
    ('legacy_hop_gate', 'crates/db/src/models/bot_webhook_fanout.rs',
     'if bots.is_empty() || hop_for_message(tx, message)? >= HOP_LIMIT {',
     'if bots.is_empty() {', 'agent_legacy_bot_fanout_http'),
    ('private_work_details', 'crates/db/src/models/agent_payloads.rs',
     'let visible = public || allowed;', 'let visible = public || allowed || true;',
     'agent_reads_work_bytes'),
    ('concurrent_budget', 'crates/db/src/models/agent_posting.rs',
     'if usage < limit {', 'if usage <= limit {', 'agent_concurrent_bot_budget'),
]
for name, file, before, after, test in mutations:
    path = root / file
    original = path.read_bytes()
    source = original.decode()
    assert source.count(before) == 1, (name, source.count(before))
    try:
        path.write_text(source.replace(before, after, 1))
        result = subprocess.run(
            ['mise', 'exec', 'rust@1.98.1', '--', 'cargo', 'test', '--locked', '-j2',
             '-p', 'campfire', '--bin', 'campfire', test, '--', '--nocapture'],
            cwd=root, env=dict(os.environ, CI='1', CARGO_BUILD_JOBS='2', TMPDIR=str(scratch),
                              CABLE_TEST_PORT_RANGE='52900-52919',
                              INTEGRATION_TEST_PORT_RANGE='52920-52949', MAIL_TEST_PORT_RANGE='52920-52949'),
            capture_output=True, text=True)
        output = result.stdout + result.stderr
        (scratch / (name + '.log')).write_text(output)
        summary = re.search(r'^test result: FAILED\..*$', output, re.M)
        assert result.returncode != 0 and summary and 'assertion' in output and 'error[E' not in output, output[-4000:]
        print('WS11-api ' + name + ' mutation: ' + summary.group(), flush=True)
    finally:
        path.write_bytes(original)
print('WS11-api legacy/security mutations: 5 broken guards rejected; sources restored', flush=True)
