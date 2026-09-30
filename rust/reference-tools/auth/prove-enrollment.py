#!/usr/bin/env python3
"""Reject enrollment defects against the real seeded HTTP stack, then restore each source."""
from pathlib import Path
import os, re, subprocess
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT.parent/'.scratch/enrollment-mutations'; OUT.mkdir(parents=True,exist_ok=True)
ctrl='crates/campfire/src/controllers/two_factor.rs'
cases=[
 ('session-left-unverified',ctrl,'session.mark_two_factor_verified(tx)?;','// deliberately omitted','enrollment_confirms'),
 ('user-limit-not-enforced',ctrl,'RateLimit::new(scope, 10, SignedDuration::from_mins(15))','RateLimit::new(scope, 100, SignedDuration::from_mins(15))','enrollment_user_limit'),
 ('accept-wrong-code',ctrl,'if !credential.confirm_with_setup_secret(\n                tx,\n                &ArEncryption::new(&secrets),\n                &setup,\n                &code,\n            )?','if { let _ = (&mut credential, &setup, &code); false }','enrollment_is_bound'),
 ('expired-setup-accepted','crates/db/src/models/two_factor.rs','self.expires_at <= now','{ let _ = now; false }','enrollment_is_bound'),
 ('old-sessions-survive',ctrl,'.filter(|s| s.id != session_id)','.filter(|_| false)','enrollment_confirms'),
 ('no-cache-protection',ctrl,'    c.no_store();','    // deliberately omitted','enrollment_reuses'),
 ('setup-not-session-bound',ctrl,'TwoFactorSetupSecret::valid_for(tx.conn(), session_id, tx.now())? else','TwoFactorSetupSecret::valid_for(tx.conn(), tx.conn().query_row("SELECT t.session_id FROM two_factor_setup_secrets t JOIN sessions s ON s.id=t.session_id WHERE s.user_id=? AND t.expires_at > ? ORDER BY t.id LIMIT 1", rusqlite::params![user.id,tx.now()], |r| r.get(0))?, tx.now())? else','enrollment_is_bound'),
]
for name,path,needle,replacement,test in cases:
 source=ROOT/path; original=source.read_text()
 pattern = r'\s*'.join(re.escape(part) for part in needle.split())
 matched = re.search(pattern, original)
 if matched is None: raise RuntimeError(f'{name}: anchor disappeared')
 try:
  source.write_text(original[:matched.start()]+replacement+original[matched.end():])
  result=subprocess.run(['cargo','test','--locked','-j','4','-p','campfire','app::two_factor_tests::'+test],cwd=ROOT,env=dict(os.environ,TMPDIR=str(ROOT.parent/'.scratch/tmp')),text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  (OUT/(name+'.log')).write_text(result.stdout)
  summaries=[s for s in result.stdout.splitlines() if s.startswith('test result:')]
  if result.returncode==0 or not any('FAILED' in s for s in summaries): raise RuntimeError(f'{name}: no real failing test; inspect {OUT}')
  print(name+': '+summaries[-1],flush=True)
 finally: source.write_text(original)
print(f'WS9 enrollment security gates: {len(cases)} deliberate defects rejected',flush=True)
