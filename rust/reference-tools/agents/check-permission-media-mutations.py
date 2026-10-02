#!/usr/bin/env python3
"""Prove warmed reads, reply-room scope and synchronous media analysis can fail."""
from pathlib import Path
import os
import re
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root.parent / '.scratch/permission-media-mutations'
scratch.mkdir(parents=True, exist_ok=True)
mutations = [
    ('current_read_grant', 'crates/campfire/src/controllers/agents/reads.rs',
     'if !agent_access::capability_for_agent(p.conn, agent_id, "read_messages", Some(room.id))? {',
     'if false && !agent_access::capability_for_agent(p.conn, agent_id, "read_messages", Some(room.id))? {',
     'agent_permissions_mcp_after_success'),
    ('reply_room_scope', 'crates/rails_compat/src/verifiers.rs',
     '(metadata::ruby_to_s(data.get("room_id")) == room_id).then',
     '(!metadata::ruby_to_s(data.get("room_id")).is_empty()).then',
     'bot_http_reply_invalid_scope'),
    ('media_analysis', 'crates/campfire/src/controllers/messages.rs',
     'let blob = analyze_attachment(app, blob).await?;', 'let blob = blob;',
     'agent_attachments_rest_bytes'),
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
print('WS11-api permission/media mutations: 3 broken guards rejected; sources restored', flush=True)
