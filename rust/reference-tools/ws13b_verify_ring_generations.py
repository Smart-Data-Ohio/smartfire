#!/usr/bin/env python3
"""Verify the independently regenerated Rails invitation-generation corpus."""

from status_pin_identity import PIN, PIN_FULL, PIN_IMAGE
import json
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[2]
actual = json.loads(Path(sys.argv[1]).read_text())
expected = json.loads((root / "rust/crates/db/src/tests/huddle_ring_generations.json").read_text())
assert actual == expected, "invitation generation corpus differs from pinned Rails"
assert actual["reference_pin"] == PIN
assert len(actual["cases"]) == 6
for case in actual["cases"]:
    initial, _, retry = case["phases"]
    assert initial["invited_at"] != retry["invited_at"]
    assert len(retry["frames"]) == 1
    assert retry["frames"][0]["huddleInvitation"]["state"] == "unread"
print(f'WS13b ring generations: 6 retry sequences match {PIN}; one unread retry frame each')
