#!/usr/bin/env python3
"""Compare eight freshly generated pinned Rails corpora to committed JSON.

Generate each script into DIRECTORY/<script stem>.json using parity/bin/reference.
"""
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
assert len(sys.argv) == 2, "usage: ws13b_verify_vectors.py GENERATED_DIRECTORY"
generated = Path(sys.argv[1])
corpora = [
    ("huddle_revocation", "huddle_revocation_vectors", 9, None),
    ("huddle_grant_sequences", "huddle_grant_sequence_vectors", 13, 54),
    ("huddle_invitation_sequences", "huddle_invitation_sequence_vectors", 38, 136),
    ("huddle_notifier_sequences", "huddle_notifier_sequence_vectors", 33, 172),
    ("huddle_domain_lifecycle_sequences", "huddle_domain_lifecycle_sequence_vectors", 20, 57),
    ("huddle_membership_creation", "huddle_membership_creation_vectors", 6, None),
    ("huddle_join_push_sequences", "huddle_join_push_sequence_vectors", 13, 31),
]
for script, filename, count, steps in corpora:
    expected = json.loads((ROOT / "rust/crates/db/src/tests" / f"{filename}.json").read_text())
    actual = json.loads((generated / f"{script}.json").read_text())
    assert actual["reference_pin"] == "d7c7de92", script
    assert len(actual["cases"]) == count, script
    if steps is not None:
        assert sum(len(case["results"]) for case in actual["cases"]) == steps, script
    assert actual == expected, script
    print(f"{script}: {count} cases; regenerated JSON matches")
actual = json.loads((generated / "huddle_job_contracts.json").read_text())
expected = json.loads((ROOT / "rust/crates/campfire/src/huddle/huddle_job_contract_vectors.json").read_text())
assert actual["reference_pin"] == "d7c7de92"
assert len(actual["invitations"]) == 4
assert len(actual["presence"]["counts"]) == 3 and actual["presence"]["missing"] == []
assert len(actual["reconciler"]) == 5
assert actual == expected, "huddle_job_contracts"
print("huddle_job_contracts: 4 invitation jobs; 3 presence streams; 5 reconciler cases; regenerated JSON matches")
print("WS13b corpora: all 8 regenerated corpora match d7c7de92")
