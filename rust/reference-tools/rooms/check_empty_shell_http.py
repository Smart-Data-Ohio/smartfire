#!/usr/bin/env python3
"""Reproduce this room owner from the plain pinned Rails image."""
from pathlib import Path
import subprocess
import sys
import argparse

parser=argparse.ArgumentParser()
parser.add_argument("--record", action="store_true")
args=parser.parse_args()

runner=Path(__file__).resolve().parents[1] / "users/verify_room_merge.py"
command=[sys.executable, str(runner), "--only", 'empty-shell']
if args.record:
    command.append("--write")
subprocess.run(command, check=True)
