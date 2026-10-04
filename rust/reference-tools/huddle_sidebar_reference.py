#!/usr/bin/env python3
"""Verify sidebar/layout inputs in the current plain Rails reference image."""
import hashlib
import os
from pathlib import Path
import subprocess
root = Path(__file__).resolve().parents[2]
pin = (root / "rust/parity/reference.sha").read_text().strip()
image = os.environ.get("PARITY_IMAGE", "campfire-reference")
paths = ["app/views/users/sidebars/show.html.erb", "app/views/layouts/application.html.erb"]
checks = subprocess.check_output(["docker", "run", "--rm", "--network", "none", "--entrypoint", "sha256sum", image, *["/rails/" + path for path in paths]], text=True).splitlines()
for path, row in zip(paths, checks, strict=True):
    source = subprocess.check_output(["git", "show", f"{pin}:{path}"], cwd=root)
    assert hashlib.sha256(source).hexdigest() == row.split()[0], path
print(f"Sidebar reference: {len(paths)} source files verified at {pin}; no derivative image")
