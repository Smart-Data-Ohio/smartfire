#!/usr/bin/env python3
"""Regenerate WS17 goldens from our verified pinned Rails image, never from upstream."""
import json
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root / ".scratch"
scratch.mkdir(exist_ok=True)
env = dict(os.environ, PARITY_NAMESPACE="ws17", PARITY_OWNER="ws17",
           PARITY_IMAGE="triage-reference-d7c7de92:latest")
subprocess.run(["python3", "rust/reference-tools/ws17_verify_reference.py"], cwd=root, check=True)
outputs = []
for name, script, destination in [
    ("policy", "rust/reference-tools/ws17_vectors.rb", "rust/crates/db/src/tests/ws17_vectors.json"),
    ("presence", "rust/reference-tools/ws17_presence_vectors.rb", "rust/vectors/ws17_presence.json"),
    ("web-push", "rust/crates/campfire/src/integrations/testdata/oracle/web_push.rb",
     "rust/crates/campfire/src/integrations/testdata/web_push_expected.json"),
]:
    pending = scratch / f"{name}-regenerated.json"
    with pending.open("w") as stdout, (scratch / f"{name}-regenerated.log").open("w") as stderr:
        subprocess.run(["rust/parity/bin/reference", "runner", "--seed", "default", script],
                       cwd=root, env=env, stdout=stdout, stderr=stderr, check=True)
    value = json.loads(pending.read_text())
    (root / destination).write_text(pending.read_text())
    outputs.append(value)
policy, presence, web_push = outputs
assert web_push["vapid_subject"] == "mailto:support@smartdata.net"
assert json.loads(web_push["message"])["options"]["tag"] == "room-1"
print(f"Rails WS17 vectors: {len(policy['policies'])} policy; {len(policy['statuses'])} status; "
      f"{len(policy['integer_coercions'])} Integer; {len(presence['cases'])} presence HTTP; 1 encrypted payload")
