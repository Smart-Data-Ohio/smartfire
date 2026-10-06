#!/usr/bin/env python3
"""Require actual writer defects to fail the Rust browser behavior gate."""
import argparse
from pathlib import Path
import subprocess

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary',type=Path,required=True)
args=p.parse_args()
root=Path(__file__).resolve().parents[4]
runner=Path(__file__).with_name('system_behavior.py')
cases = [
 ('work','--inject-work-status','agent_work_assignment_test.rb',0,'committed work status remained planned'),
 ('pages','--inject-stream-finalize','agent_streaming_test.rb: streaming message renders, updates live, and finalizes',6,'stream did not finalize'),
]
for scenario,_,_,controls,_ in cases:
 result=subprocess.run(['python3',str(runner),'--binary',str(args.binary.resolve()),'--scenario',scenario],cwd=root,capture_output=True,text=True)
 output=result.stdout+result.stderr
 if result.returncode or output.count(f'Agent system behavior: {controls+1} passed; 0 failed; 0 deferred') != 1:
  raise RuntimeError(f'Invalid discrimination: unmutated {scenario} baseline failed\n{output}')
 print(f'{scenario} baseline: Rust {controls+1} passed; 0 failed', flush=True)
for scenario,injection,name,controls,assertion in cases:
 result=subprocess.run(['python3',str(runner),'--binary',str(args.binary.resolve()),'--scenario',scenario,injection],cwd=root,capture_output=True,text=True)
 output=result.stdout+result.stderr
 failures=[line for line in output.splitlines() if line.startswith('FAIL ')]
 if result.returncode != 1 or len(failures) != 1 or f'FAIL {name}' not in failures[0] or assertion not in failures[0]:
  raise RuntimeError(f'Invalid discrimination: {scenario} did not fail at its intended assertion\n{output}')
 if 'Rust system behavior:' not in output or f'Agent system behavior: {controls} passed; 1 failed; 0 deferred' not in output.split('Rust system behavior:',1)[1]:
  raise RuntimeError(f'Invalid discrimination: {scenario} control counts changed\n{output}')
 print(f'{scenario} writer discrimination: Rust {controls} passed, 1 deliberate failure; gate rejected; waits unchanged')
