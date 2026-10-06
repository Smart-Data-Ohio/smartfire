#!/usr/bin/env python3
"""All eight real-policy ring assertions must detect a publication regression."""
import os
from pathlib import Path
import re
import subprocess
ROOT = Path(__file__).resolve().parents[2]
path = ROOT / "rust/crates/db/src/models/huddle_invitations.rs"
original = path.read_text()
before = 'invitation["silent"] = serde_json::Value::Bool(!sound_allowed);'
after = 'invitation["silent"] = serde_json::Value::Bool(sound_allowed);'
environment = dict(os.environ, CARGO_BUILD_JOBS="2", TMPDIR=str(ROOT / ".scratch"), CARGO_TARGET_DIR=str(ROOT / "rust/target"), CI="1", CABLE_TEST_PORT_RANGE="53000-53049", MAIL_TEST_PORT_RANGE="53050-53099")
try:
    assert original.count(before) == 1
    path.write_text(original.replace(before, after, 1))
    result = subprocess.run(["cargo", "test", "--locked", "--manifest-path", str(ROOT / "rust/Cargo.toml"), "-p", "campfire_db", "huddle_ring_policy_seam_test", "--", "--test-threads=8"], cwd=ROOT, env=environment, capture_output=True, text=True)
    output = result.stdout + result.stderr
    (ROOT / ".scratch/ring-discrimination-detail.log").write_text(output)
    summary = next((s for s in re.findall(r"^test result: FAILED\..*$", output, re.M) if "8 failed;" in s), None)
    assert result.returncode != 0 and summary and "could not compile" not in output and "panicked at" in output, output[-5000:]
    print(f"ring-sound-inverted: {summary}")
finally:
    path.write_text(original)
print("WS13b ring discrimination: 1 compiled regression detected across all 8 real-policy declarations; source restored")
