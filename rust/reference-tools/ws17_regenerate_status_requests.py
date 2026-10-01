#!/usr/bin/env python3
"""Actual pinned status-controller requests, persisted facts and complete Turbo strings."""
import json
import os
from pathlib import Path
import subprocess
root = Path(__file__).resolve().parents[2]
scratch = root / ".scratch"
scratch.mkdir(exist_ok=True)
subprocess.run(["python3", "rust/reference-tools/ws17_verify_reference.py"], cwd=root, check=True)
env = dict(os.environ, PARITY_NAMESPACE="ws17", PARITY_OWNER="ws17", PARITY_IMAGE="triage-reference-d7c7de92:latest")
with (scratch / "status-requests-generated.json").open("w") as out, (scratch / "status-requests-generated.log").open("w") as err:
    subprocess.run(["rust/parity/bin/reference", "runner", "--seed", "default", "rust/reference-tools/ws17_status_requests.rb"], cwd=root, env=env, stdout=out, stderr=err, check=True)
value = json.loads((scratch / "status-requests-generated.json").read_text())
assert value["reference"] == "d7c7de92"
(root / "rust/vectors/ws17_status_requests.json").write_text(json.dumps(value, ensure_ascii=False) + "\n")
print(f"Rails status requests: {len(value['rows'])} requests; {sum(len(row['frames']) for row in value['rows'])} complete Turbo frames; {len(value['claims'])} conditional OOO claims; 2 seeded failures")
