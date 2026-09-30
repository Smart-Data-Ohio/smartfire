#!/usr/bin/env python3
"""Fail on duplicate workspace dependency keys, including duplicates TOML would reject."""
from pathlib import Path
import re
import tomllib

source = (Path(__file__).resolve().parents[1] / "Cargo.toml").read_text()
parsed = tomllib.loads(source)
section = source.split("[workspace.dependencies]", 1)[1].split("\n[", 1)[0]
keys = re.findall(r"^([\w-]+)\s*=", section, re.M)
assert len(keys) == len(set(keys)) == len(parsed["workspace"]["dependencies"])
print(f"workspace dependency keys: {len(keys)} unique; 0 duplicates")
