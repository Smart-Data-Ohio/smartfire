#!/usr/bin/env python3
"""Reject runtime device, mail, owner-scope, disconnect and idempotence defects."""
from pathlib import Path
import os, re, subprocess
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT.parent/'.scratch/session-mutations'
OUT.mkdir(parents=True,exist_ok=True)
domain='crates/campfire/src/authentication.rs'
cases=[
 ('device-tracking-omitted',domain,'if campfire_db::UserDevice::record_sign_in(','if false && campfire_db::UserDevice::record_sign_in(','completed_sign_ins'),
 ('sign-in-mail-omitted',domain,'campfire_mail::jobs::new_sign_in_alert_later(tx, item.id);','// deliberately omitted','configured_sign_in'),
 ('session-owner-ignored','crates/campfire/src/controllers/users/sessions.rs','let user_id = user.id;','let user_id = 127326141;','revoke_scopes'),
 ('revocation-disconnect-omitted',domain,'session.destroy(tx)?;\n    user.reset_remote_connections(tx);','session.destroy(tx)?;','revocation_drains'),
 ('noop-revocation-audited',domain,'if others.is_empty()','if false','revoke_others'),
]
for name,path,needle,replacement,test in cases:
 source=ROOT/path;original=source.read_text()
 pattern=r'\s*'.join(re.escape(p) for p in needle.split());matched=re.search(pattern,original)
 if matched is None:raise RuntimeError(f'{name}: mutation anchor disappeared')
 try:
  source.write_text(original[:matched.start()]+replacement+original[matched.end():])
  result=subprocess.run(['cargo','test','--locked','-j','4','-p','campfire','app::session_management_tests::'+test],cwd=ROOT,env=dict(os.environ,CI='1',TMPDIR=str(ROOT.parent/'.scratch/tmp'),CABLE_TEST_PORT_RANGE='51600-51699'),stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  (OUT/(name+'.log')).write_text(result.stdout)
  summaries=[s for s in result.stdout.splitlines() if s.startswith('test result:')]
  if result.returncode==0 or not any('FAILED' in s for s in summaries):raise RuntimeError(f'{name}: no real failing test; see {OUT}')
  print(name+': '+summaries[-1],flush=True)
 finally:source.write_text(original)
print(f'WS9 session security gates: {len(cases)} deliberate defects rejected',flush=True)
