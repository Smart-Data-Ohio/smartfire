#!/usr/bin/env python3
"""Reproduce viewer-zone card bytes exclusively from the pinned Rails HTTP path."""
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
scratch = root / ".scratch/zone-followup/rails"
scratch.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, PARITY_NAMESPACE="ws8br-event-zones", PARITY_OWNER="ws8br",
           PARITY_IMAGE="ws8br-reference-status-2e20b24c")
run = subprocess.run([str(root / "rust/parity/bin/reference"), "exec", "--seed", "default",
                      "--time", "2026-03-02T16:00:00Z", "--freeze", "bin/rails", "runner",
                      "--skip-executor", "/work/reference-tools/rooms/event_zones.rb"],
                     cwd=root, env=env, capture_output=True)
(scratch / "stdout.json").write_bytes(run.stdout)
(scratch / "stderr.log").write_bytes(run.stderr)
run.check_returncode()
captured = json.loads(run.stdout)
for path, digest in captured["sources"].items():
    pinned = subprocess.check_output(["git", "show", f"d7c7de92:{path}"], cwd=root)
    assert digest == hashlib.sha256(pinned).hexdigest(), f"Rails source drift: {path}"
fixture = root / "rust/crates/campfire/src/controllers/rooms/event_zones.json"
if args.record:
    fixture.write_text(json.dumps(captured, indent=2, ensure_ascii=False) + "\n")
assert captured == json.loads(fixture.read_text()), "Rails event-zone HTTP corpus changed"
print("Rails viewer-zone HTTP oracle: 18 responses, 24 complete event containers and 6 complete PR headers; Hawaii, both Eastern DST transitions and UTC fallbacks; byte-identical")
