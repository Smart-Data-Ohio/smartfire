#!/usr/bin/env python3
"""Run every behavioral receipt in template-coverage.json, plus the whole views suite."""
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
coverage = json.loads((ROOT / 'parity/template-coverage.json').read_text())
names = sorted({receipt['test'] for receipt in coverage['evidence'].values()
                if not receipt['test_file'].startswith('rust/crates/views/')})
expression = 'package(campfire_views) | ' + ' | '.join(f'test({name})' for name in names)
command = ['cargo', 'nextest', 'run', '--locked', '-p', 'campfire_views', '-p', 'campfire',
           '-p', 'campfire_mail', '-p', 'campfire_richtext', '-j', '4', '-E', expression]
print(f'Template coverage: {len(coverage["evidence"])} receipts; all views tests; four test workers', flush=True)
raise SystemExit(subprocess.call(command, cwd=ROOT))
