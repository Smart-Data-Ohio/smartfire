#!/usr/bin/env python3
"""Check acceptance IDs, assertion citations, exact CI PASS receipts and open rows.

Optional --ci-log accepts the raw `gh run view 37200618245 --log` output, so the
stored baseline receipts can be verified without committing the entire job log.
Optional --nextest-log verifies the final local run's hash, counts and every
retained closure's named test execution, separately from the historical CI run.
This is an inventory integrity check; it does not decide semantic test coverage.
"""
import argparse
import collections
import hashlib
import json
import pathlib
import re
import subprocess

root = pathlib.Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--ci-log', type=pathlib.Path)
parser.add_argument('--nextest-log', type=pathlib.Path)
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
remaining_ids = re.findall(r'^\| (WS\d+[eg]-\d+) \|', remaining, re.MULTILINE)
open_ids = {r['id'] for r in records if r['disposition'] == 'unsupported_assertion'}
assert len(remaining_ids) == len(set(remaining_ids)), 'Duplicate remaining record'
assert set(remaining_ids) == open_ids, 'Remaining list differs from ledger'
continuation = data.get('continuation_audit')
continuation_ids = set()
if continuation:
    for file in continuation['maps']:
        mapping = json.loads((root / file).read_text())
        for entry in mapping['records']:
            assert entry['id'] not in continuation_ids
            continuation_ids.add(entry['id'])
            record = next(r for r in records if r['id'] == entry['id'])
            assert record['disposition'] == 'implemented', entry['id']
            assert record['evidence']['test'] == entry['test'], entry['id']
            assert record['evidence']['assertion_map']['file'] == file, entry['id']
    # The separate continuation schema checks every Rails call, exact native
    # assertion bytes and physical source locations; it is not an audit waiver.
    subprocess.run(['python3', str(root / 'rust/reference-tools/cutover/assertion-maps.py'),
                    *(str(root / f) for f in continuation['maps'])], check=True)
    assert len(continuation_ids) == continuation['closed_records']
    assert continuation['parent_open_total'] == data['closure_audit']['open_total']
    assert len(open_ids) == continuation['open_total']
audited = []
assertion_count = 0
mapped_count = 0


def source_lines(file):
    return (root / file).read_text().splitlines()


def assertion_lines(record):
    """All explicit assertion calls in this Rails declaration, excluding helpers."""
    lines = source_lines(record['rails'])
    start = record['rails_line'] - 1
    indent = len(lines[start]) - len(lines[start].lstrip())
    end = len(lines)
    for index in range(start + 1, len(lines)):
        line = lines[index]
        if (len(line) - len(line.lstrip()) == indent
                and re.match(r'\s*(test\s+["\']|def\s|private\b|end\s*$)', line)):
            end = index
            break
    return {index + 1 for index in range(start + 1, end)
            if re.search(r'\b(?:assert\w*|refute\w*)\b', lines[index])
            and not lines[index].lstrip().startswith('#')}


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
            # Existing test names can acquire new assertions. Their old execution
            # receipt is historical and does not certify the added assertions.
            assert 'ci_log_line' not in evidence, r['id']
            if test in ci:
                assert evidence.get('supplemental_to_baseline') is True, r['id']
            prior = evidence['baseline_receipt']
            if 'ci_log_line' in prior:
                assert prior['ci_log_line'] == ci[prior['test']], r['id']
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
    audit = r.get('assertion_audit')
    if not audit:
        if r['id'] in continuation_ids:
            assert r['continuation'] in {'rust/ledger-ws14-ws15-b', 'rust/ledger-ws14-ws15-c'}, r['id']
        else:
            assert r['disposition'] == 'unsupported_assertion', r['id']
        continue
    audited.append(r)
    assert r['prior_disposition'] in {'passed', 'implemented', 'outside_gate'}, r['id']
    assert audit['outcome'] in {'retain', 'reopen', 'outside_gate'}, r['id']
    maps = audit['assertion_map']
    rails_locations = set()
    rust_locations = collections.defaultdict(set)
    for mapping in maps:
        file, number = mapping['rails'].rsplit(':', 1)
        assert file == r['rails'], r['id']
        number = int(number)
        assert number > r['rails_line'], r['id']
        assert source_lines(file)[number - 1].strip(), (r['id'], mapping)
        assert number not in rails_locations, (r['id'], number)
        rails_locations.add(number)
        assertion_count += 1
        if mapping['rust']:
            mapped_count += 1
        for location in mapping['rust']:
            file, number = location.rsplit(':', 1)
            number = int(number)
            lines = source_lines(file)
            actual = lines[number - 1]
            # Complete byte comparisons use a failure helper on mismatch.
            is_mismatch = (re.search(r'if\s+\w+\s*!=\s*\w+\s*\{', actual)
                           and 'rails_mismatch(' in lines[number])
            assert (re.search(r'\b(?:assert\w*!|expect\s*\(|assert_\w+\s*\()', actual)
                    or is_mismatch), (r['id'], location, actual)
            rust_locations[file].add(number)
    assert assertion_lines(r) <= rails_locations, (r['id'], assertion_lines(r) - rails_locations)
    if audit['outcome'] == 'retain':
        assert r['disposition'] in {'passed', 'implemented'}, r['id']
        assert maps and all(m['rust'] for m in maps), r['id']
        assert not audit['missing_assertions'], r['id']
        compact = '; '.join('`' + file + ':' + ','.join(map(str, sorted(numbers))) + '`'
                            for file, numbers in sorted(rust_locations.items()))
        assert f'Assertions: {compact}.' in line, r['id']
    elif audit['outcome'] == 'reopen':
        assert r['disposition'] == 'unsupported_assertion', r['id']
        assert audit['missing_assertions'], r['id']
        assert r['evidence']['detail'] in line, r['id']
        assert r['evidence']['detail'].replace('|', '&#124;') in remaining, r['id']
    else:
        assert r['disposition'] == 'outside_gate', r['id']

summary = data['closure_audit']
retained = {r['id'] for r in audited if r['assertion_audit']['outcome'] == 'retain'}
reopened = {r['id'] for r in audited if r['assertion_audit']['outcome'] == 'reopen'}
outside = {r['id'] for r in audited if r['assertion_audit']['outcome'] == 'outside_gate'}
strengthened = {r['id'] for r in audited
                if r['id'] in retained and r['assertion_audit']['added_assertions']}
assert len(audited) == summary['records_audited'] == 220
assert len(retained | reopened) == summary['previously_closed'] == 219
assert assertion_count == summary['rails_assertions'] == 555
assert set(summary['retained']) == retained
assert set(summary['reopened']) == reopened
assert set(summary['outside_gate']) == outside == {'WS14g-268'}
assert set(summary['closed_with_new_assertions']) == strengthened
if not continuation:
    assert len(open_ids) == summary['open_total']
if args.nextest_log:
    raw = args.nextest_log.read_bytes()
    receipt = (continuation['verification']['nextest'] if continuation else summary['verification']['nextest'])
    assert hashlib.sha256(raw).hexdigest() == receipt['raw_log_sha256']
    text = raw.decode()
    names = [package + ' ' + name for package, name in
             re.findall(r'PASS\s+\[.*?\]\s+\(.*?\)\s+(\S+)\s+(.+)', text)]
    assert len(names) == len(set(names)) == receipt['passed']
    assert not re.search(r'\bFAIL\s+\[', text)
    run = re.search(r'Summary\s+\[[^]]+\]\s+(\d+) tests run: (\d+) passed'
                    r'(?: \([^)]*\))?, (\d+) skipped', text)
    assert run and int(run[1]) == int(run[2]) == receipt['passed']
    assert int(run[3]) == receipt['skipped']
    for record in records:
        if record['disposition'] in {'passed', 'implemented'}:
            assert record['evidence']['test'] in names, record['id']

counts = collections.Counter(r['disposition'] for r in records)
print(f"Acceptance ledger: {len(records)} records checked; {counts['passed']} baseline-CI passed; "
      f"{counts['implemented']} current implementation receipts; {counts['outside_gate']} test-only outside gate; "
      f"{counts['unsupported_assertion']} explicitly open")
print(f'Full closure audit: {len(audited)} records; {assertion_count} Rails assertions/predicates; '
      f'{mapped_count} mapped; {len(strengthened)} strengthened closures; {len(reopened)} reopened')
if args.ci_log:
    print('CI receipt verification: 4948 PASS entries; 0 FAIL entries; all cited names/lines match run 37200618245')
if args.nextest_log:
    print(f"Final local run: {receipt['passed']} distinct PASS entries; 0 FAIL entries; "
          f"{receipt['skipped']} skipped; all {sum(r['disposition'] in {'passed', 'implemented'} for r in records)} retained closures have named PASS receipts")
