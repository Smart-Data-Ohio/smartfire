#!/usr/bin/env python3
"""Compile regressions in this continuation's authorization, privacy, validation and upload paths."""
import os
from pathlib import Path
import re
import subprocess
ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / '.scratch/lifecycle-discrimination'
OUT.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, CI='1', TMPDIR=str(ROOT / '.scratch'), CARGO_TARGET_DIR=str(ROOT / 'rust/target'),
           CABLE_TEST_PORT_RANGE='52000-52049', MAIL_TEST_PORT_RANGE='52000-52049')
def check(name, path, old, new, test):
    source = ROOT / path
    original = source.read_text()
    assert old in original, name
    try:
        source.write_text(original.replace(old, new))
        result = subprocess.run(['mise', 'exec', 'rust@1.98.1', '--', 'cargo', 'test', '--locked', '-j4',
            '--manifest-path', 'rust/Cargo.toml', '-p', 'campfire', '--bin', 'campfire', test, '--', '--exact'],
            cwd=ROOT, env=env, capture_output=True, text=True)
        output = result.stdout + result.stderr
        (OUT / f'{name}.log').write_text(output)
        summary = re.findall(r'^test result: FAILED\..*$', output, re.M)
        assert result.returncode != 0 and summary and f'test {test} ... FAILED' in output, name
        print(f'{name}: {summary[0]}', flush=True)
    finally:
        source.write_text(original)
writes = 'rust/crates/campfire/src/controllers/channel_threads/writes.rs'
write_tests = 'controllers::channel_threads::write_tests::'
check('creator-lock-permission', writes, 'Some("locked") => moderator,', 'Some("locked") => settings,',
      write_tests + 'thread_creator_cannot_moderate_or_delete_even_when_joined')
check('creator-delete-permission', writes, 'if !thread.manageable_by(tx.conn(), &actor)?', 'if !thread.settings_manageable_by(tx.conn(), &actor)?',
      write_tests + 'thread_creator_cannot_moderate_or_delete_even_when_joined')
check('initial-upload-processing', writes, 'if let Some(blob) = blob { messages::process_attachment(c.app(), blob).await?; }', 'let _ = blob;',
      write_tests + 'initial_thread_upload_is_processed_and_downloadable')
check('tag-validation-atomicity', 'rust/crates/db/src/models/channel_thread.rs', 'changed.validate(tx.conn(), &room, names.as_deref())?.into_result()?;', 'let _ = &room; let _ = &names;',
      write_tests + 'lifecycle_actions_match_rails_responses_and_atomic_rows')
check('content-anchor-scope', 'rust/crates/campfire/src/controllers/channel_threads.rs', 'messages::paging_anchor(conn, Timeline::Thread(id), value)',
      'Message::find(conn, value.to_s().as_deref().and_then(cast_integer).unwrap_or(0))',
      'controllers::channel_threads::content_tests::content_scopes_room_and_anchor_without_joining_and_denies_bots')
forwards = 'rust/crates/campfire/src/controllers/message_forwards.rs'
forward_tests = 'controllers::message_forwards_tests::'
check('forward-source-scope', forwards, 'if message.room_id != room || message.thread_id != thread',
      'if false && (message.room_id != room || message.thread_id != thread)',
      forward_tests + 'forward_endpoints_scope_sources_and_reject_bots_and_forgery_first')
check('forward-source-privacy', forwards, 'match Message::find_reachable(conn, viewer, id)', 'match Message::find(conn, id)',
      forward_tests + 'pickers_refusals_and_private_source_urls_match_rails_bytes')
check('forward-create-csrf', forwards, 'pub async fn create(c: &mut Ctx) -> Result {\n    before_actions(c, Before::default()).await?;',
      'pub async fn create(c: &mut Ctx) -> Result {\n    before_actions(c, Before::default().skip_forgery_protection()).await?;',
      forward_tests + 'forward_endpoints_scope_sources_and_reject_bots_and_forgery_first')
print('WS8bm lifecycle discrimination: 8 compiled regressions detected; sources restored', flush=True)
