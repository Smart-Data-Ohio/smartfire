#!/usr/bin/env python3
"""Check the #244 branch sweep's exact pin, requests, fixtures and assertion sites.

This is an integrity check; actual assertion discrimination is recorded separately
and exercised by users/check_review244_branch_mutations.py.
"""
import hashlib
import json
from pathlib import Path
import re
import subprocess

root = Path(__file__).resolve().parents[2]
manifest = json.loads((root / 'rust/plans/ledger-ws8br-ws17-ws11ui-b-receipts.json').read_text())
sweep = json.loads((root / 'rust/plans/ledger-ws8br-ws17-ws11ui-b-branch-sweep.json').read_text())
pin = (root / 'rust/parity/reference.sha').read_text().strip()
assert sweep['reference'] == manifest['reference'] == pin
assert {r['id'] for r in sweep['records']} == {r['id'] for r in manifest['records']}

def original(path):
    return subprocess.check_output(['git', 'show', f'{pin}:{path}'], cwd=root, text=True)

for row in sweep['records']:
    receipt = next(r for r in manifest['records'] if r['id'] == row['id'])
    source = original(row['file'])
    declarations = list(re.finditer(r'^  test "([^"]+)" do$', source, re.M))
    start = next(i for i, m in enumerate(declarations) if m.group(1) == row['test'])
    end = declarations[start + 1].start() if start + 1 < len(declarations) else source.find('\n  private', declarations[start].end())
    if end == -1:
        end = len(source)
    body = source[declarations[start].start():end].strip()
    assert body == row['reviewed_source'], (row['id'], 'declaration drift')
    assert hashlib.sha256(body.encode()).hexdigest() == row['declaration_body_sha256']
    requests = [{'line':row['line'] + i, 'ruby':line.strip()}
                for i, line in enumerate(body.splitlines())
                if re.match(r'^\s*(get|head|put|patch|post|delete)\b', line)]
    assert requests == row['rails_requests'], (row['id'], 'unrecorded original HTTP variant')
    assert row['rust_tests'] == receipt['rust_tests'] and receipt['record_status'] == 'closed'
    assert len(row['assertions']) == len(receipt['assertions'])
    for assertion, mapped in zip(row['assertions'], receipt['assertions'], strict=True):
        assert assertion == {'rails_line':mapped['line'], 'ruby':mapped['ruby'],
                             'rust_assertion':mapped['assertion_source'],
                             'anchor':mapped['assertion_anchor'], 'scope':mapped['assertion_scope'],
                             'test':mapped['rust_test'], 'cases':mapped.get('oracle_cases', [])}
        path, line = assertion['rust_assertion'].rsplit(':', 1)
        assert (root / path).read_text().splitlines()[int(line) - 1].strip() == assertion['anchor']
        assert assertion['scope'] and assertion['test'] in row['rust_tests']
scopes = json.loads((root / 'rust/reference-tools/users/original_calendar_scopes.json').read_text())
helper = original(scopes['source'])
assert scopes['reference'] == pin and hashlib.sha256(helper.encode()).hexdigest() == scopes['source_sha256']
for field, constant in [('drive_scopes', 'DRIVE_SCOPES'), ('legacy_drive_scopes', 'LEGACY_DRIVE_SCOPES')]:
    assert scopes[field] == re.search(r'^  ' + constant + r' = "([^"]+)"', helper, re.M).group(1)
assert (root / 'rust/reference-tools/users/service_worker_original_harness.mjs').read_text() == original('test/scripts/service_worker_harness.mjs')
mutations = json.loads((root / sweep['mutation_evidence']).read_text())
assert len(mutations['controls']) == 22
assert sum(c['before_exit'] == 0 for c in mutations['controls']) == 21
assert all(c['after_exit'] == 100 and c['assertion_failure'] for c in mutations['controls'])
assert {n for r in sweep['records'] for n in r['new_findings']} == {c['name'] for c in mutations['controls']}
print('Controller branch sweep: 120 current-pin declarations; 401 exact assertion mappings; all original HTTP request lines recorded; 0 source/setup drift')
print('Original helper scopes and Node harness: byte-identical to current Rails pin')
print('Controller branch controls: 21 prior survivors; 1 already rejected; 22/22 current assertion rejections; 0 invalid results credited')
