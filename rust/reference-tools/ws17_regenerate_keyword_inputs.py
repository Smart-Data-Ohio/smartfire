#!/usr/bin/env python3
"""Generate actual Rails params coercion and keyword replacement vectors."""

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
with (scratch / "keyword-input.json").open("w") as out, (scratch / "keyword-input.log").open("w") as err:
    subprocess.run(["rust/parity/bin/reference", "runner", "--seed", "default", "rust/reference-tools/ws17_keyword_input_vectors.rb"], cwd=root, env=env, stdout=out, stderr=err, check=True)
value = json.loads((scratch / "keyword-input.json").read_text())
assert value["reference"] == PIN
(root / "rust/vectors/ws17_keyword_input.json").write_text(json.dumps(value, ensure_ascii=False) + "\n")
print(f"Rails keyword input: {len(value['rows'])} parameter and writer scenarios")
