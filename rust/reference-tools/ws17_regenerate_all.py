#!/usr/bin/env python3
"""Replay current-pin status contracts; shared core/assets exports remain with their owner."""
import argparse
import os
from pathlib import Path
import subprocess
ROOT = Path(__file__).resolve().parents[2]
NAMES = ['vectors', 'unicode', 'settings', 'settings_views', 'status_requests', 'profile_ui', 'dm_profile', 'calendar_dispatch', 'message_activity', 'keyword_inputs', 'endpoint_urls', 'named_policy', 'named_readers', 'named_gating', 'named_calendar_status', 'notification_push', 'auth_reference']
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("output", type=Path)
parser.add_argument("--names", default="")
options = parser.parse_args()
options.output = options.output.resolve()
options.output.mkdir(parents=True, exist_ok=True)
names = NAMES
if options.names:
    selected = set(options.names.split(","))
    assert selected <= set(names), selected - set(names)
    names = [name for name in names if name in selected]
env = dict(os.environ, PARITY_IMAGE=os.environ.get("PARITY_IMAGE", "campfire-reference"), PARITY_PIN_LOG_DIR=str(options.output), PARITY_CPUS=os.environ.get("PARITY_CPUS", "1"))
for name in names:
    args = ["python3", str(ROOT / "rust/reference-tools" / f"ws17_regenerate_{name}.py")]
    if name == "auth_reference":
        args.append("--skip-shared-exports")
    with (options.output / f"{name}-summary.log").open("wb") as log:
        subprocess.run(args, cwd=ROOT, env=env, stdout=log, stderr=log, check=True)
    print(f"Rails status producer: {name}; current-pin sources verified and outputs captured", flush=True)
print(f"Rails status: {len(names)} producer groups complete; shared core/assets exports skipped", flush=True)
