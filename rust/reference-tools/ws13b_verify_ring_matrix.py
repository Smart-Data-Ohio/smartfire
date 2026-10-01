#!/usr/bin/env python3
"""Compare the full Rails-generated oracle, including its declared projection."""
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parents[2]
expected = json.loads((root / "rust/crates/campfire/src/jobs/huddle/ring_matrix.json").read_text())
actual = json.loads(Path(sys.argv[1]).read_text())

def compare(value):
    assert value == expected, "Rails ring sequence matrix differs"

compare(actual)
assert actual["reference_pin"] == "d7c7de92"
for path, digest in actual["source_sha256"].items():
    pinned = subprocess.check_output(["git", "show", f"d7c7de92:{path}"], cwd=root)
    assert hashlib.sha256(pinned).hexdigest() == digest, path
cases = actual["cases"]
assert len(cases) == 180
assert len({case["spec"]["name"] for case in cases}) == 180
assert sum(case["spec"]["banner"] for case in cases) == 78
assert sum(case["spec"]["newest_first"] for case in cases) == 90
assert {step["seconds"] for case in cases for step in case["spec"]["steps"]} >= {0, 1, 120, 121, 181}
for case in cases:
    assert len(case["spec"]["steps"]) == len(case["phases"])
# Discriminating comparison check: losing the pending deduped banner, duplicating
# a retry, or changing room metadata must each make the complete comparison fail.
mutations = []
for name, mode in [("pending_initial_reissue_1/banner/oldest", "lose"),
                   ("handled_retry_181/item/oldest", "duplicate"),
                   ("delayed_initial_0/item/oldest", "metadata")]:
    changed = copy.deepcopy(actual)
    case = next(case for case in changed["cases"] if case["spec"]["name"] == name)
    frames = case["phases"][-1]["delivered"]
    assert len(frames) == 1
    if mode == "lose":
        frames.clear()
    elif mode == "duplicate":
        frames.append(copy.deepcopy(frames[0]))
    else:
        frames[0]["huddleInvitation"]["roomName"] = "wrong room"
    try:
        compare(changed)
    except AssertionError:
        mutations.append(mode)
    else:
        raise AssertionError(f"comparison accepted {mode} mutation")
print("WS13b Rails ring matrix: 180 sequences match d7c7de92; 78 banner, 102 item; both drain orders")
print(f"WS13b ring matrix discrimination: {len(mutations)} injected mismatches rejected")
