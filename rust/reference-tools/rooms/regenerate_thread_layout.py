#!/usr/bin/env python3
"""Regenerate complete thread pages from the shared pin, which includes #163."""
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[3]
subprocess.run(["python3", str(root / "rust/reference-tools/messaging/features-reference-check.py")], cwd=root, check=True)
subprocess.run(["python3", str(root / "rust/reference-tools/messaging/check-goldens.py"), "--write", "thread-pages"], cwd=root, check=True)
