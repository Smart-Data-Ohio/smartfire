#!/usr/bin/env python3
"""Raw cargo counts, distinguishing deliberately missing-seed unit tests."""
from pathlib import Path
import re
import sys

output = Path(sys.argv[1]).read_text()
rows = re.findall(r"^test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;.*$", output, re.M)
assert rows, "no cargo summaries"
print(f"WS11-api cargo totals: {sum(int(r[0]) for r in rows)} passed; {sum(int(r[1]) for r in rows)} failed; {sum(int(r[2]) for r in rows)} ignored; {len(rows)} result summaries")
skips = len(re.findall(r"^skipping locally: parity/\.seed/", output, re.M))
expected = int("test controllers::presenters::test_support::missing_seed_may_skip_locally ... ok" in output)
assert skips >= expected
print(f"WS11-api seed skips: {skips - expected} actual; {expected} intentional missing-seed unit notice")
