#!/usr/bin/env python3
"""Capture notification payloads from the current plain Rails reference pin."""
import json
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[2]
pin = (root / "rust/parity/reference.sha").read_text().strip()[:8]
scratch = Path(os.environ.get("PARITY_PIN_LOG_DIR", root / ".scratch"))
scratch.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, PARITY_NAMESPACE="ws17", PARITY_OWNER="ws17",
           PARITY_IMAGE=os.environ.get("PARITY_IMAGE", "campfire-reference"))
subprocess.run(["python3", "rust/reference-tools/ws17_verify_reference.py"], cwd=root, env=env, check=True)
with (scratch / "notification-push-generated.json").open("w") as out, (scratch / "notification-push-generated.log").open("w") as err:
    subprocess.run(["rust/parity/bin/reference", "runner", "--seed", "default", "rust/reference-tools/ws17_notification_push.rb"], cwd=root, env=env, stdout=out, stderr=err, check=True)
value = json.loads((scratch / "notification-push-generated.json").read_text())
assert value["reference"] == pin and value["board_reference"] == pin
(root / "rust/vectors/ws17_notification_push.json").write_text(json.dumps(value, ensure_ascii=False) + "\n")
print(f"Rails notification push: {len(value['rows'])} complete source/policy payload cases; plain reference {pin}")
