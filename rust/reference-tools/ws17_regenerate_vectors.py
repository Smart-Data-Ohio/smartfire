#!/usr/bin/env python3
"""Regenerate WS17 goldens from our verified pinned Rails image, never from upstream."""

from status_pin_identity import PIN, PIN_FULL, PIN_IMAGE
import argparse
import json
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--include-web-push", action="store_true",
                    help="Regenerate the separate unpinned encryption sampler with fresh keys")
options = parser.parse_args()
scratch = Path(os.environ.get("PARITY_PIN_LOG_DIR", root / ".scratch"))
scratch.mkdir(exist_ok=True)
env = dict(os.environ, PARITY_NAMESPACE="ws17", PARITY_OWNER="ws17",
           PARITY_IMAGE=PIN_IMAGE)
subprocess.run(["python3", "rust/reference-tools/ws17_verify_reference.py"], cwd=root, check=True)
outputs = []
for name, script, destination in [
    ("policy", "rust/reference-tools/ws17_vectors.rb", "rust/crates/db/src/tests/ws17_vectors.json"),
    ("presence", "rust/reference-tools/ws17_presence_vectors.rb", "rust/vectors/ws17_presence.json"),
    *([("web-push", "rust/crates/campfire/src/integrations/testdata/oracle/web_push.rb",
        "rust/crates/campfire/src/integrations/testdata/web_push_expected.json")]
      if options.include_web_push else []),
]:
    pending = scratch / f"{name}-regenerated.json"
    with pending.open("w") as stdout, (scratch / f"{name}-regenerated.log").open("w") as stderr:
        subprocess.run(["rust/parity/bin/reference", "runner", "--seed", "default", script],
                       cwd=root, env=env, stdout=stdout, stderr=stderr, check=True)
    value = json.loads(pending.read_text())
    (root / destination).write_text(pending.read_text())
    outputs.append(value)
policy, presence = outputs[:2]
if options.include_web_push:
    web_push = outputs[2]
    assert web_push["vapid_subject"] == "mailto:support@smartdata.net"
    assert json.loads(web_push["message"])["options"]["tag"] == "room-1"
print(f"Rails WS17 vectors: {len(policy['policies'])} policy; {len(policy['statuses'])} status; "
      f"{len(policy['integer_coercions'])} Integer; {len(presence['cases'])} presence HTTP; "
      f"{'1 fresh encryption sampler' if options.include_web_push else 'unkeyed encryption sampler preserved'}")
