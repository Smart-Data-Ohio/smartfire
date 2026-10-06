#!/usr/bin/env python3
"""Demand assertion failures for viewer isolation, recipients and cleanup; restore every mutation."""
import os, subprocess
from pathlib import Path
root = Path(__file__).resolve().parents[2]
scratch = root.parent / '.scratch/ws14g/mutations'
scratch.mkdir(parents=True, exist_ok=True)
def check(name, relative, before, after, test, count=-1):
    path = root / relative
    source = path.read_text()
    assert before in source, name
    try:
        path.write_text(source.replace(before, after, count))
        env = dict(os.environ, CI='1', TMPDIR=str(scratch), CARGO_PROFILE_TEST_DEBUG='0', CARGO_PROFILE_DEV_DEBUG='0')
        result = subprocess.run(['cargo', 'test', '--locked', '-j', '4', '-p', 'campfire', test, '--', '--nocapture'], cwd=root, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (scratch / f'{name}.log').write_text(result.stdout)
        lines = [line for line in result.stdout.splitlines() if line.startswith('test result:')]
        assert result.returncode == 101 and 'assertion' in result.stdout and any('FAILED' in line for line in lines), result.stdout
        print('\n'.join(lines), flush=True)
        print(f'Google continuation mutation {name}: rejected', flush=True)
    finally:
        path.write_text(source)
check('drive-viewer-cache', 'crates/campfire/src/integrations/google/drive.rs', '(user_id, file_id.into())', '(0, file_id.into())', 'google_drive_viewer_inaccessible_file')
check('drive-forbidden-response', 'crates/campfire/src/integrations/google/api.rs', '403 if drive => missing(),', '403 if drive => Error::Rejected("forbidden".into()),', 'google_drive_viewer_inaccessible_file')
check('recipient-self-bypass', 'crates/campfire/src/controllers/google_drive.rs', 'drive_recipients::eligible(conn, room, id)', 'drive_recipients::eligible(conn, room, -id)', 'google_drive_recipients_require_human_membership')
check('cleanup-retry-bypass', 'crates/campfire/src/integrations/google/calendar.rs', 'Err(e) if e.unavailable() => return Err(e),', 'Err(e) if e.unavailable() => return Err(api::Error::Rejected("quota".into())),', 'google_calendar_cleanup_deletes_before_revoke', count=1)
print('Google continuation discrimination: 4 mutations rejected', flush=True)
