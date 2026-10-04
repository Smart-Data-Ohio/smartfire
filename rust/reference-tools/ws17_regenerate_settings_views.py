#!/usr/bin/env python3
"""Generate byte goldens from actual pinned Rails status/notification partials."""

from status_pin_identity import PIN, PIN_FULL, PIN_IMAGE
import json
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = Path(os.environ.get("PARITY_PIN_LOG_DIR", root / ".scratch"))
scratch.mkdir(exist_ok=True)
subprocess.run(["python3", "rust/reference-tools/ws17_verify_reference.py"], cwd=root, check=True)
env = dict(os.environ, PARITY_NAMESPACE="ws17", PARITY_OWNER="ws17", PARITY_IMAGE=PIN_IMAGE)
with (scratch / "settings-views-generated.json").open("w") as out, (scratch / "settings-views-generated.log").open("w") as err:
    subprocess.run(["rust/parity/bin/reference", "runner", "--seed", "default", "rust/reference-tools/ws17_settings_views.rb"], cwd=root, env=env, stdout=out, stderr=err, check=True)
value = json.loads((scratch / "settings-views-generated.json").read_text())
assert value["reference"] == PIN
destination = root / "rust/crates/views/tests/golden/ws17-settings.json"
destination.write_text(json.dumps(value, ensure_ascii=False) + "\n")
print(f"Rails settings HTML: {len(value['rows'])} states; {len(value['rows']) * 2} complete forms; {len(value['rows']) * 2} complete status partials")
