#!/usr/bin/env python3
"""Count raw cargo summaries and explicit skip notices without treating ignored tests as run."""
from pathlib import Path
import re
import sys

text = Path(sys.argv[1]).read_text()
rows = re.findall(r'^test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;.*$', text, re.M)
if not rows:
    raise SystemExit('no test result summaries')
print(f'WS11 workspace totals: {sum(int(row[0]) for row in rows)} passed; {sum(int(row[1]) for row in rows)} failed; {sum(int(row[2]) for row in rows)} ignored; {len(rows)} result summaries')
print(f'WS11 missing-seed skips: {len(re.findall(r"(?:skipping: parity|SKIPPED)", text))}')
