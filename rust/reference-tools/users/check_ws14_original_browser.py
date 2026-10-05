#!/usr/bin/env python3
"""Validate unchanged pinned declaration inputs and actual assertion receipts."""
import argparse
import hashlib
import json
import pathlib
import re
import subprocess
from ws14_original_browser_support import helper_assertions

root = pathlib.Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--receipt-log', type=pathlib.Path)
parser.add_argument('--id-prefix', default='WS14')
args = parser.parse_args()
directory = root / 'rust/reference-tools/users/original_browser'
manifest = json.loads((directory / 'manifest.json').read_text())
assert manifest['reference'] == (root / 'rust/parity/reference.sha').read_text().strip()
for original, entry in manifest['files'].items():
    raw = subprocess.check_output(['git', 'show', manifest['reference'] + ':' + original], cwd=root)
    assert raw == (directory / original).read_bytes(), original
    assert hashlib.sha256(raw).hexdigest() == entry['sha256'], original
    assert str((directory / original).relative_to(root)) == entry['file'], original

for record in manifest['records']:
    lines = (directory / record['file']).read_text().splitlines()
    start = record['line'] - 1
    assert lines[start].strip() == 'test "' + record['title'] + '" do', record['id']
    end = next(i for i in range(start + 1, len(lines)) if lines[i] == '  end')
    expected = [i + 1 for i in range(start, end)
                if re.search(r'\bassert(?:_\w+)?\b|\.expects\(', lines[i])
                and not lines[i].lstrip().startswith('#')]
    assert expected == record['assertion_lines'], record['id']
    assert helper_assertions(directory, manifest, record) == record['helper_assertions'], record['id']
    wrapper = root / record.get('wrapper_file', 'rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs')
    assert wrapper.exists(), wrapper
    assert 'async fn original_browser_' + record['id'].lower().replace('-', '_') + '()' in wrapper.read_text()

if args.receipt_log:
    text = args.receipt_log.read_text()
    receipts = {rid: json.loads(raw) for rid, raw in
                re.findall(r'^\s*WS14_ORIGINAL_RECEIPT (WS\d+[eg]-\d+) (.+)$', text, re.MULTILINE)}
    selected = [row for row in manifest['records'] if row['id'].startswith(args.id_prefix)]
    for record in selected:
        receipt = receipts[record['id']]
        assert receipt['reference'] == manifest['reference']
        assert receipt['source'] == record['file']
        assert set(record['assertion_lines']) <= set(receipt['lines']), record['id']
        for helper in record['helper_assertions']:
            assert helper['line'] in receipt['files'].get(helper['file'], []), (record['id'], helper)
        function = record.get('wrapper_module', 'controllers::ws14_original_browser_tests') + '::original_browser_' + record['id'].lower().replace('-', '_')
        assert any('PASS' in line and line.endswith(' ' + function) for line in text.splitlines()), record['id']
    assert not re.search(r'\bFAIL\s+\[', text)
    print(f"Original browser receipts: {len(selected)} declarations; "
          f"{sum(len(row['assertion_lines']) for row in selected)} exact declaration assertion calls; "
          f"{sum(len(row['helper_assertions']) for row in selected)} nested helper assertions executed")
print(f"Original browser source: {len(manifest['files'])} byte-identical pinned files; {len(manifest['records'])} wrappers")
