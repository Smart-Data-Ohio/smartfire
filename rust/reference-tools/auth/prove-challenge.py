#!/usr/bin/env python3
"""Reproducible runtime faults for the second-factor and self-service HTTP tests."""
from pathlib import Path
import os,re,subprocess
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT.parent/'.scratch/challenge-mutations';OUT.mkdir(parents=True,exist_ok=True)
ctrl='crates/campfire/src/controllers/two_factor.rs'
domain='crates/campfire/src/authentication.rs'
cases=[
 ('locked-code-accepted',domain,'if credential.locked_out(tx.now())','if false','challenge_backup'),
 ('pending-state-never-expires','crates/campfire/src/concerns/session_keys.rs','if to_i(expires_at) < now.as_second()','if false','challenge_pending_expires'),
 ('challenge-session-unverified','crates/campfire/src/concerns.rs','start_session(c, user, true).await','start_session(c, user, false).await','challenge_first_factor'),
 ('reauthentication-bypassed',ctrl,'async fn reauthenticated(c: &mut Ctx, user: &User) -> Result<bool> {','async fn reauthenticated(c: &mut Ctx, user: &User) -> Result<bool> { return Ok(true);','self_service_requires'),
 ('unverified-disable-allowed',ctrl,'if current_session(c).is_some_and(|s| s.two_factor_verified())','if true','disable_refuses'),
 ('google-subject-ignored',ctrl,'if linked_subject(c, user.id).await?.as_deref() != Some(verified_subject)','if false','google_step_up'),
 ('google-owner-ignored',ctrl,'.filter(|user| user.id == flow_user_id)','.filter(|_| true)','google_step_up'),
 ('google-reusable','crates/campfire/src/concerns/session_keys.rs','session.remove(REAUTH_KEY)','session.get(REAUTH_KEY).cloned()','google_step_up'),
 ('user-limiter-removed',ctrl,'RateLimit::new(scope, 10, SignedDuration::from_mins(15))','RateLimit::new(scope, 100, SignedDuration::from_mins(15))','challenge_lockouts'),
 ('configured-lockout-not-enqueued',domain,'campfire_mail::jobs::lockout_notice_later(tx, user_id);','// deliberately omitted','configured_lockout'),
]
for name,path,needle,replacement,test in cases:
 source=ROOT/path;original=source.read_text();pattern=r'\s*'.join(re.escape(p) for p in needle.split());matched=re.search(pattern,original)
 if matched is None:raise RuntimeError(f'{name}: mutation anchor disappeared')
 try:
  source.write_text(original[:matched.start()]+replacement+original[matched.end():])
  result=subprocess.run(['cargo','test','--locked','-j','4','-p','campfire','app::challenge_tests::'+test],cwd=ROOT,env=dict(os.environ,TMPDIR=str(ROOT.parent/'.scratch/tmp')),stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  (OUT/(name+'.log')).write_text(result.stdout);summaries=[s for s in result.stdout.splitlines() if s.startswith('test result:')]
  if result.returncode==0 or not any('FAILED' in s for s in summaries):raise RuntimeError(f'{name}: no real failing test; see {OUT}')
  print(name+': '+summaries[-1],flush=True)
 finally:source.write_text(original)
print(f'WS9 challenge security gates: {len(cases)} deliberate defects rejected',flush=True)
