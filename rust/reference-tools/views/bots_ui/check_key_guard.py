#!/usr/bin/env python3
"""Require the Rails member/owner key-reset denial through the real HTTP stack."""
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[3]
source = root / "crates/campfire/src/controllers/accounts/bots/keys.rs"
original = source.read_text()
needle = "    concerns::ensure_can_administer(c)?;\n"
assert original.count(needle) == 1, "key guard changed; review injection"
try:
    source.write_text(original.replace(needle, ""))
    result = subprocess.run(
        ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked",
         "-p", "campfire", "members_and_bot_owners_cannot_reset_keys_even_with_sudo", "--", "--nocapture", "--test-threads=8"],
        cwd=root, env={**os.environ, "CI": "1", "CARGO_BUILD_JOBS": "2"}, capture_output=True, text=True,
    )
    output = result.stdout + result.stderr
    assert result.returncode != 0 and "test result: FAILED." in output and "member key reset:" in output, f"key guard regression escaped HTTP test\n{output}"
    print(next(line for line in output.splitlines() if line.startswith("test result:")))
    print("Key-reset guard discrimination: missing administrator gate rejected; source restored")
finally:
    source.write_text(original)
