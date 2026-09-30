#!/usr/bin/env python3
"""Reproduce failing-first assertions. Every source is restored, including on failure."""
import os
import pathlib
import subprocess

root = pathlib.Path(__file__).resolve().parents[3]
scratch = pathlib.Path('/home/riels/.cache/rust-port/ws8slash')
scratch.mkdir(parents=True, exist_ok=True)
env = os.environ | {'TMPDIR': str(scratch / 'tmp'), 'CARGO_TARGET_DIR': str(root / 'rust/target')}

def run(label, relative, before, after, package, test, required):
    path = root / relative
    source = path.read_text()
    assert before in source, (label, 'mutation anchor changed')
    try:
        path.write_text(source.replace(before, after, 1))
        command = ['mise', 'exec', 'rust@1.98.1', '--', 'cargo', 'test', '-j', '6', '--manifest-path', str(root / 'rust/Cargo.toml'), '-p', package, test, '--', '--test-threads=4']
        result = subprocess.run(command, cwd=root, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (scratch / f'discrimination-{label}.log').write_text(result.stdout)
        assert result.returncode != 0 and 'test result: FAILED.' in result.stdout, (label, result.stdout[-3000:])
        for name in required:
            assert f'::{name} ... FAILED' in result.stdout, (label, name, result.stdout[-3000:])
        assert 'assertion' in result.stdout, (label, 'not an assertion failure')
        print(f'{label}: detected ({len(required)} named tests)')
        print(next(line for line in result.stdout.splitlines() if line.startswith('test result: FAILED.')))
    finally:
        path.write_text(source)

run('registry', 'rust/crates/db/src/slash_commands.rs', 'root: true,', 'root: false,', 'campfire_db', 'slash_registry', ['slash_registry_and_recognition_match_rails'])
run('parser', 'rust/crates/db/src/slash_commands/time_parser.rs', 'let text = strip(text);', 'if !text.is_empty() { return None; }\n    let text = strip(text);', 'campfire_db', 'slash_parser', ['slash_parser_matches_rails_timezone_and_dst_vectors'])
run('dispatch', 'rust/crates/db/src/slash_commands.rs', '"poll" => Ok(CommandResult::new("open_poll")),', '"poll" => Ok(CommandResult::new("ephemeral")),', 'campfire_db', 'slash_dispatch_and_rows', ['slash_dispatch_and_rows_match_rails'])
run('callbacks', 'rust/crates/db/src/slash_commands.rs', 'Event::Broadcast(Broadcast::append(', 'Event::Broadcast(Broadcast::replace(', 'campfire_db', 'slash_callbacks', ['slash_callbacks_match_rails'])
run('validation', 'rust/crates/db/src/slash_commands/user_settings.rs', 'errors.into_result()?;', 'let _ = errors;', 'campfire_db', 'slash_preexisting', ['slash_preexisting_user_validations_and_calendar_match_rails'])
run('push-atomicity', 'rust/crates/db/src/models/message.rs', 'message.push_later_in_conversation(tx);', '// omitted by discrimination check', 'campfire_jobs', 'ws8_slash', ['slash_root_reminder_and_push_are_one_transaction', 'slash_thread_membership_message_and_push_roll_back_together', 'slash_legacy_webhook_rejection_rolls_back_the_root_post_and_push'])
run('webhook-atomicity', 'rust/crates/db/src/slash_commands.rs', 'bot.deliver_webhook_later(tx, message.id)?;', 'let _ = bot;', 'campfire_jobs', 'slash_legacy_webhook_rejection', ['slash_legacy_webhook_rejection_rolls_back_the_root_post_and_push'])
run('runtime-richtext', 'rust/crates/campfire/src/rich_text.rs', '        let mut stmt=conn.prepare_cached(', '        if !source.is_empty() { return Ok(source.into()); }\n        let mut stmt=conn.prepare_cached(', 'campfire', 'slash_runtime_', ['slash_runtime_richtext_and_index_rows_match_rails'])
print('WS8 slash discrimination: 8 mutations detected; all 9 new tests failed; sources restored')
