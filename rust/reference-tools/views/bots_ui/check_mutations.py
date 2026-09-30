#!/usr/bin/env python3
"""Require seeded HTTP tests to reject missing sudo, owner and no-store gates."""
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[3]
source = root / "crates/campfire/src/controllers/accounts/bots.rs"
original = source.read_text()
cases = [
    ("sudo", "concerns::require_sudo_mode(c)?;", "", "create_requires_administrator_and_sudo"),
    ("owner-webhook", "concerns::ensure_can_administer(c)?;", "", "owner_cannot_change_webhook_but_can_submit_unchanged"),
    ("key-cache", 'c.set_header("cache-control", "no-store");', "", "create_reveals_key_once_creates_workspace_agent_and_audits"),
    ("owner-update", "if manages {", "if false && manages {", "owner_updates_profile_without_sudo_and_keeps_webhook"),
]
try:
    for label, needle, replacement, test in cases:
        assert needle in original, f"{label}: source changed; review injection"
        source.write_text(original.replace(needle, replacement))
        result = subprocess.run(
            ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j", "4",
             "-p", "campfire", test, "--", "--nocapture"],
            cwd=root, env={**os.environ, "CI": "1"}, capture_output=True, text=True,
        )
        output = result.stdout + result.stderr
        assert result.returncode != 0 and "test result: FAILED." in output and "panicked at" in output, f"{label}: mutation escaped HTTP test\n{output}"
        print(label + ": " + next(line for line in output.splitlines() if line.startswith("test result:")))
        source.write_text(original)
    print("Bot mutation discrimination: 4 regressions detected; source restored")
finally:
    source.write_text(original)
