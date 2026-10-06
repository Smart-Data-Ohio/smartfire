#!/usr/bin/env python3
"""Require the real board presenter to reject per-post parent lookups."""
import argparse
import os
import subprocess
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--scratch', type=Path)
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
scratch = args.scratch.resolve() if args.scratch else root.parent / '.scratch/ws14g/board-mutations'
scratch.mkdir(parents=True, exist_ok=True)
path = root / 'crates/campfire/src/controllers/presenters/boards.rs'
source = path.read_text()
before = '.status_in_room(room, Timestamp::from_jiff(p.now))'
after = '.status(p.conn, Timestamp::from_jiff(p.now))?'
assert source.count(before) == 1
try:
    path.write_text(source.replace(before, after))
    env = dict(os.environ, CI='1', TMPDIR=str(scratch), CARGO_BUILD_JOBS='2',
               CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0', RUST_TEST_THREADS='8')
    result = subprocess.run(['cargo', 'test', '--offline', '--locked', '-j', '2',
                             '-p', 'campfire', 'board_rows_reuse_the_loaded_room_for_aged_posts',
                             '--', '--nocapture', '--test-threads=8'],
                            cwd=root, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    (scratch / 'per-post-room-read.log').write_text(result.stdout)
    summaries = [l for l in result.stdout.splitlines() if l.startswith('test result:')]
    assert result.returncode == 101 and 'assertion' in result.stdout and any('FAILED' in l for l in summaries), result.stdout
    print('per-post-room-read: ' + summaries[-1], flush=True)
finally:
    path.write_text(source)
print('Board lifecycle discrimination: 1 mutation rejected; sources restored', flush=True)
