#!/usr/bin/env python3
"""Prove the HTTP directory test rejects Rust's contextual final-sigma ordering."""
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[3]
source = root / "crates/db/src/models/agent_profile.rs"
original = source.read_text()
assert original.count("ruby_downcase(&record.user.name)") == 1
try:
    source.write_text(original.replace("ruby_downcase(&record.user.name)", "record.user.name.to_lowercase()"))
    result = subprocess.run(
        ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j", "4",
         "-p", "campfire", "agent_directory_lists_active_then_inactive_without_private_facts",
         "--", "--nocapture"],
        cwd=root, env={**os.environ, "CI": "1"}, capture_output=True, text=True,
    )
    output = result.stdout + result.stderr
    print(output, end="")
    assert result.returncode != 0 and "Ruby lowercases Σ without Rust's final-sigma context rule" in output and "test result: FAILED." in output, "directory test did not reject incorrect Unicode ordering"
    print("Directory downcase injection: rejected (Greek final-sigma order differed); source restored")
finally:
    source.write_text(original)
