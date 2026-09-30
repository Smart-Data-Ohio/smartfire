#!/usr/bin/env python3
"""Reproduce the ten executed review failures against d336ca78, then restore fixes."""
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
BASE = "d336ca787be68c73f1e82e342fef25051ed1ad48"
FILES = [
    "rust/crates/db/src/models/user.rs",
    "rust/crates/campfire/src/authentication.rs",
    "rust/crates/campfire/src/controllers/users/profiles.rs",
    "rust/crates/campfire/src/controllers/sessions.rs",
    "rust/crates/campfire/src/controllers/accounts/users.rs",
    "rust/crates/campfire/src/controllers/users/bans.rs",
    "rust/crates/campfire/src/concerns/session_keys.rs",
]
OUT = ROOT / ".scratch/auth/round-three-proof"
OUT.mkdir(parents=True, exist_ok=True)
(ROOT / ".scratch/tmp").mkdir(parents=True, exist_ok=True)
fixed = {name: (ROOT / name).read_bytes() for name in FILES}
env = os.environ.copy()
env.update(CI="1", CABLE_TEST_PORT_RANGE="51600-51699", TMPDIR=str(ROOT / ".scratch/tmp"))
try:
    for name in FILES:
        (ROOT / name).write_bytes(subprocess.check_output(["git", "show", f"{BASE}:{name}"], cwd=ROOT))
    result = subprocess.run([
        "cargo", "test", "--manifest-path", "rust/Cargo.toml", "--locked", "-j", "4", "-p", "campfire",
        "app::round_three_security_tests", "--", "--test-threads=4",
    ], cwd=ROOT, env=env, capture_output=True, text=True)
    output = result.stdout + result.stderr
    (OUT / "native-red.log").write_text(output)
    summary = re.search(r"test result: FAILED\. 0 passed; 10 failed; 0 ignored;[^\n]*", output)
    assert result.returncode != 0 and summary, f"Expected ten executed failures, not a compile error: {output[-4000:]}"
    print(f"Against {BASE}: {summary.group()}")
finally:
    for name, content in fixed.items():
        (ROOT / name).write_bytes(content)
    print(f"Restored all {len(FILES)} fixed production files")
