#!/usr/bin/env python3
"""Replay all 80 actual relative-consumer requests through the shared renderer.

Slice H closes the formerly flagged 24 non-UTC cases. There is no fixture mask
or expected-difference allowance. check_rendering_mutants.py rejects restoring
UTC timestamps and avatar versions at the actual publication assertion.
"""
from pathlib import Path
import subprocess,sys
ROOT=Path(__file__).resolve().parents[3]
runner=sys.argv[1:]
if runner[:1]==['--']:runner=runner[1:]
assert runner
subprocess.run(runner+['test','--locked','-p','campfire','--bin','campfire','exceptional_relative_consumers_match_rails_complete_state_with_flat_reads','-j2','--','--test-threads=4','--nocapture'],cwd=ROOT,check=True)
print('WS8bm2 relative renderer: 80/80 real requests matched Rails; non-UTC timestamp/avatar gap closed',flush=True)
