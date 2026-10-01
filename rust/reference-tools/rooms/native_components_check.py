#!/usr/bin/env python3
"""Strict owner-component acceptance. Failures remain explicit; no masks or allowlists."""
import argparse,json,os,subprocess,difflib
from pathlib import Path
parser=argparse.ArgumentParser();parser.add_argument('--capture-log',type=Path,required=True);args=parser.parse_args()
root=Path(__file__).resolve().parents[3]
lines=args.capture_log.read_text().splitlines()
line=next(line for line in lines if line.startswith('WS8BR_NATIVE_COMPONENTS:'))
actual=json.loads(line.split(':',1)[1])
golden=json.loads((root/'rust/crates/views/tests/golden/rooms/native_components.json').read_text())['rows']
scratch=root/'.scratch/native-components-diff';scratch.mkdir(parents=True,exist_ok=True)
passed=failed=0
for expected,row in zip(golden,actual,strict=True):
 assert (row['room_id'],row['user_id'])==(expected['room_id'],expected['user_id'])
 for name in ['message_list','composer','pending_template']:
  a,b=row[name],expected[name]
  actual_bytes,expected_bytes=a.encode('utf-8'),b.encode('utf-8')
  if actual_bytes==expected_bytes:passed+=1;print(f"PASS room {row['room_id']} {name}: {len(actual_bytes)} exact bytes")
  else:
   failed+=1;prefix=scratch/f"{row['room_id']}-{name}"
   prefix.with_suffix('.actual').write_text(a);prefix.with_suffix('.expected').write_text(b)
   prefix.with_suffix('.diff').write_text('\n'.join(difflib.unified_diff(b.splitlines(),a.splitlines(),fromfile='Rails',tofile='Rust')))
   print(f"FAIL room {row['room_id']} {name}: Rust {len(actual_bytes)} bytes, Rails {len(expected_bytes)} bytes")
print(f"Native room component acceptance: {passed} exact matches; {failed} differences; no masks")
raise SystemExit(bool(failed))
