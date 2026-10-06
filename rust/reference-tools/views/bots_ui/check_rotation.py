#!/usr/bin/env python3
"""Require real key retirement and one-time no-store rendering, then restore sources."""
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[3]
domain = root / "crates/db/src/models/user.rs"
controller = root / "crates/campfire/src/controllers/accounts/bots/keys.rs"
cases = [
    ("retired-digest", domain, "params![digest, now, self.id]",
     "params![self.bot_token_digest, now, self.id]"),
    ("no-store", controller, 'c.set_header("cache-control", "no-store");',
     'c.set_header("cache-control", "public");'),
]
originals = {path: path.read_text() for _, path, *_ in cases}
try:
    for name, path, needle, replacement in cases:
        original = originals[path]
        assert original.count(needle) == 1, f"{name}: review changed rotation seam"
        path.write_text(original.replace(needle, replacement))
        result = subprocess.run(
            ["cargo", "test", "--locked",
             "-p", "campfire", "ws11_key_rotation_requires_sudo_and_shows_the_key_once", "--", "--nocapture", "--test-threads=8"],
            cwd=root, env={**os.environ, "CI": "1", "CARGO_BUILD_JOBS": "2"}, capture_output=True, text=True,
        )
        output = result.stdout + result.stderr
        assert result.returncode != 0 and "test result: FAILED." in output and "panicked at" in output, f"{name}: regression escaped HTTP test\n{output}"
        print(name + ": " + next(line for line in output.splitlines() if line.startswith("test result:")), flush=True)
        path.write_text(original)
    print(f"Key rotation discrimination: {len(cases)} regressions rejected; sources restored")
finally:
    for path, original in originals.items():
        path.write_text(original)
