#!/usr/bin/env python3
"""Demand assertion failures for the new Calendar differentials; restore all sources."""
import argparse
import os
import subprocess
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--only', default='')
parser.add_argument('--scratch', type=Path)
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
scratch = args.scratch.resolve() if args.scratch else root.parent / '.scratch/ws14g/calendar-completion/mutations'
scratch.mkdir(parents=True, exist_ok=True)
cases = [
    ('focus-time', 'crates/db/src/models/google_meeting_cache.rs',
     '&& item["eventType"] != "focusTime"', '&& true',
     'campfire_db', 'google_named_meeting_and_ooo'),
    ('cache-uniqueness', 'crates/db/src/models/google_meeting_cache.rs',
     'if find(tx.conn(), user_id)?.is_some() {', 'if false {',
     'campfire_db', 'google_meeting_cache_creation_validates'),
    ('transient-intervals', 'crates/campfire/src/integrations/google/meeting_refresh.rs',
     'store_error(app, user_id, now, UNREACHABLE, true)',
     'store_error(app, user_id, now, UNREACHABLE, false)',
     'campfire', 'google_meeting_refresh_complete_states'),
    ('privacy-fields', 'crates/campfire/src/integrations/google/api.rs',
     'items(eventType,start,end,status,transparency,attendees(self,responseStatus)),nextPageToken',
     'items(summary,eventType,start,end,status,transparency,attendees(self,responseStatus)),nextPageToken',
     'campfire', 'google_meeting_refresh_complete_states'),
    ('throttle-boundary', 'crates/campfire/src/integrations/google/meeting_refresh.rs',
     't > now.ago(jiff::SignedDuration::from_secs(60))',
     't >= now.ago(jiff::SignedDuration::from_secs(60))',
     'campfire', 'google_meeting_refresh_complete_states'),
    ('claim-boundary', 'crates/db/src/models/google_meeting_cache.rs',
     'refresh_pending_at<=?', 'refresh_pending_at<?',
     'campfire', 'google_meeting_refresh_complete_states'),
    ('broadcast-set-true', 'crates/db/src/models/google_meeting_cache.rs',
     'fetch_error=excluded.fetch_error,fetched_at=excluded.fetched_at',
     'in_meeting_broadcast=1,fetch_error=excluded.fetch_error,fetched_at=excluded.fetched_at',
     'campfire', 'google_meeting_refresh_complete_states'),
    ('broadcast-set-false', 'crates/db/src/models/google_meeting_cache.rs',
     'fetch_error=excluded.fetch_error,fetched_at=excluded.fetched_at',
     'in_meeting_broadcast=0,fetch_error=excluded.fetch_error,fetched_at=excluded.fetched_at',
     'campfire', 'google_meeting_refresh_complete_states'),
]
checked = 0
for name, relative, before, after, package, test in cases:
    if args.only and name not in args.only.split(','):
        continue
    path = root / relative
    source = path.read_text()
    assert source.count(before) == 1, (name, source.count(before))
    try:
        path.write_text(source.replace(before, after))
        env = dict(os.environ, CI='1', TMPDIR=str(scratch), CARGO_BUILD_JOBS='2',
                   CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0', RUST_TEST_THREADS='8')
        result = subprocess.run(
            ['cargo', 'test', '--offline', '--locked', '-j', '2',
             '-p', package, test, '--', '--nocapture', '--test-threads=8'],
            cwd=root, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (scratch / f'{name}.log').write_text(result.stdout)
        summaries = [line for line in result.stdout.splitlines() if line.startswith('test result:')]
        assert result.returncode == 101 and 'assertion' in result.stdout and any('FAILED' in l for l in summaries), result.stdout
        print(name + ': ' + summaries[-1], flush=True)
        checked += 1
    finally:
        path.write_text(source)
print(f'Calendar cache discrimination: {checked} mutations rejected', flush=True)
