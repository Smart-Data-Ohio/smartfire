#!/usr/bin/env python3
"""Capture each missing-representation scenario in its own fresh pinned seed."""

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
env.setdefault("PARITY_NAMESPACE", "ws11api-r5-representations")
cases = []
for name in ["video_preview", "video_variant", "jpeg_variant"]:
    with (output / (name + ".log")).open("w") as stderr:
        result = subprocess.run([
            str(root / "rust/parity/bin/reference"), "exec", "--seed", "default",
            "bin/rails", "runner", "/work/reference-tools/agents/review192r5_missing_representations.rb", name,
        ], cwd=root, env=env, stdout=subprocess.PIPE, stderr=stderr, text=True)
    assert result.returncode == 0, (name, result.returncode)
    (output / (name + "-raw.json")).write_text(result.stdout)
    case = json.loads(result.stdout)
    for response in [value for value in case.values() if isinstance(value, dict) and "headers" in value]:
        checked(response["headers"])
        # Only the three explicitly approved per-request names are stabilized.
        for header, value in {"date":"Mon, 02 Mar 2026 16:00:00 GMT", "x-request-id":"3574925f-479d-44f8-82b7-fc039af5367c", "x-runtime":"0.000000"}.items():
            if header in response["headers"]:
                assert len(response["headers"][header]) == 1, (header, response["headers"][header])
                response["headers"][header] = [value]
    cases.append(case)
vector = {"reference_pin": PIN, "notes": [
    "Real exact signed HTTP variations are processed and committed before deleting one file. Fixed storage keys, mtimes and CSP nonce-generator entropy are fixture inputs. Real config.ru Rack::Deflater entrypoint, HTTP/1.1, Accept */* and no Accept-Encoding.",
    "All seven responses per scenario retain their exact bodies and every response header, including signed Locations. Only Date, X-Request-Id and X-Runtime values are stabilized by name; raw receipts are retained beside the capture. The six named security defaults on Rust streamed proxies are approved additions, with exact values checked. Missing-file requests change no rows, files or jobs.",
], "cases": cases}
(output / "agent_review192r5_representations.json").write_text(json.dumps(vector, ensure_ascii=False, indent=2) + "\n")
print("PR192 R5 Rails representations: 3 scenarios; 21 exact response bodies; every header checked; 6 approved security additions and 3 per-request header names")
