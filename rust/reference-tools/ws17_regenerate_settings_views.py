#!/usr/bin/env python3
"""Generate byte goldens from actual pinned Rails status/notification partials."""
import json
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root / ".scratch"
scratch.mkdir(exist_ok=True)
subprocess.run(["python3", "rust/reference-tools/ws17_verify_reference.py"], cwd=root, check=True)
env = dict(os.environ, PARITY_NAMESPACE="ws17", PARITY_OWNER="ws17", PARITY_IMAGE="triage-reference-d7c7de92:latest")
with (scratch / "settings-views-generated.json").open("w") as out, (scratch / "settings-views-generated.log").open("w") as err:
    subprocess.run(["rust/parity/bin/reference", "runner", "--seed", "default", "rust/reference-tools/ws17_settings_views.rb"], cwd=root, env=env, stdout=out, stderr=err, check=True)
value = json.loads((scratch / "settings-views-generated.json").read_text())
assert value["reference"] == "d7c7de92"
destination = root / "rust/crates/views/tests/golden/ws17-settings.json"
destination.write_text(json.dumps(value, ensure_ascii=False) + "\n")
print(f"Rails settings HTML: {len(value['rows'])} states; {len(value['rows']) * 2} complete partials")
