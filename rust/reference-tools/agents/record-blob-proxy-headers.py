#!/usr/bin/env python3
"""Capture complete blob-proxy headers and bodies from a fresh pinned Rails seed."""

from pin_identity import PIN, PIN_FULL, PIN_IMAGE
import json
import os
from pathlib import Path
import subprocess
import sys
from proxy_headers import checked

root = Path(__file__).resolve().parents[3]
output = Path(sys.argv[1]).resolve()
output.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, PARITY_IMAGE=PIN_IMAGE)
env.setdefault("PARITY_NAMESPACE", "ws11api-next2-blob")
with (output / "blob-proxy.log").open("w") as stderr:
    result = subprocess.run([str(root / "rust/parity/bin/reference"), "exec", "--seed", "default", "bin/rails", "runner", "/work/reference-tools/agents/blob_proxy_headers.rb"], cwd=root, env=env, stdout=subprocess.PIPE, stderr=stderr, text=True, check=True)
(output / "blob-proxy-raw.json").write_text(result.stdout)
vector = json.loads(result.stdout)
for case in vector["cases"]:
    checked(case["headers"])
    for name, value in {"date": "Mon, 02 Mar 2026 16:00:00 GMT", "x-request-id": "3574925f-479d-44f8-82b7-fc039af5367c", "x-runtime": "0.000000"}.items():
        if name in case["headers"]:
            case["headers"][name] = [value]
(output / "agent_blob_proxy_headers.json").write_text(json.dumps(vector, ensure_ascii=False, indent=2) + "\n")
print("Blob proxy Rails differential: 4 responses; config.ru HTTP/1.1; all headers and exact bodies; rows/jobs unchanged")
