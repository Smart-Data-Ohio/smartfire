#!/usr/bin/env python3
"""Compare independently replayed review probes against the committed Rails oracle."""
import json
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[2]
actual = json.loads(Path(sys.argv[1]).read_text())
expected = json.loads((root / "rust/crates/db/src/tests/ws13b_review_fixes.json").read_text())
assert actual == expected, "review probe results differ from pinned Rails fixture"
assert actual["reference_pin"] == "d7c7de92"
print("WS13b review probes: 7 ring sequences, 3 post-commit failures, 22 timestamps, 3 rejoin boundaries match d7c7de92")
