#!/usr/bin/env python3
"""Render every original assertion, including reopened gaps, for human review."""
import json
from pathlib import Path
root = Path(__file__).resolve().parents[1]
manifest = json.loads((root / 'plans/ledger-ws8br-ws17-ws11ui-b-receipts.json').read_text())
lines = ['# PR #244 per-assertion audit', '',
         f"Original declarations and runtime fixtures use Rails {manifest['reference']}.", '',
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
    lines += ['']
output = root / 'plans/ledger-ws8br-ws17-ws11ui-b-assertions.md'
output.write_text('\n'.join(lines) + '\n')
print(f'Assertion mapping: {len(manifest["records"])} declarations; {sum(len(r["assertions"]) for r in manifest["records"])} explicit rows')
