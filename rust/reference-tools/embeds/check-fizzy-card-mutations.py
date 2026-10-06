#!/usr/bin/env python3
"""Compile discriminating card-policy defects, and restore each source even on failure."""
from pathlib import Path
import os
import subprocess

root = Path(__file__).resolve().parents[2]
env = os.environ.copy()
env.setdefault('TMPDIR', str(root.parent / '.scratch'))
env.setdefault('CARGO_TARGET_DIR', str(root / '.scratch/target'))
env['CI'] = '1'
env['CABLE_TEST_PORT_RANGE'] = '51550-51599'
mutations = [
    ('crates/campfire/src/controllers/fizzy_cards.rs', 'message.room_id != room.id || !referenced', 'message.room_id != room.id', 'ws15e_fizzy_frames_authorize_room_message_reference_before_cache_or_enqueue'),
    ('crates/campfire/src/controllers/fizzy_cards.rs', 'message.room_id != room.id || !referenced', '!referenced', 'ws15e_fizzy_frames_authorize_room_message_reference_before_cache_or_enqueue'),
    ('crates/campfire/src/integrations/fizzy/cards.rs', 'fizzy_card_id=?1 AND user_id=?2', 'fizzy_card_id=?1 AND ?2 IS NOT NULL', 'ws15e_fizzy_frames_use_viewer_cache_and_claim_stale_refresh_once'),
    ('crates/campfire/src/integrations/fizzy/cards.rs', 'AND (fetch_requested_at IS NULL OR fetch_requested_at<?3)', 'AND ?3 IS NOT NULL', 'ws15e_fizzy_cache_boundary_claims_and_validation'),
    ('crates/campfire/src/integrations/fizzy/cards.rs', 'tx.emit_after_commit(Event::job(&super::fetch::FetchJob {', 'let _ = Event::job(&super::fetch::FetchJob {', 'ws15e_fizzy_create_and_frame_claim_roll_back_when_durable_insert_is_rejected'),
    ('crates/campfire/src/controllers/presenters/fizzy_cards.rs', 'let html = container(conn, &message)?;', 'let html = format!("{}{}", container(conn, &message)?, conn.query_row("SELECT payload FROM fizzy_card_caches WHERE fizzy_card_id=? LIMIT 1", [card_id], |r| r.get::<_, String>(0))?);', 'ws15e_fizzy_broadcasts_committed_content_free_frames_to_room_and_thread'),
    ('crates/campfire/src/integrations/fizzy/cards.rs', 'fetch_requested_at<?3)', 'fetch_requested_at<=?3)', 'ws15e_fizzy_cache_boundary_claims_and_validation'),
    ('crates/views/src/fizzy_cards.rs', 'uri.scheme.as_deref() == Some("https")', 'true', 'ws15e_fizzy_frames_use_viewer_cache_and_claim_stale_refresh_once'),
]
for index, (filename, old, replacement, test) in enumerate(mutations, 1):
    path = root / filename
    source = path.read_text()
    assert source.count(old) == 1, (filename, old)
    # The event-discard mutant changes a nested function call into a value assignment.
    mutated = source.replace(old, replacement)
    if 'let _ = Event::job' in replacement:
        mutated = mutated.replace('user_id: self.user_id,\n            }));', 'user_id: self.user_id,\n            });')
    try:
        path.write_text(mutated)
        result = subprocess.run(['cargo', 'test', '-j', '4', '-p', 'campfire', test, '--', '--nocapture'], cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        (root.parent / '.scratch' / f'fizzy-card-mutation-{index}.log').write_text(result.stdout)
        assert result.returncode != 0 and 'test result: FAILED' in result.stdout, f'Undetected or uncompiled mutation: {test}\n{result.stdout[-2500:]}'
        print(f'{index} {filename} {test}: ' + next(line for line in result.stdout.splitlines() if line.startswith('test result:')), flush=True)
    finally:
        path.write_text(source)
print(f'WS15e Fizzy card mutation checks: {len(mutations)} detected, 0 survived')
