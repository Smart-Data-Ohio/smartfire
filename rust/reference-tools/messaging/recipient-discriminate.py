#!/usr/bin/env python3
"""Verify complete forward delivery rejects a compiled private-subscription regression."""
import os
from pathlib import Path
import re
import subprocess
ROOT=Path(__file__).resolve().parents[3]
OUT=ROOT/'.scratch/recipient-discrimination'
OUT.mkdir(parents=True,exist_ok=True)
source=ROOT/'rust/crates/campfire/src/channels/room_messages.rs'
original=source.read_text()
old='Room::find_for_user(conn, user_id, room_id)'
new='let _ = user_id; Room::find_by_id(conn, room_id)'
assert original.count(old)==1
command=['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j4','--manifest-path','rust/Cargo.toml','-p','campfire','--bin','campfire','channels::tests::hub_test::message_parity::positive_forward_message_and_unread_frames_match_rails_for_every_recipient','--','--exact','--nocapture']
env=dict(os.environ,CI='1',TMPDIR=str(ROOT/'.scratch'),CARGO_TARGET_DIR=str(ROOT/'rust/target'),CABLE_TEST_PORT_RANGE='52000-52049',MAIL_TEST_PORT_RANGE='52000-52049')
env.pop('RUST_TEST_THREADS',None)
try:
    source.write_text(original.replace(old,new))
    result=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True)
    output=result.stdout+result.stderr
    (OUT/'private-stream.log').write_text(output)
    summary=re.findall(r'^test result: FAILED\..*$',output,re.M)
    assert result.returncode and summary and 'confirm_subscription' in output and 'reject_subscription' in output
    print('private-stream-scope: '+summary[0],flush=True)
finally:
    source.write_text(original)
result=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True)
output=result.stdout+result.stderr
(OUT/'restored.log').write_text(output)
assert result.returncode==0,'Restored recipient differential failed'
for line in output.splitlines():
    if line.startswith(('test result:','WS8bm forward recipients:')):print(line,flush=True)
print('WS8bm recipient discriminator: compiled private-stream regression rejected; source restored and all eligible deliveries passed',flush=True)
