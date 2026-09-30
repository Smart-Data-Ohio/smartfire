#!/usr/bin/env python3
"""Runtime reset authorization, cleanup, audit and IP/user-window security defects."""
from pathlib import Path
import os,re,subprocess
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT.parent/'.scratch/admin-mutations';OUT.mkdir(parents=True,exist_ok=True)
ctrl='crates/campfire/src/controllers/accounts/users/two_factor_resets.rs'
domain='crates/campfire/src/authentication.rs'
rates='crates/campfire/src/controllers/two_factor.rs'
cases=[
 ('reset-admin-gate-omitted',ctrl,'concerns::ensure_can_administer(c)?;','// deliberately omitted','reset_requires'),
 ('reset-self-allowed',ctrl,'if user.id == require_current_user(c)?.id','if false','self_and_unenrolled'),
 ('reset-inactive-bots-allowed',ctrl,'.filter(|u| u.is_active() && !u.is_bot())','.filter(|_|true)','reset_requires'),
 ('reset-sessions-survive',domain,'for session in Session::for_user(tx.conn(), user.id)?','for session in Vec::<Session>::new()','reset_removes'),
 ('reset-audit-wrong-action',domain,'action: "two_factor.reset".into()','action: "two_factor.not_reset".into()','failed_audit'),
 ('ip-limiter-omitted',rates,'RateLimit::new(scope, 10, SignedDuration::from_mins(3))','RateLimit::new(scope, 100, SignedDuration::from_mins(3))','self_service_limits'),
 ('ip-window-too-long',rates,'RateLimit::new(scope, 10, SignedDuration::from_mins(3))','RateLimit::new(scope, 10, SignedDuration::from_mins(5))','self_service_limits'),
 ('user-limiter-omitted',rates,'RateLimit::new(scope, 10, SignedDuration::from_mins(15))','RateLimit::new(scope, 100, SignedDuration::from_mins(15))','self_service_limits'),
 ('user-window-too-short',rates,'RateLimit::new(scope, 10, SignedDuration::from_mins(15))','RateLimit::new(scope, 10, SignedDuration::from_mins(5))','self_service_limits'),
]
for name,path,needle,replacement,test in cases:
 source=ROOT/path;original=source.read_text();matched=re.search(r'\s*'.join(re.escape(p) for p in needle.split()),original)
 if matched is None:raise RuntimeError(f'{name}: mutation anchor disappeared')
 try:
  source.write_text(original[:matched.start()]+replacement+original[matched.end():])
  result=subprocess.run(['cargo','test','--locked','-j','4','-p','campfire','app::admin_two_factor_tests::'+test],cwd=ROOT,env=dict(os.environ,CI='1',TMPDIR=str(ROOT.parent/'.scratch/tmp'),CABLE_TEST_PORT_RANGE='51600-51699'),stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  (OUT/(name+'.log')).write_text(result.stdout);summaries=[s for s in result.stdout.splitlines() if s.startswith('test result:')]
  if result.returncode==0 or not any('FAILED' in s for s in summaries):raise RuntimeError(f'{name}: no real failing test; see {OUT}')
  print(name+': '+summaries[-1],flush=True)
 finally:source.write_text(original)
print(f'WS9 admin and rate security gates: {len(cases)} deliberate defects rejected',flush=True)
