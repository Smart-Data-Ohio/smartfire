#!/usr/bin/env python3
"""Run the pin's original OOO/meeting tests; record actual operation sequences."""
import json
import os
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[2]
scratch = root / '.scratch'
subprocess.run(['python3', 'rust/reference-tools/ws17_verify_reference.py'], cwd=root, check=True)
for name in ['out_of_office', 'meeting_status']:
    expected = subprocess.check_output(['git', 'show', f'd7c7de92:test/models/user/{name}_test.rb'], cwd=root)
    assert (root / f'rust/reference-tools/pinned/ws17-{name}_test.rb').read_bytes() == expected
print('pinned calendar status declarations verified: 2 files byte-identical to d7c7de92')
env = dict(os.environ, PARITY_NAMESPACE='ws17', PARITY_OWNER='ws17', PARITY_IMAGE='triage-reference-d7c7de92:latest')
with (scratch / 'named-calendar-status-generated.json').open('w') as out, (scratch / 'named-calendar-status-generated.log').open('w') as err:
    subprocess.run(['rust/parity/bin/reference', 'runner', '--seed', 'default', '--time', '2026-09-23T12:00:00Z',
                    '--freeze', 'rust/reference-tools/ws17_named_calendar_status.rb'], cwd=root, env=env, stdout=out, stderr=err, check=True)
v = json.loads((scratch / 'named-calendar-status-generated.json').read_text())
assert v['reference'] == 'd7c7de92'
(root / 'rust/vectors/ws17_named_calendar_status.json').write_text(json.dumps(v, ensure_ascii=False) + '\n')
for name in ['out_of_office', 'meeting_status']:
    rows = [row for row in v['rows'] if row['file'] == f'test/models/user/{name}_test.rb']
    print('Rails named %s: %d passed cases; %d original Rails assertions; %d operations' %
          (name, len(rows), sum(row['assertions'] for row in rows), sum(len(row['steps']) for row in rows)))
