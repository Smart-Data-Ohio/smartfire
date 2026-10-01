#!/usr/bin/env python3
"""TOML rejects duplicate keys, including merged workspace.dependencies entries."""
from pathlib import Path
import subprocess
import tomllib

root = Path(__file__).resolve().parents[2]
paths = subprocess.check_output(["rg", "--files", "--hidden", str(root), "-g", "Cargo.toml", "-g", "!target/**"], text=True).splitlines()
assert paths
for path in paths:
    with open(path, "rb") as file:
        tomllib.load(file)
print("Cargo TOML duplicate-key check: all manifests parse; no duplicate workspace dependency keys")
