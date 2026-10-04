#!/usr/bin/env python3
"""Refresh provenance on declarative inputs; leave all request/assertion data intact."""
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
PIN = (ROOT / "rust/parity/reference.sha").read_text().strip()[:8]
NAMES = ["case-ports.json", "next5-work-named-inputs.json", "next6-named-inputs.json", "next6-named-controls.json", "pr227-deleted-work-inputs.json", "pr227-delivery-inputs.json"]
for name in NAMES:
    path = Path(__file__).parent / name
    value = json.loads(path.read_text())
    key = "reference_pin" if "reference_pin" in value else "reference"
    source_files = set()
    def sources(node):
        if isinstance(node, dict):
            for key, entry in node.items():
                if key in ("rails_file", "source_file", "file") and isinstance(entry, str) and entry.startswith("test/"):
                    source_files.add(entry)
                sources(entry)
        elif isinstance(node, list):
            for entry in node:
                sources(entry)
    sources(value)
    for source in source_files:
        subprocess.check_call(["git", "cat-file", "-e", f"{PIN}:{source}"], cwd=ROOT)
    value[key] = PIN
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")
    print(f"Input provenance: {name}; {len(source_files)} source declarations present at {PIN}")
subprocess.run(["python3", "rust/reference-tools/agents/write-case-status.py"], cwd=ROOT, check=True)
