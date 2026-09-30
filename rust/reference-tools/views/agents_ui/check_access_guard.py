#!/usr/bin/env python3
"""Prove the directory's token and human-only policy is enforced by HTTP tests."""
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[3]
source = root / "crates/campfire/src/controllers/agents/directory.rs"
original = source.read_text()
assert original.count("Before::default()") == 1
assert original.count("if require_current_user(c)?.is_bot()") == 1
try:
    source.write_text(original.replace("Before::default()", "Before::default().allow_agent_access().allow_bot_access()")
                     .replace("if require_current_user(c)?.is_bot()", "if false && require_current_user(c)?.is_bot()"))
    result = subprocess.run(
        ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j", "4",
         "-p", "campfire", "agent_directory_", "--", "--nocapture"],
        cwd=root, env={**os.environ, "CI": "1"}, capture_output=True, text=True,
    )
    output = result.stdout + result.stderr
    print(output, end="")
    assert result.returncode != 0 and "left: 200" in output and "right: 403" in output and "test result: FAILED." in output, "directory tests did not reject bot/token access"
    print("Directory access injection: rejected (bot/token received 200 instead of 403); source restored")
finally:
    source.write_text(original)
