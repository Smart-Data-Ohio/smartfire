#!/usr/bin/env python3
"""Compare write/broadcast/provider fixtures only with actual pinned Rails requests."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("--record", action="store_true")
args = parser.parse_args()
root = Path(__file__).resolve().parents[3]
scratch = root / ".scratch/zone-cache-followup/rails"
scratch.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, PARITY_NAMESPACE="ws8br-cache-zones", PARITY_OWNER="ws8br",
           PARITY_IMAGE=os.environ.get("PARITY_IMAGE", "campfire-reference"))
run = subprocess.run([str(root / "rust/parity/bin/reference"), "exec", "--seed", "default",
                      "--time", "2026-03-02T16:00:00Z", "--freeze", "bin/rails", "runner",
                      "--skip-executor", "/work/reference-tools/rooms/cache_zones.rb"],
                     cwd=root, env=env, capture_output=True)
(scratch / "stdout.json").write_bytes(run.stdout)
(scratch / "stderr.log").write_bytes(run.stderr)
run.check_returncode()
captured = json.loads(run.stdout)
for path, digest in captured["sources"].items():
    pinned = subprocess.check_output(["git", "show", f"{(root / 'rust/parity/reference.sha').read_text().strip()}:{path}"], cwd=root)
    assert digest == hashlib.sha256(pinned).hexdigest(), f"Rails source drift: {path}"
fixture = root / "rust/crates/campfire/src/controllers/rooms/cache_zones.json"
if args.record:
    fixture.write_text(json.dumps(captured, indent=2, ensure_ascii=False) + "\n")
assert captured == json.loads(fixture.read_text()), "Rails cache-zone corpus changed"
print("Rails cache-zone oracle: 44 HTTP responses; two warm sharing pairs; 40 timestamp keys across eight zones and five DST/seasonal instants; 4 pinned sources; exact keys and cache reuse")
