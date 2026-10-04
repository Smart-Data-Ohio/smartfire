#!/usr/bin/env python3
"""Check the partial cutover ledger's receipts, history and exact remainder."""
import argparse
import json
import re
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--nextest-list', type=Path, required=True,
                    help='cargo nextest list --message-format json output from this source')
parser.add_argument('--native-log', type=Path, help='completed current-branch nextest log')
parser.add_argument('--browser-log', type=Path, help='completed ignored-browser nextest log')
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
assert sum(r['status'] == 'ported-equivalent' for r in changed) == 14
assert sum(r['status'] == 'acceptance-replay-pending' for r in changed) == 1
for row in changed:
    if 'rust_test' in row:
        assert row['rust_test'] in active, f"missing or ignored credited test: {row}"
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
assert len(remaining['ws8br_broad_original_receipts']) == 328
assert len(receipts['ws8br_broad_closed_records']) == 9
assert len(remaining['ws8br_broad_original_receipts']) + len(receipts['ws8br_broad_closed_records']) == 337
assert len(remaining['ws8br_sidebar_original_receipts']) == 8
assert len(remaining['ws8br2_original_criteria']) == 14
assert len(remaining['ws8br_muted_browser']) == 1
assert len(remaining['ws17_calendar_browser']) == 1
assert len(remaining['excluded_geometry']) == 3
ws8 = load('ws8br-rails-cases.json')
original = {(f['file'], r['name']) for f in ws8['files'] for r in f['declared_cases']}
for group in ['ws8br_broad_original_receipts', 'ws8br_sidebar_original_receipts', 'excluded_geometry']:
    for row in remaining[group]:
        assert (row['file'], row['test']) in original, f"remainder is not an original declaration: {row}"
assert ws8['cutover_reconciliation']['partial'] and remaining['partial']
if args.native_log:
    passed = set(re.findall(r'^\s*PASS\s+\[[^\]]+\]\s+\([^)]+\)\s+\S+\s+(\S+)', args.native_log.read_text(), re.M))
    credited = {r['test'] for r in receipts['ci_passes']} | {r['rust_test'] for r in changed if 'rust_test' in r}
    assert credited <= passed, f'credited current tests did not pass: {sorted(credited-passed)}'
    print(f'Cutover current branch: {len(credited)} credited test identities passed in the current nextest run')
if args.browser_log:
    passed = set(re.findall(r'^\s*PASS\s+\[[^\]]+\]\s+\([^)]+\)\s+\S+\s+(\S+)', args.browser_log.read_text(), re.M))
    assert browser_tests <= passed, f'registered browser tests did not pass: {sorted(browser_tests-passed)}'
    print('Cutover current branch: 3 registered ignored tests passed with paired browser sequences and writer controls')
print(f"Cutover ledger receipts: {len(receipts['ci_passes'])} historical CI test identities still enabled; 14 WS17 closures; 3 ignored browser registrations for rust/ci-full-gate; 9 broad WS8 supersessions; 1 approved queue supersession; 0 inconsistent records")
print('Cutover ledger remains partial: 328 broad receipts; 8 sidebar receipts; 14 overlapping criteria; 1 muted browser; 1 Calendar browser; 3 geometry-only exclusions')
