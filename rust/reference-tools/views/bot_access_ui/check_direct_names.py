#!/usr/bin/env python3
"""Prove the pinned direct-room name oracle rejects formatting regressions."""
from pathlib import Path
import os
import subprocess

root = Path(__file__).resolve().parents[3]
source = root / "crates/views/src/rooms.rs"
original = source.read_text()
cases = [("separator", '.join(", ")', '.join(" and ")'),
         ("remainder", "names.len().saturating_sub(3)", "names.len().saturating_sub(2)")]
try:
    for name, needle, replacement in cases:
        assert original.count(needle) == 1, f"{name}: source changed; review injection"
        source.write_text(original.replace(needle, replacement))
        result = subprocess.run(
            ["cargo", "test", "--locked",
             "-p", "campfire_views", "--test", "core",
             "bot_access_pages_match_pinned_rails_bytes", "--", "--nocapture", "--test-threads=8"],
            cwd=root, env=os.environ.copy(), capture_output=True, text=True)
        output = result.stdout + result.stderr
        assert result.returncode != 0 and "test result: FAILED." in output and "panicked at" in output, f"{name}: mutation escaped oracle\n{output}"
        print(name + ": " + next(line for line in output.splitlines() if line.startswith("test result:")), flush=True)
        source.write_text(original)
    print("Direct-room name discrimination: 2 regressions detected; source restored")
finally:
    source.write_text(original)
