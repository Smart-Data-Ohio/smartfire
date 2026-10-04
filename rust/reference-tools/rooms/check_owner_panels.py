#!/usr/bin/env python3
"""Reproduce this room owner from the plain pinned Rails image."""
from pathlib import Path
import subprocess
import sys

runner=Path(__file__).resolve().parents[1] / "users/verify_room_merge.py"
command=[sys.executable, str(runner), "--only", 'panels', 'pins']
subprocess.run(command, check=True)
