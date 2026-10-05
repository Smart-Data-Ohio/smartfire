#!/usr/bin/env python3
"""Require real surviving/rejected relocated-producer receipts; exclude setup failures."""
import argparse
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--evidence',type=Path,required=True)
a=p.parse_args()
controls=[('picker','picker-filter','picker controls moved outside required frame {"scoped":0,"global":8}','test/system/people_group_dms_test.rb:192: original assertion'),('tour','tour-finish','real next control outside tour card',"waiting for locator('.tour__card [data-tour-target=")]
for name,case,producer,rejection in controls:
 before=(a.evidence/f'{name}-before.log').read_text()
 after=(a.evidence/f'{name}-after.log').read_text()
 for phase,log in [('before',before),('after',after)]:
  assert 'ORIGINAL_MUTATION '+producer in log,(name,phase,'producer not changed')
  assert 'INVALID_CONTROL transport' not in log and 'Rust server exited' not in log and 'failed startup' not in log,(name,phase,'invalid transport/setup')
 assert f'ORIGINAL_CASE {case}: passed' in before and 'ORIGINAL_ASSERTION transport:0' in before,(name,'original mutation did not survive')
 assert f'ORIGINAL_CASE {case}: passed' not in after and rejection in after,(name,'scoped helper failed to discriminate')
 print(f'Scope control {name}: survived before; rejected after at required ancestor; 0 invalid controls')
before=(a.evidence/'status-before.log').read_text()
after=(a.evidence/'status-after.log').read_text()
producer='ORIGINAL_MUTATION real status text field outside user card with form association retained'
assert producer in before and producer in after
assert 'Rust directory browser scenarios:' in before and 'Rust directory browser scenarios:' in after
assert 'sidebar-popup-save-closes-and-persists: passed' in before
assert 'sidebar-popup-save-closes-and-persists: passed' not in after
assert 'TimeoutError' in after and "waiting for locator('#user_card').locator('#status_popup_custom_status_text')" in after
assert 'INVALID_CONTROL transport' not in after and 'Rust server exited' not in after
print('Scope control status: survived before; rejected after at required ancestor; 0 invalid controls')
print('Scope mutation receipts: 3 surviving before; 3 rejected after; 0 invalid controls')
