#!/usr/bin/env python3
"""Emit only #244 touched and receipt-credited nextest identities."""
import json
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[3]
plans = root / 'rust/plans'
receipts = json.loads((plans / 'ledger-ws8br-ws17-ws11ui-receipts.json').read_text())
mapping = json.loads((plans / 'ledger-ws8br-ws17-ws11ui-b-receipts.json').read_text())
ws17 = json.loads((plans / 'ws17-rails-test-inventory.json').read_text())
listing = json.loads(Path(sys.argv[1]).read_text())
active = {name for suite in listing['rust-suites'].values()
          for name, info in suite['testcases'].items() if not info['ignored']}
names = {r['test'] for r in receipts['ci_passes']} | {r['test'] for r in receipts['review240_assertion_tests']}
names |= {name for row in mapping['records'] for name in row['rust_tests']}
names |= {r['rust_test'] for r in ws17['tests'] if r.get('history') and
          r['history'][-1].get('base') == receipts['base'] and 'rust_test' in r}
prefixes = ['controllers::public_pages::tests::', 'controllers::users::profile_sections_tests::',
            'controllers::users::profile_settings_tests::', 'controllers::users::people_tests::',
            'controllers::users::joining_tests::', 'controllers::users::layout_preferences_tests::']
names |= {n for n in active if any(n.startswith(p) for p in prefixes)}
assert names <= active, sorted(names - active)
print(' or '.join('test(=' + name + ')' for name in sorted(names)))
print(f'Targeted receipt/touched-test identities: {len(names)}', file=sys.stderr)
