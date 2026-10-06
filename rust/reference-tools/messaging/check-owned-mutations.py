#!/usr/bin/env python3
"""Prove owned controller/cache regressions reject broken production paths."""
from pathlib import Path
import os
import sys
import subprocess
ROOT = Path(__file__).resolve().parents[3]
SCRATCH = ROOT / '.scratch/owned-mutations'
SCRATCH.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, CI='1', CARGO_BUILD_JOBS='2', RUST_TEST_THREADS='8',
           CABLE_TEST_PORT_RANGE=os.environ.get('CABLE_TEST_PORT_RANGE', '52000-52049'),
           MAIL_TEST_PORT_RANGE=os.environ.get('MAIL_TEST_PORT_RANGE', '52000-52049'))
mutations = [
    ('agent-root-delivery-omitted', 'rust/crates/db/src/models/agent_delivery.rs',
     'tx.emit_after_commit(Event::job(&DeliveryJob { event_id: e.id }));',
     'let _ = e.id;',
     'rich_text_and_markdown_root_mentions'),
    ('revoked-agent-delivery-allowed', 'rust/crates/db/src/models/agent_delivery.rs',
     'let suppressed = if revoked {',
     'let suppressed = if false && revoked {',
     'revoked_root_mention'),
    ('thread-unread-uses-notification-preference', 'rust/crates/db/src/models/channel_thread.rs',
     'if membership.user_id == message.creator_id',
     'if membership.involvement == crate::ThreadInvolvement::Nothing || membership.user_id == message.creator_id',
     'thread_post_marks_all_joined_users'),
    ('forward-picker-n-plus-one', 'rust/crates/campfire/src/controllers/message_forwards.rs',
     'let rows = rooms.iter().filter(|room| !room.board()).map(|room| {',
     'let rows = rooms.iter().filter(|room| !room.board()).map(|room| { let _ = Room::find(conn, room.id)?;',
     'forward_picker_excludes_boards'),
    ('identical-save-edited', 'rust/crates/db/src/models/message.rs',
     'let edited_at = if content_changes { Some(now) } else { self.edited_at };',
     'let edited_at = if stamp_edited { Some(now) } else { self.edited_at };',
     'root_edit_markers_match_rails'),
    ('legacy-v2-cache-reused', 'rust/crates/views/src/fragment_cache/keys.rs',
     'pub const PRESENTATION_CACHE_VERSION: i64 = 3;',
     'pub const PRESENTATION_CACHE_VERSION: i64 = 2;',
     'legacy_v2_fragment_and_page_validators'),
    ('cached-token-leak', 'rust/crates/campfire/src/controllers/presenters.rs',
     '.map(std::sync::Arc::new)',
     '.map(|mut html| { html.push_str(r#"<input type="hidden" name="authenticity_token" value="foreign-session" />"#); std::sync::Arc::new(html) })',
     'cached_pages_refreshes_and_thread_pages'),
    ('csrf-check-bypassed', 'rust/crates/kit/src/ctx.rs',
     'if valid_origin && self.any_authenticity_token_valid() {',
     'if valid_origin && { let _ = self.any_authenticity_token_valid(); true } {',
     'cached_owned_forms_submit'),
    ('github-card-omitted', 'rust/crates/campfire/src/controllers/presenters/search_preloads.rs',
     'let html = crate::controllers::presenters::github::message_cards_in_zone(p.conn, p.app(), message, &p.render_zone)?;',
     r'let html = format!("<div id=\"github_pr_cards_message_{}\" class=\"github-pr-cards\"></div>\n", message.client_message_id);',
     'complete_github_containers'),
    ('drive-author-bypassed', 'rust/crates/campfire/src/controllers/messages.rs',
     'if message.system_note || require_current_user(c)?.id != message.creator_id {',
     'if message.system_note || { let _ = require_current_user(c)?; false } {',
     'root_and_thread_drive_requests'),
]
if sys.argv[1:]:
    requested = set(sys.argv[1:])
    assert requested <= {row[0] for row in mutations}, requested
    mutations = [row for row in mutations if row[0] in requested]
for name, file, before, after, test in mutations:
    path = ROOT / file
    source = path.read_text()
    assert source.count(before) == 1, name
    try:
        path.write_text(source.replace(before, after))
        run = subprocess.run(['cargo', 'test', '--locked', '-j2',
                              '--manifest-path', 'rust/Cargo.toml', '-p', 'campfire', test, '--', '--nocapture'],
                             cwd=ROOT, env=env, capture_output=True, text=True)
        output = run.stdout + run.stderr
        (SCRATCH / f'{name}.log').write_text(output)
        summary = [line for line in output.splitlines() if line.startswith('test result:')]
        assert run.returncode and summary and '1 failed;' in summary[-1], output
        print(f'{name}: {summary[-1]}', flush=True)
    finally:
        path.write_text(source)
print(f'WS8bm owned mutations: {len(mutations)} rejected; 0 survived; production files restored', flush=True)
