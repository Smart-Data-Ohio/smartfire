#!/usr/bin/env python3
"""Prove every newly pinned named comparison rejects a wrong observable.

Temporarily changes only the three new oracle files, never production sources;
restores their exact bytes even on interruption/error. Supply an owned target
and the usual CI/port/TMPDIR environment. These are oracle negative controls,
not claims that the already-correct production behavior failed initially.
"""
import json
import os
from pathlib import Path
import re
import subprocess
import sys

root = Path(__file__).resolve().parents[3]
logs = Path(sys.argv[1]).resolve()
logs.mkdir(parents=True, exist_ok=True)
assert os.environ.get("CARGO_TARGET_DIR"), "supply an owned CARGO_TARGET_DIR"
names = ["agents_next2_models.json", "agents_next2_jobs.json", "agents_stream_remaining.json"]
originals = {root / "rust/vectors" / name: (root / "rust/vectors" / name).read_bytes() for name in names}
try:
    models = json.loads(next(data for path, data in originals.items() if path.name == names[0]))
    values = models["results"]
    values["create_bot"]["name"] = "incorrect factory name"
    values["reset_key"]["digest"] = "incorrect digest"
    values["hop_limit"]["events"][-1]["hop"] = 0
    values["human_root"]["events"][0]["hop"] = 1
    values["delete_no_owner"]["events"] = 1
    values["budget_viewer"]["stranger"] = 1
    values["budget_handoff"]["final_usage"]["messages"] = 2
    jobs = json.loads(next(data for path, data in originals.items() if path.name == names[1]))
    jobs["results"]["queued_bot"]["requests"][0]["signature"] = "incorrect queued HMAC"
    jobs["results"]["delete_owned"]["after"][-1]["metadata"]["work_snapshot"]["title"] = "incorrect deletion snapshot"
    streams = json.loads(next(data for path, data in originals.items() if path.name == names[2]))
    for name in ["start", "finalize", "append", "trailing"]:
        # Keep the frame counts intact so controls fail on exact comparison,
        # rather than introducing timeouts or changing timing thresholds.
        streams["results"][name]["snapshots"][-1]["frames"][-1] += "incorrect rendered suffix"
    for name, vector in zip(names, [models, jobs, streams]):
        (root / "rust/vectors" / name).write_text(json.dumps(vector, ensure_ascii=False, indent=2) + "\n")
    for package, expected in [("campfire_db", 7), ("campfire", 6)]:
        log = logs / (package + "-negative-controls.log")
        with log.open("w") as output:
            result = subprocess.run(["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j2", "--manifest-path", "rust/Cargo.toml", "-p", package, "ws11_next2_", "--", "--nocapture", "--test-threads=8"], cwd=root, stdout=output, stderr=subprocess.STDOUT)
        summaries = re.findall(r"^test result:.*$", log.read_text(), re.M)
        assert result.returncode == 101 and summaries, (package, result.returncode)
        assert f"{expected} failed;" in summaries[-1], summaries
        # The blob-header case also shares this filter and should keep passing.
        print(f"WS11 negative controls {package}: {summaries[-1]}", flush=True)
finally:
    for path, data in originals.items():
        path.write_bytes(data)
        assert path.read_bytes() == data
print("WS11 remaining comparison negative controls: 13 expected failures; exact oracle bytes restored", flush=True)
