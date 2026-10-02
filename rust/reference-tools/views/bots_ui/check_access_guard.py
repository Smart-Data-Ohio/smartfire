#!/usr/bin/env python3
"""Prove the HTTP access test rejects a missing owner/admin check, then restore it."""
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[3]
source = root / "crates/campfire/src/controllers/accounts/bots.rs"
original = source.read_text()
needle = "if manages {"
assert original.count(needle) == 1, "management guard changed; review the injection"
try:
    source.write_text(original.replace(needle, "if true || manages {"))
    result = subprocess.run(
        ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked",
         "-p", "campfire", "bot_edit_pages_follow_admin_owner_and_legacy_access", "--", "--nocapture", "--test-threads=8"],
        cwd=root, env={**os.environ, "CI": "1", "CARGO_BUILD_JOBS": "2"}, capture_output=True, text=True,
    )
    output = result.stdout + result.stderr
    print(output, end="")
    assert result.returncode != 0 and "left: 200" in output and "right: 403" in output and "test result: FAILED." in output, "access test did not reject the missing guard"
    print("Access-guard injection: rejected (member received 200 instead of 403); source restored")
finally:
    source.write_text(original)
