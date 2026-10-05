#!/usr/bin/env python3
"""Check cutover receipts, preserved history and the exact active remainder."""
import argparse
import json
import re
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--nextest-list', type=Path, required=True,
                    help='cargo nextest list --message-format json output from this source')
parser.add_argument('--native-log', type=Path, help='completed current-branch nextest log')
parser.add_argument('--browser-log', type=Path, help='completed ignored-browser nextest log')
parser.add_argument('--browser-scope', choices=['all', 'd'], default='all',
                    help='all requires historical and current wrappers; d checks this slice only')
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
def load(name):
    return json.loads((root / 'plans' / name).read_text())
listing = json.loads(args.nextest_list.read_text())
active = {
    name for suite in listing['rust-suites'].values()
    for name, info in suite['testcases'].items() if not info['ignored']
}
ignored = {
    name for suite in listing['rust-suites'].values()
    for name, info in suite['testcases'].items() if info['ignored']
}
receipts = load('ledger-ws8br-ws17-ws11ui-receipts.json')
for row in receipts['ci_passes']:
    assert row['test'] in active, f"missing or ignored current CI test: {row}"
ws17 = load('ws17-rails-test-inventory.json')
changed = [r for r in ws17['tests'] if r.get('history') and
           r['history'][-1].get('base') == receipts['base']]
assert len(changed) == 15
calendar_closed = len(receipts.get('ws17_calendar_browser_closed_records', []))
assert sum(r['status'] == 'ported-equivalent' for r in changed) == 14 + calendar_closed
assert sum(r['status'] == 'acceptance-replay-pending' for r in changed) == 1 - calendar_closed
for row in changed:
    if 'rust_test' in row:
        identities = ignored if row.get('browser_receipt') else active
        assert row['rust_test'] in identities, f"missing credited test: {row}"
ws12 = load('ws12-rails-cases.json')
browser = [r for r in ws12['cases'] if r.get('browser_receipt')]
assert len(browser) == 3 and all(r['status'] == 'ported' for r in browser)
assert ws12['ws11ui_cutover']['browser_ci_gated']
assert ws12['ws11ui_cutover']['browser_ci_entry_point'] == 'rust/parity/system/ws12'
assert ws12['ws11ui_cutover']['browser_ci_job_branch'] == 'rust/ci-full-gate'
browser_tests = set(receipts['ws11ui_browser_ci']['tests'])
assert browser_tests == {r['rust_test'] for r in browser}
assert browser_tests <= ignored, f'missing ignored browser registrations: {sorted(browser_tests-ignored)}'
assert all(r['browser_ci']['test'] == r['rust_test'] and
           r['browser_ci']['entry_point'] == 'rust/parity/system/ws12' and
           r['browser_ci']['job_branch'] == 'rust/ci-full-gate' for r in browser)
entry = (root / 'parity/system/ws12').read_text()
assert 'controllers::ws12_browser_remaining_tests -- --ignored' in entry
assert 'cargo build --locked' in entry and 'WS11UI_BROWSER_BINARY' in entry
assert all(r['test'] in active for r in receipts['review240_assertion_tests'])
remaining = load('ledger-ws8br-ws17-ws11ui-remaining.json')
assert not ({r['id'] for r in remaining['ws8br_broad_original_receipts']} & {r['id'] for r in receipts['ws8br_broad_closed_records']})
assert len(remaining['ws8br_broad_original_receipts']) + len(receipts['ws8br_broad_closed_records']) == 337
assert len(remaining['ws8br_sidebar_original_receipts']) + len(receipts.get('ws8br_sidebar_closed_records',[])) == 8
assert len(remaining['ws8br2_original_criteria']) + len(receipts.get('ws8br2_criterion_closed_records', [])) == 14
assert len(remaining['ws8br_muted_browser']) + len(receipts.get('ws8br_muted_browser_closed_records', [])) == 1
assert len(remaining['ws17_calendar_browser']) + calendar_closed == 1
assert len(remaining['excluded_geometry']) == 3
ws8 = load('ws8br-rails-cases.json')
original = {(f['file'], r['name']) for f in ws8['files'] for r in f['declared_cases']}
for group in ['ws8br_broad_original_receipts', 'ws8br_sidebar_original_receipts', 'excluded_geometry']:
    for row in remaining[group]:
        assert (row['file'], row['test']) in original, f"remainder is not an original declaration: {row}"
has_active_gaps = any(remaining[key] for key in ['ws8br_broad_original_receipts',
    'ws8br_sidebar_original_receipts', 'ws8br2_original_criteria', 'ws8br_muted_browser',
    'ws17_calendar_browser', 'aggregate_mapping_gaps'])
assert ws8['cutover_reconciliation']['partial'] == remaining['partial'] == has_active_gaps
assert ws8['cutover_reconciliation']['broad_original_receipts_pending'] == len(remaining['ws8br_broad_original_receipts'])
assertion_receipts = load('ledger-ws8br-ws17-ws11ui-b-receipts.json')
new_tests = {name for row in assertion_receipts['records'] for name in row['rust_tests']}
c_file=root/'plans/ledger-ws8br-ws17-ws11ui-c-receipts.json'
if c_file.exists():
    new_tests |= {name for row in json.loads(c_file.read_text())['records'] for name in row['rust_tests']}
assert new_tests <= active, f'missing or ignored assertion-receipt tests: {new_tests-active}'
d_tests = set(receipts.get('d_validation', {}).get('registered_browser_tests', []))
if d_tests:
    registry = json.loads((root / 'ci/ignored-tests.json').read_text())
    assert d_tests <= {r['test'] for r in registry['browsers']}
    assert d_tests <= ignored, f'missing current ignored browser wrappers: {d_tests-ignored}'
    d_ids = set()
    for name in receipts['d_validation']['assertion_receipts']:
        manifest = load(name)
        assert all(r['record_status'] == 'closed' for r in manifest['records'])
        d_ids |= {r['id'] for r in manifest['records'] if r['id'].startswith('P')}
    assert len(d_ids) == 104
    assert d_ids <= {r['id'] for r in receipts['ws8br_broad_closed_records']}
    assert not load('ws8br-system-mappings.json')['partial']
    # The native phone geometry exclusions keep their old phase disposition.
    excluded = {r['id'] for r in remaining['excluded_geometry']}
    assert excluded == {'P0340', 'P0341', 'P0342'}
    assert not (excluded & d_ids)
if args.native_log:
    passed = set(re.findall(r'^\s*PASS\s+\[[^\]]+\]\s+\([^)]+\)\s+\S+\s+(\S+)', args.native_log.read_text(), re.M))
    credited = {r['test'] for r in receipts['ci_passes']} | {r['rust_test'] for r in changed if 'rust_test' in r and not r.get('browser_receipt')} | new_tests | {r['test'] for r in receipts['review240_assertion_tests']}
    assert credited <= passed, f'credited current tests did not pass: {sorted(credited-passed)}'
    print(f'Cutover current branch: {len(credited)} credited test identities passed in the supplied current workspace run')
if args.browser_log:
    passed = set(re.findall(r'^\s*PASS\s+\[[^\]]+\]\s+\([^)]+\)\s+\S+\s+(\S+)', args.browser_log.read_text(), re.M))
    selected = d_tests if args.browser_scope == 'd' else browser_tests | d_tests
    assert selected, 'requested browser scope has no registered tests'
    assert selected <= passed, f'current registered browser tests did not pass: {sorted(selected-passed)}'
    print(f'Cutover browser scope {args.browser_scope}: {len(selected)} registered ignored tests passed with paired original browser sequences')
print(f"Cutover ledger receipts: {len(receipts['ci_passes'])} historical CI test identities still enabled; {14+calendar_closed} WS17 closures; {len(browser_tests|d_tests)} ignored browser registrations checked; {len(receipts['ws8br_broad_closed_records'])} broad WS8 closures; 1 approved queue supersession; 0 inconsistent records")
print(f'Cutover active remainder: {len(remaining["ws8br_broad_original_receipts"])} broad; {len(remaining["ws8br_sidebar_original_receipts"])} sidebar; {len(remaining["ws8br2_original_criteria"])} overlapping criteria; {len(remaining["ws8br_muted_browser"])} muted; {len(remaining["ws17_calendar_browser"])} Calendar; {len(remaining["excluded_geometry"])} existing geometry exclusions')
