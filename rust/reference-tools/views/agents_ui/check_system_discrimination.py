#!/usr/bin/env python3
"""Require actual writer defects to fail the browser parity gate."""
import argparse
from pathlib import Path
import subprocess

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary',type=Path,required=True)
args=p.parse_args()
root=Path(__file__).resolve().parents[4]
runner=Path(__file__).with_name('system_behavior.py')
for scenario,injection,name,controls in [
 ('work','--inject-work-status','agent_work_assignment_test.rb',0),
 ('pages','--inject-stream-finalize','agent_streaming_test.rb: streaming message renders, updates live, and finalizes',6),
]:
 result=subprocess.run(['python3',str(runner),'--binary',str(args.binary.resolve()),'--scenario',scenario,injection],cwd=root,capture_output=True,text=True)
 output=result.stdout+result.stderr
 assert result.returncode and f'FAIL {name}' in output, output
 rails,rust=output.split('Rust system behavior:',1)
 assert f'Agent system behavior: {controls+1} passed; 0 failed; 0 deferred' in rails, output
 assert f'Agent system behavior: {controls} passed; 1 failed; 0 deferred' in rust, output
 print(f'{scenario} writer discrimination: Rails {controls+1} passed; Rust {controls} passed, 1 deliberate failure; gate rejected; waits unchanged')
