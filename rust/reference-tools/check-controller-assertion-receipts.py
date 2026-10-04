#!/usr/bin/env python3
"""Validate individual pinned controller assertions and their enabled/pass receipts."""
import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--nextest-list', type=Path, required=True)
parser.add_argument('--native-log', type=Path, required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
rust = root / 'rust'
manifest = json.loads((rust / 'plans/ledger-ws8br-ws17-ws11ui-b-receipts.json').read_text())
listing = json.loads(args.nextest_list.read_text())
active = {name for suite in listing['rust-suites'].values()
          for name, info in suite['testcases'].items() if not info['ignored']}
passed = set(re.findall(r'^\s*PASS\s+\[[^\]]+\]\s+\([^)]+\)\s+\S+\s+(\S+)',
                       args.native_log.read_text(), re.M))
records = manifest['records']
assert len({r['id'] for r in records}) == len(records)
assertions = 0
credited = set()
for record in records:
    source = subprocess.check_output(['git', 'show', f"{manifest['reference']}:{record['file']}"], cwd=root, text=True)
    assert hashlib.sha256(source.encode()).hexdigest() == record['source_sha256']
    declarations = list(re.finditer(r'^  test "([^"]+)" do$', source, re.M))
    declaration = next(m for m in declarations if m.group(1) == record['test'])
    index = declarations.index(declaration)
    end = declarations[index+1].start() if index+1 < len(declarations) else source.find('\n  private', declaration.end())
    if end == -1:
        end = len(source)
    start_line = source[:declaration.start()].count('\n') + 1
    original = {start_line+i: s.strip() for i, s in enumerate(source[declaration.start():end].splitlines())
                if s.strip().startswith(('assert', 'refute'))}
    receipts = {a['line']: a['ruby'] for a in record['assertions']}
    assert receipts == original, f"missing/changed original assertion: {record['id']}"
    for assertion in record['assertions']:
        assert assertion['rust_test'] in record['rust_tests']
        assert assertion['observation'] and assertion['checks']
        if assertion.get('disposition') == 'missing-discriminating-assertion':
            assert record['record_status'] == 'reopened' and assertion['historical_insufficient_receipt']
            continue
        assert assertion['assertion_scope'], f"missing executed assertion scope: {assertion}"
        assert not assertion['assertion_anchor'].startswith(('fn ', 'async fn ')), f"function declaration is not assertion evidence: {assertion}"
        path, line = assertion['assertion_source'].rsplit(':', 1)
        assert (root / path).read_text().splitlines()[int(line)-1].strip() == assertion['assertion_anchor'], f"stale assertion source: {assertion}"
        for extra in assertion.get('additional_assertion_sources', []):
            path, line = extra['path'].rsplit(':', 1)
            assert (root / path).read_text().splitlines()[int(line)-1].strip() == extra['anchor'], f"stale additional assertion source: {extra}"
    for name in record['rust_tests']:
        assert name in active, f'missing or ignored: {name}'
        assert name in passed, f'no actual pass receipt: {name}'
        credited.add(name)
    assertions += len(record['assertions'])
closed = sum(r['record_status'] == 'closed' for r in records)
gaps = sum(a.get('disposition') == 'missing-discriminating-assertion' for r in records for a in r['assertions'])
print(f'Controller per-assertion receipts: {len(records)} audited declarations ({closed} closed, {len(records)-closed} reopened); {assertions} assertion sites; {len(credited)} enabled native test identities passed; {gaps} explicit reopened gaps; 0 unaccounted assertions')
