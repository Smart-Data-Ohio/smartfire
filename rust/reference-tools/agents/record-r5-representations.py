#!/usr/bin/env python3
"""Capture each missing-representation scenario in its own fresh pinned seed."""
import json
import os
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parents[3]
output = Path(sys.argv[1]).resolve()
output.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, PARITY_IMAGE="ws11api-reference:d7c7de92")
env.setdefault("PARITY_NAMESPACE", "ws11api-r5-representations")
cases = []
for name in ["video_preview", "video_variant", "jpeg_variant"]:
    with (output / (name + ".log")).open("w") as stderr:
        result = subprocess.run([
            str(root / "rust/parity/bin/reference"), "exec", "--seed", "default",
            "bin/rails", "runner", "/work/reference-tools/agents/review192r5_missing_representations.rb", name,
        ], cwd=root, env=env, stdout=subprocess.PIPE, stderr=stderr, text=True)
    assert result.returncode == 0, (name, result.returncode)
    cases.append(json.loads(result.stdout))
vector = {"reference_pin": "d7c7de92", "notes": [
    "Real exact signed HTTP variations are processed and committed before deleting one file. Fixed storage keys and mtimes are fixture inputs; no responses or headers are masked.",
    "All seven responses per scenario retain their exact bodies and selected headers, including signed Locations. Missing-file requests change no rows, files or jobs.",
], "cases": cases}
(output / "agent_review192r5_representations.json").write_text(json.dumps(vector, ensure_ascii=False, indent=2) + "\n")
print("PR192 R5 Rails representations: 3 scenarios; 21 exact response bodies and header sets; 0 masks")
