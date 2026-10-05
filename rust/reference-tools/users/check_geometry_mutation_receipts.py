#!/usr/bin/env python3
"""Discriminate DOM-first geometry rejection from invalid mutation/transport runs."""
import argparse
import json
from pathlib import Path
import re
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--evidence', type=Path, required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[3]
pin = (root / 'rust/parity/reference.sha').read_text().strip()
rails = subprocess.check_output(
    ['git', 'show', pin + ':test/system/people_group_dms_test.rb'], cwd=root, text=True,
)
helper = (root / 'rust/reference-tools/users/browser_scopes.mjs').read_text()
for component in ('row_height', 'avatar_width'):
    expression = re.search(component + r' = page.evaluate_script\("([^"\n]+)"\)', rails)[1]
    assert expression in helper, (component, 'Rails geometry query changed')
legacy = (root / 'rust/reference-tools/users/browser_picker.mjs').read_text().splitlines()
height_line = next(n for n, line in enumerate(legacy, 1) if 'assert.ok(rect.height>=44' in line)
for name, case, rejection in (
    ('original', 'ORIGINAL_CASE picker-phone: passed',
     'test/system/people_group_dms_test.rb:303: original assertion'),
    ('legacy', 'picker-phone-targets-and-width: passed', f'browser_picker.mjs:{height_line}:'),
):
    before = (args.evidence / f'{name}-before.log').read_text()
    after = (args.evidence / f'{name}-after.log').read_text()
    for phase, log in (('before', before), ('after', after)):
        match = re.search(r'^GEOMETRY_MUTATION phone-first-row (.+)$', log, re.M)
        assert match, (name, phase, 'producer mutation not reached')
        geometry = json.loads(match[1])
        assert geometry['height'] == geometry['avatar'] == 0
        assert geometry['matched'] >= 2 and geometry['nextHeight'] >= 44
        assert abs(geometry['nextAvatar'] - 32) <= 1
        assert not any(s in log for s in (
            'INVALID_CONTROL', 'Rust server exited', 'failed startup',
            'ERR_NETWORK_CHANGED', 'ERR_CONNECTION_REFUSED', 'TimeoutError',
        )), (name, phase, 'invalid setup/transport control')
    assert case in before and case not in after, (name, 'mutation must survive then fail')
    assert 'ERR_ASSERTION' in after and rejection in after, (name, 'wrong rejection')
    print(f'Geometry control {name}: survived before; rejected after at Rails row-height predicate; 0 invalid controls')
print('Geometry mutation receipts: 2 surviving before; 2 rejected after; 0 invalid controls')
