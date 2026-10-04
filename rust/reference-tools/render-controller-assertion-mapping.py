#!/usr/bin/env python3
"""Render every original assertion, including reopened gaps, for human review."""
import argparse
import json
from pathlib import Path
root = Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--manifest',type=Path,default=root/'plans/ledger-ws8br-ws17-ws11ui-b-receipts.json')
parser.add_argument('--output',type=Path,default=root/'plans/ledger-ws8br-ws17-ws11ui-b-assertions.md')
args=parser.parse_args()
manifest = json.loads(args.manifest.read_text())
lines = ['# Individual original assertion audit', '',
         'Original declarations use Rails d7c7de92. Runtime fixtures follow the current reference pin.', '',
         'Every row names the original assertion and the actual Rust check. Whole-byte/DOM checks retain tags, attributes, text and cardinality; their fixture cases are named below. Reopened gaps are explicit and retain the previous insufficient claim in JSON history.', '']
for row in manifest['records']:
    lines += [f"## {row['id']}: {row['test']}", '', f"Status: **{row['record_status']}**. Native identities:", '']
    lines += [f"- `{name}`" for name in row['rust_tests']]
    lines += ['', '| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |', '| --- | --- | --- |']
    for assertion in row['assertions']:
        ruby = assertion['ruby'].replace('|', '&#124;').replace('`', '&#96;')
        pointer = assertion.get('assertion_source', '**OPEN**')
        observation = assertion['checks'].replace('|', '&#124;')
        cases = ', '.join(assertion.get('oracle_cases', []))
        if cases:
            observation += f" Cases: {cases}."
        additional = ', '.join(p['path'] for p in assertion.get('additional_assertion_sources', []))
        if additional:
            pointer += '; ' + additional
        lines += [f"| `{row['file']}:{assertion['line']}` — `{ruby}` | {pointer} | {observation} |"]
    for helper in row.get('helper_expansion', []):
        lines += [f"| `{helper['ruby_file']}:{helper['line']}` — `{helper['ruby']}` (private helper) | {helper['rust_assertion']['path']} | The calling original case executes this routed-response assertion. |"]
    lines += ['']
output = args.output
output.write_text('\n'.join(lines) + '\n')
print(f'Assertion mapping: {len(manifest["records"])} declarations; {sum(len(r["assertions"]) for r in manifest["records"])} explicit rows')
