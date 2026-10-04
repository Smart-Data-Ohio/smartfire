#!/usr/bin/env python3
"""Regenerate writer and legal-zone vectors from the pinned Smartfire Rails image."""

from status_pin_identity import PIN, PIN_FULL, PIN_IMAGE
import json
import os
from pathlib import Path
import subprocess

root=Path(__file__).resolve().parents[2]
scratch = Path(os.environ.get("PARITY_PIN_LOG_DIR", root / ".scratch"))
scratch.mkdir(exist_ok=True)
env=dict(os.environ,PARITY_NAMESPACE="ws17",PARITY_OWNER="ws17",PARITY_IMAGE=PIN_IMAGE)
subprocess.run(["python3","rust/reference-tools/ws17_verify_reference.py"],cwd=root,check=True)
with (scratch/"settings-generated.json").open("w") as stdout, (scratch/"settings-generated.log").open("w") as stderr:
    subprocess.run(["rust/parity/bin/reference","runner","--seed","default","rust/reference-tools/ws17_settings_vectors.rb"],cwd=root,env=env,stdout=stdout,stderr=stderr,check=True)
value=json.loads((scratch/"settings-generated.json").read_text())
assert value["reference"]==PIN
(root/"rust/crates/db/src/tests/ws17_settings_vectors.json").write_text(json.dumps({k:v for k,v in value.items() if k!="zones"},ensure_ascii=False)+"\n")
(root/"rust/crates/db/src/tests/ws17_settings_zones.json").write_text(json.dumps(value["zones"])+"\n")
print(f"Rails settings vectors: {len(value['rows'])} presets; {len(value['clocks'])} clock setters; {len(value['validations'])} validations; {len(value['zones']['names'])} legal zone names; 1 dirty-write scenario")
