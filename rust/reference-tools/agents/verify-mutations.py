#!/usr/bin/env python3
"""Compile regressions and require assertion failures in each security/contract test."""
from pathlib import Path
import os
import re
import subprocess

root = Path(__file__).resolve().parents[2]
worktree = root.parent
scratch = worktree / '.scratch' / 'ws11-mutations'
scratch.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, TMPDIR=str(worktree / '.scratch'), CARGO_TARGET_DIR=str(root / 'target'),
           CABLE_TEST_PORT_RANGE='52200-52249', MAIL_TEST_PORT_RANGE='52200-52249')
cmd = ['mise', 'exec', 'rust@1.98.1', '--', 'cargo', 'test', '--locked', '-j', '4', '-p', 'campfire',
       '--bin', 'campfire']
mutations = [
    ('reply-scope', 'crates/campfire/src/controllers/messages/by_bots.rs',
     '    deny_bot_reply_token(c)?;', '', 'ws11_reply_token_only_creates_messages'),
    ('system-notes', 'crates/campfire/src/controllers/messages/by_bots.rs',
     'if message.system_note {', 'if false {', 'ws11_bot_cannot_manage_system_notes'),
    ('root-count', 'crates/campfire/src/controllers/messages/by_bots.rs',
     'Message::count_roots_in_room(conn, room_id)?', 'Message::count_in_room(conn, room_id)?', 'ws11_bot_pagination_counts_only_root_messages'),
    ('sudo-rotation', 'crates/campfire/src/controllers/accounts/bots/keys.rs',
     '    concerns::require_sudo_mode(c)?;', '', 'ws11_key_rotation_requires_sudo_and_shows_the_key_once'),
    ('scrub-registration', 'crates/campfire/src/jobs/periodic.rs',
     '    periodic.task(clear_plaintext_bot_tokens_task());', '', 'the_periodic_loops_run_with_the_jobs'),
    ('credential-auth', 'crates/db/src/models/agent_access.rs',
     'if campfire_richtext::ruby::is_blank(secret) {', 'if true {', 'ws11_agent_credentials_are_authenticated_then_denied_on_human_endpoints'),
    ('capability-revocation', 'crates/db/src/models/agent_access.rs',
     'Ok(Some(exists(conn,', 'Ok(Some(true || exists(conn,', 'ws11_bot_grants_are_checked_on_every_request_after_membership'),
    ('credential-use-throttle', 'crates/db/src/models/agent_access.rs',
     'AND (last_used_at IS NULL OR last_used_at<=?)', 'AND (? IS NOT NULL)', 'ws11_agent_credentials_are_authenticated_then_denied_on_human_endpoints'),
]
for name, relative, before, after, test in mutations:
    path = root / relative
    original = path.read_text()
    if before not in original:
        raise RuntimeError(f'{name}: mutation anchor missing')
    try:
        path.write_text(original.replace(before, after))
        result = subprocess.run(cmd + [test, '--', '--nocapture'], cwd=root, env=env,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        (scratch / f'{name}.log').write_text(result.stdout)
        summary = re.search(r'^test result: FAILED\..*$', result.stdout, re.M)
        if result.returncode != 101 or summary is None or f'{test} ... FAILED' not in result.stdout or 'error[E' in result.stdout:
            raise RuntimeError(f'{name}: regression was not detected by its test; see {scratch/name}.log')
        print(f'{name}: {summary.group()}', flush=True)
    finally:
        path.write_text(original)
print(f'WS11 discrimination: {len(mutations)} compiled regressions detected; sources restored')
