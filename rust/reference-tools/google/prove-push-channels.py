#!/usr/bin/env python3
"""Reject broken channel authentication, ordering and preload behavior; always restore."""
import argparse
import os
import subprocess
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--scratch', type=Path)
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
scratch = args.scratch.resolve() if args.scratch else root.parent / '.scratch/ws14g/calendar-completion/push-mutations'
scratch.mkdir(parents=True, exist_ok=True)
cases = [
    ('token-authentication', 'crates/db/src/models/google_calendar.rs',
     '&& bool::from(', '&& !bool::from(',
     'google_push_channel_tokens_and_notification_claims'),
    ('notification-replay', 'crates/db/src/models/google_calendar.rs',
     'last_message_number<?', 'last_message_number<=?',
     'google_push_channel_tokens_and_notification_claims'),
    ('old-channel-stop', 'crates/campfire/src/integrations/google/calendar.rs',
     'if let Some((_, old_channel, Some(resource_id))) = old',
     'if let Some((_, old_channel, Some(resource_id))) = None::<(i64, String, Option<String>)>',
     'google_push_channel_watch_renewal_and_preload'),
    ('user-preload', 'crates/campfire/src/integrations/google/calendar.rs',
     'for (id, user, expiry) in channels {',
     'for (id, user, expiry) in channels {\n'
     '        app.db.read(move |conn| Ok(conn.query_row("SELECT id FROM users WHERE id=?", [user], |r| r.get::<_, i64>(0))?)).await?;',
     'google_push_channel_watch_renewal_and_preload'),
    ('account-preload', 'crates/campfire/src/integrations/google/calendar.rs',
     'for (id, user, expiry) in channels {',
     'for (id, user, expiry) in channels {\n'
     '        let _ = app.db.read(move |conn| GoogleAccount::for_user(conn, user)).await?;',
     'google_push_channel_watch_renewal_and_preload'),
    ('renewal-boundary', 'crates/campfire/src/integrations/google/calendar.rs',
     't <= now.since(jiff::SignedDuration::from_hours(24))',
     't < now.since(jiff::SignedDuration::from_hours(24))',
     'google_push_channel_watch_renewal_and_preload'),
    ('watch-credential-rescue', 'crates/campfire/src/integrations/google/calendar.rs',
     'let response = match async {\n'
     '        let mut credentials = api\n'
     '            .credentials_from_account(&app.db, &app.secrets, account)\n'
     '            .await?;',
     'let mut credentials = api\n'
     '        .credentials_from_account(&app.db, &app.secrets, account)\n'
     '        .await?;\n'
     '    let response = match async {',
     'google_push_channel_unreadable_access_records_watch_error'),
]
for name, relative, before, after, test in cases:
    path = root / relative
    source = path.read_text()
    assert source.count(before) == 1, (name, source.count(before))
    try:
        path.write_text(source.replace(before, after))
        env = dict(os.environ, CI='1', TMPDIR=str(scratch), CARGO_BUILD_JOBS='2',
                   CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0', RUST_TEST_THREADS='8')
        run = subprocess.run(['cargo', 'test', '--offline', '--locked', '-j', '2',
                              '-p', 'campfire', test, '--', '--nocapture', '--test-threads=8'],
                             cwd=root, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (scratch / f'{name}.log').write_text(run.stdout)
        summaries = [line for line in run.stdout.splitlines() if line.startswith('test result:')]
        needle = 'per-channel user lookup regressed' if name == 'user-preload' else 'assertion'
        assert run.returncode == 101 and needle in run.stdout and any('FAILED' in l for l in summaries), run.stdout
        print(name + ': ' + summaries[-1], flush=True)
    finally:
        path.write_text(source)
print(f'Calendar channel discrimination: {len(cases)} mutations rejected', flush=True)
