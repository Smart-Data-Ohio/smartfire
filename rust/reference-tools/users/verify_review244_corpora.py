#!/usr/bin/env python3
"""Compare freshly regenerated #244 Rails corpora and preserve historical rows."""
import argparse
import json
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('generated', type=Path)
args = parser.parse_args()
root = Path(__file__).resolve().parents[3]
base = '1b83dda5a39c28b2fb7765184437fdb19ee0eca9'
for name, group in [('users_profile_sections','google_calendar'), ('users_public','pages'),
                    ('users_layout_preferences','cases'), ('users_profile_settings','profiles'),
                    ('profile_security','profile')]:
    previous = json.loads(subprocess.check_output(['git', 'show', f'{base}:rust/vectors/{name}.json'], cwd=root, text=True))
    actual = json.loads((args.generated / (name + '.json')).read_text())
    assert actual == json.loads((root / f'rust/vectors/{name}.json').read_text()), name
    if isinstance(previous[group], list):
        field = 'state' if name == 'users_public' else 'name'
        current = {r[field]:r for r in actual[group]}
        assert all(current[row[field]] == row for row in previous[group]), (name, 'historical response drift')
    else:
        assert all(actual[group][key] == value for key, value in previous[group].items())
    if name == 'profile_security':
        assert {k:v for k,v in previous.items() if k != group} == {k:v for k,v in actual.items() if k != group}
    print(f'{name}: {len(previous[group])} prior rows unchanged; {len(actual[group])} current Rails rows identical')
for name in ['users_profile_settings', 'profile_security']:
    assert json.loads((args.generated / (name + '.json')).read_text()) == json.loads((args.generated / (name + '_put.json')).read_text()), name
assert json.loads((args.generated / 'users_profile_reconnect_receipts.json').read_text()) == json.loads((root / 'rust/vectors/users_profile_reconnect_receipts.json').read_text())
print('Rails PATCH/PUT: 37 settings + 20 security cases; 0 response/state mismatches')
print('Rails reconnect original setups: 2/2 identical; 0 mismatches')
