#!/usr/bin/env python3
"""Regenerate the new Calendar fixtures from the Rails pin, with Google HTTP recorded only."""

from pin_identity import PIN, PIN_FULL, PIN_IMAGE
import argparse
import hashlib
import json
import os
import subprocess
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--only', default='refresh,intervals,push,clock')
parser.add_argument('--log-dir', type=Path)
args = parser.parse_args()
selected = set(args.only.split(','))
assert selected <= {'refresh', 'intervals', 'push', 'clock'}, selected
root = Path(__file__).resolve().parents[2]
scratch = args.log_dir.resolve() if args.log_dir else root.parent / '.scratch/ws14g/calendar-completion/logs'
scratch.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, PARITY_NAMESPACE='ws14g', PARITY_OWNER='ws14g', PARITY_IMAGE=PIN_IMAGE)
base = [str(root / 'parity/bin/reference'), 'runner', '--seed', 'default',
        '--time', '2026-09-23T10:30:00Z', '--freeze']

def run(script, name, extra=()):
    result = subprocess.run(base + list(extra) + [str(root / 'reference-tools/google' / script)],
                            cwd=root, env=env, text=True, capture_output=True)
    (scratch / f'{name}-rails.log').write_text(result.stderr)
    assert result.returncode == 0, result.stderr
    value = json.loads(result.stdout)
    (scratch / f'{name}.json').write_text(result.stdout)
    for line in result.stderr.splitlines():
        if line.startswith('Pinned Rails '):
            print(line, flush=True)
    return value, result.stdout

for name, script, vector in [
    ('refresh', 'meeting-refresh-cases.rb', 'google_meeting_refresh.json'),
    ('intervals', 'named-intervals.rb', 'google_named_intervals.json'),
    ('push', 'push-channel-cases.rb', 'google_push_channels.json'),
]:
    if name not in selected:
        continue
    value, text = run(script, name)
    assert value['reference'] == PIN
    if name == 'intervals':
        for row in value['rows'] + [value['cache_uniqueness']]:
            source = subprocess.check_output(['git', 'show', f"{PIN_FULL}:{row['file']}"], cwd=root)
            assert hashlib.sha256(source).hexdigest() == row['source_sha256'], row['file']
        print(f'Pinned Rails named sources: 3 source digests match {PIN} exactly', flush=True)
    (root / 'vectors' / vector).write_text(text)

if 'clock' in selected:
    rows = []
    for fraction in ['000000', '123456', '999999']:
        value, _ = run('cache-clock-precision.rb', f'cache-clock-{fraction}',
                       ['-e', f'FAKETIME=2026-09-23 10:30:00.{fraction}'])
        assert value['now'].endswith(f'.{fraction}Z'), value
        rows.append(value)
    (root / 'vectors/google_cache_clock.json').write_text(
        json.dumps({'reference': PIN, 'rows': rows}, indent=2) + '\n')
    line = 'Pinned Rails cache clock: 3 fractional observations; 6 upserts; microsecond fetched_at; millisecond SQL timestamps; unchanged upserts preserve timestamps'
    (scratch / 'cache-clock-summary.log').write_text(line + '\n')
    print(line, flush=True)
