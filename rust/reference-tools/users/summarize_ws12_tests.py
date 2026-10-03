#!/usr/bin/env python3
"""Print raw aggregate cargo receipts without counting missing-seed skips as parity coverage."""
from pathlib import Path
import re
import sys

output = Path(sys.argv[1]).read_text()
rows = re.findall(r"^test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;.*$", output, re.M)
assert rows, "no cargo summaries"
print(f"WS12 workspace totals: {sum(int(r[0]) for r in rows)} passed; {sum(int(r[1]) for r in rows)} failed; {sum(int(r[2]) for r in rows)} ignored; {len(rows)} result summaries")
skips = len(re.findall(r"^skipping locally: parity/\.seed/", output, re.M))
intentional = int("test controllers::presenters::test_support::missing_seed_may_skip_locally ... ok" in output)
assert skips >= intentional
print(f"WS12 seed skips: {skips - intentional} actual; {intentional} intentional missing-seed unit notice")
