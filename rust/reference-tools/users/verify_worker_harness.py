#!/usr/bin/env python3
"""Keep the original Rails event assertions; change only how worker bytes are loaded."""
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[2]
reference = (root / "parity/reference.sha").read_text().strip()
original = subprocess.check_output([
    "git", "show", f"{reference}:test/scripts/service_worker_harness.mjs"
], cwd=root)
before = b'const source = readFileSync(new URL("../../app/views/pwa/service_worker.js", import.meta.url), "utf8")'
after = b'// The browser driver supplies the unmodified HTTP response body on stdin.\nconst source = readFileSync(0, "utf8")'
assert original.count(before) == 1
assert original.replace(before, after) == (root / "reference-tools/users/service_worker_harness.mjs").read_bytes()
print("WS8br2 worker harness provenance: pinned Rails assertions unchanged; only stdin source loading differs")
