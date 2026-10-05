#!/usr/bin/env python3
"""Select the merged/credited native tests and scope-wrapper lease test, never the full gate."""
import json
from pathlib import Path
import sys
root=Path(__file__).resolve().parents[3]
listing=json.loads(Path(sys.argv[1]).read_text())
active={n for suite in listing['rust-suites'].values() for n,case in suite['testcases'].items() if not case['ignored']}
names={name for suffix in ['b-receipts','c-receipts'] for row in json.loads((root/f'rust/plans/ledger-ws8br-ws17-ws11ui-{suffix}.json').read_text())['records'] for name in row['rust_tests']}
prefixes=['controllers::public_pages::tests::','controllers::users::profile_sections_tests::','controllers::users::profile_settings_tests::','controllers::users::people_tests::','controllers::users::joining_tests::','controllers::users::layout_preferences_tests::','app::profile_security_tests::']
names|={n for n in active if any(n.startswith(p) for p in prefixes)}
names.add('controllers::ws11ui_original_browser_tests::original_browser_port_leases_are_host_coordinated_and_ephemeral_safe')
assert names<=active,sorted(names-active)
print(' or '.join('test(='+n+')' for n in sorted(names)))
print(f'Targeted merged/touched/receipt identities: {len(names)}',file=sys.stderr)
