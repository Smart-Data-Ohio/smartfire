#!/usr/bin/env python3
"""Capture agent multi-process/multi-mode corpora from the current Rails image."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
ROOT = Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("output", type=Path)
parser.add_argument("--write", action="store_true")
options = parser.parse_args()
options.output = options.output.resolve()
options.output.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, PARITY_IMAGE=os.environ.get("PARITY_IMAGE", "campfire-reference"), PARITY_NAMESPACE="pin-refresh-aggregates", PARITY_CPUS=os.environ.get("PARITY_CPUS", "1"))
outputs = []
for mode, filename in [("models", "agents_next2_models.json"), ("jobs", "agents_next2_jobs.json"), ("streams", "agents_stream_remaining.json")]:
    directory = options.output / mode
    producer = "record-stream-remaining.py" if mode == "streams" else f"record-next2-{mode}.py"
    with (options.output / f"{mode}.log").open("wb") as log:
        subprocess.run(["python3", str(ROOT / "rust/reference-tools/agents" / producer), str(directory)], cwd=ROOT, env=env, stdout=log, stderr=log, check=True)
    outputs.append(directory / filename)
with tempfile.TemporaryDirectory(prefix="crash-", dir=options.output) as temporary:
    storage = Path(temporary)
    shutil.copytree(ROOT / "rust/parity/.seed/default", storage, dirs_exist_ok=True)
    args = [str(ROOT / "rust/parity/bin/reference"), "runner", "--storage", str(storage), "-e", "RAILS_LOG_LEVEL=fatal", str(ROOT / "rust/reference-tools/agents/deletion_crash_contract.rb")]
    with (options.output / "crash-stop.log").open("wb") as log:
        stopped = subprocess.run([*args, "stop"], cwd=ROOT, env=env, stdout=log, stderr=log)
    assert stopped.returncode == 73, stopped.returncode
    output = options.output / "agents_deletion_crash_contract.json"
    with output.open("wb") as stdout, (options.output / "crash-restart.log").open("wb") as stderr:
        subprocess.run([*args, "restart"], cwd=ROOT, env=env, stdout=stdout, stderr=stderr, check=True)
    outputs.append(output)
for output in outputs:
    json.loads(output.read_bytes())
    target = ROOT / "rust/vectors" / output.name
    if options.write:
        target.write_bytes(output.read_bytes())
    assert output.read_bytes() == target.read_bytes(), output.name
    print(f"Rails agent aggregate: {output.name}; independently captured whole file")
