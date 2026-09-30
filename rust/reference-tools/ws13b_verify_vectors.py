#!/usr/bin/env python3
"""Compare freshly generated Rails JSON to the committed WS13b corpora."""
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
assert len(sys.argv) == 3, "usage: ws13b_verify_vectors.py REVOCATION_JSON SEQUENCE_JSON"
for filename, generated, count in [
    ("huddle_revocation_vectors.json", sys.argv[1], 9),
    ("huddle_grant_sequence_vectors.json", sys.argv[2], 13),
]:
    expected = json.loads((ROOT / "rust/crates/db/src/tests" / filename).read_text())
    actual = json.loads(Path(generated).read_text())
    assert actual["reference_pin"] == "d7c7de92"
    assert len(actual["cases"]) == count
    assert actual == expected, filename
sequences = json.loads(Path(sys.argv[2]).read_text())
assert sum(len(case["results"]) for case in sequences["cases"]) == 54
print("WS13b corpora: 9 revocation cases; 13 grant sequences; 54 intermediate results; regenerated JSON matches")
