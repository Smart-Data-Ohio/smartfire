#!/usr/bin/env python3
"""Reproduce complete configured composers from Rails, preserving every byte."""
import json
import os
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[3]
scratch=root/'.scratch/picker-reference';scratch.mkdir(parents=True,exist_ok=True)
env=dict(os.environ,PARITY_NAMESPACE='ws8br-picker',PARITY_OWNER='ws8br',PARITY_IMAGE='ws8br-reference-d7c7de92')
run=subprocess.run([str(root/'rust/parity/bin/reference'),'exec','--seed','default','--time','2026-03-02T16:00:00Z','--freeze',
    'bin/rails','runner','--skip-executor','/work/reference-tools/rooms/picker_components.rb'],cwd=root,env=env,capture_output=True,check=True)
(scratch/'stdout.json').write_bytes(run.stdout);(scratch/'stderr.log').write_bytes(run.stderr)
captured=json.loads(run.stdout)
fixture=json.loads((root/'rust/crates/campfire/src/controllers/rooms/picker_config.json').read_text())
assert captured==fixture,'Rails Picker/composer bytes changed'
print('Rails Picker oracle: 8 public configurations and complete root composers reproduced; all captured bytes unchanged')
