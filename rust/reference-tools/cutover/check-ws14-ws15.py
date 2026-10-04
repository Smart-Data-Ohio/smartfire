#!/usr/bin/env python3
"""Check acceptance IDs, physical citations, exact CI PASS receipts and open rows.

Optional --ci-log accepts the raw `gh run view 37200618245 --log` output, so the
stored baseline receipts can be verified without committing the entire job log.
This is an inventory integrity check; it does not decide semantic test coverage.
"""
import argparse
import collections
import hashlib
import json
import pathlib
import re

root = pathlib.Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--ci-log', type=pathlib.Path)
args = parser.parse_args()
data = json.loads((root / 'rust/plans/ledger-ws14-ws15.json').read_text())
records = data['records']
assert len(records) == 445
assert collections.Counter(r['owner'] for r in records) == {'WS14e': 108, 'WS14g': 273, 'WS15g': 64}
assert len({r['id'] for r in records}) == 445
allowed = {'passed', 'implemented', 'outside_gate', 'unsupported_assertion'}
ci = data['baseline']['execution_receipts']
if args.ci_log:
    raw = args.ci_log.read_bytes()
    assert hashlib.sha256(raw).hexdigest() == data['baseline']['raw_log_sha256']
    text = re.sub(r'\^\[\[[0-9;]*[mK]', '', raw.decode())
    actual = {}
    for number, line in enumerate(text.splitlines(), 1):
        match = re.search(r'PASS\s+\[.*?\]\s+\(.*?\)\s+(\S+)\s+(.+)', line)
        if match:
            actual[match[1] + ' ' + match[2]] = number
    assert len(actual) == data['baseline']['tests_passed'] == 4948
    for test, line in ci.items():
        assert actual[test] == line, test
    assert not re.search(r'FAIL\s+\[', text)

remaining = (root / 'rust/plans/ledger-ws14-ws15-remaining.md').read_text()
for r in records:
    assert r['disposition'] in allowed, r['id']
    assert r['historical_receipt'], r['id']
    ledger = root / 'rust/plans' / r['ledger']
    line = ledger.read_text().splitlines()[r['line'] - 1]
    assert line.count('[' + r['id'] + ']') == 1, r['id']
    evidence = r['evidence']
    source = evidence['source']
    source_line = (root / source['file']).read_text().splitlines()[source['line'] - 1]
    if r['disposition'] in {'passed', 'implemented'}:
        assert 'test' in evidence
        test = evidence['test']
        name = test.split(' ')[-1].split('::')[-1]
        assert re.search(r'\b' + re.escape(name) + r'\b', source_line), (r['id'], source)
        assert test in line, r['id']
        if r['disposition'] == 'passed':
            assert evidence['ci_log_line'] == ci[test], r['id']
        else:
            assert test not in ci, r['id']  # Never attribute new tests to the old run.
    elif r['disposition'] == 'outside_gate':
        assert r['id'] == 'WS14g-268'
        assert 'WebMock' in evidence['detail']
    if r['disposition'] == 'unsupported_assertion':
        assert f"| {r['id']} |" in remaining, r['id']
        assert f"rust/plans/{r['ledger']}:{r['line']}" in remaining, r['id']
        assert f"{r['rails']}:{r['rails_line']}" in remaining, r['id']
        assert r['title'] in source_line, r['id']
    else:
        assert f"| {r['id']} |" not in remaining, r['id']

counts = collections.Counter(r['disposition'] for r in records)
print(f"Acceptance ledger: {len(records)} records checked; {counts['passed']} baseline-CI passed; "
      f"{counts['implemented']} new assertions; {counts['outside_gate']} test-only outside gate; "
      f"{counts['unsupported_assertion']} explicitly open")
if args.ci_log:
    print('CI receipt verification: 4948 PASS entries; 0 FAIL entries; all cited names/lines match run 37200618245')
