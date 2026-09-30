#!/usr/bin/env python3
"""Prove the WS17 regressions discriminate, restoring source after each injected defect.

Run from the worktree root. Scratch output and TMPDIR stay under .scratch.
"""
import os
from pathlib import Path
import re
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root / ".scratch"
scratch.mkdir(exist_ok=True)
env = dict(os.environ, TMPDIR=str(scratch), CARGO_TARGET_DIR=str(root / "rust/target"))
cases = [
    ("expired-status", "user_status_settings.rs",
     "self.custom_status_expires_at.is_some_and(|until| until <= now)",
     "self.custom_status_expires_at.is_some_and(|_| false)",
     "ws17_status_readers_match_rails_times_zones_and_dst"),
    ("prune-on-read", "workspace_presence_lease.rs",
     "let rows: Vec<(i64, Option<Timestamp>)>",
     'conn.execute("DELETE FROM workspace_presence_leases WHERE expires_at < ?", [now])?;\n        let rows: Vec<(i64, Option<Timestamp>)>',
     "ws17_expiry_is_inclusive_and_reads_never_prune"),
]
for name, filename, original, broken, test in cases:
    source = root / "rust/crates/db/src/models" / filename
    text = source.read_text()
    # Formatting may place a newline after the receiver in the expiry guard.
    if original not in text and name == "expired-status":
        match = re.search(r"self\s*\.custom_status_expires_at\s*\.is_some_and\(\|until\| until <= now\)", text)
        if match:
            original = match.group()
    assert original in text, f"injection anchor missing: {name}"
    log = scratch / f"{name}-injected.log"
    try:
        source.write_text(text.replace(original, broken, 1))
        with log.open("w") as output:
            result = subprocess.run([
                "mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j", "4",
                "--manifest-path", "rust/Cargo.toml", "-p", "campfire_db", test, "--", "--test-threads=4",
            ], cwd=root, env=env, stdout=output, stderr=subprocess.STDOUT, check=False)
    finally:
        source.write_text(text)
    summary = re.search(r"^test result: FAILED\..*$", log.read_text(), re.MULTILINE)
    assert result.returncode != 0 and summary, f"injected defect escaped (or did not compile): {name}"
    print(f"{name}: detected")
    print(summary.group())
