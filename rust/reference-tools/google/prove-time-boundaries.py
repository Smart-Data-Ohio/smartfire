#!/usr/bin/env python3
"""Reject equality mutations using committed Rails differentials; always restore sources."""
import argparse, os, subprocess
from pathlib import Path
parser = argparse.ArgumentParser()
parser.add_argument('--root',type=Path,default=Path(__file__).resolve().parents[2])
parser.add_argument('--only',default='')
args=parser.parse_args();root=args.root.resolve()
scratch=root.parent/'.scratch/ws14g/time-mutations';scratch.mkdir(parents=True,exist_ok=True)
cases=[
 ('account-expiry','crates/db/src/models/google_account.rs','|at| at <= now','|at| at < now','google_access_expiry_boundaries'),
 ('client-expiry','crates/campfire/src/integrations/google/api.rs','|t| t <= now.jiff()','|t| t < now.jiff()','google_access_expiry_boundaries'),
 ('flow-expiry','crates/campfire/src/integrations/google/sign_in.rs','expiry > i128::from(now.as_second())','expiry >= i128::from(now.as_second())','google_flow_expiry_boundaries'),
 ('channel-renewal','crates/campfire/src/integrations/google/calendar.rs','t <= now.since(jiff::SignedDuration::from_hours(24))','t < now.since(jiff::SignedDuration::from_hours(24))','google_channel_renewal_boundary'),
 ('meeting-throttle','crates/campfire/src/integrations/google/meeting_refresh.rs','t > now.ago(jiff::SignedDuration::from_secs(60))','t >= now.ago(jiff::SignedDuration::from_secs(60))','google_meeting_throttle_and_followup_boundaries'),
 ('followup-claim','crates/db/src/models/google_meeting_cache.rs','refresh_pending_at<=?','refresh_pending_at<?','google_meeting_throttle_and_followup_boundaries'),
 ('drive-cache','crates/campfire/src/integrations/google/drive.rs','*expiry > now','*expiry >= now','google_drive_metadata_expires_after_five_minutes'),
 ('key-cache','crates/campfire/src/integrations/google/sign_in.rs','*at > now - KEY_TTL','*at >= now - KEY_TTL','key_cache_expires_at_one_hour'),
 ('connection-id-token','crates/campfire/src/integrations/google/api.rs','integer(&v["exp"]) <= now.as_second()','integer(&v["exp"]) < now.as_second()','google_api_connection_urls_and_id_token_claims'),
]
checked=0
for name,relative,before,after,test in cases:
 if args.only and name not in args.only.split(','):continue
 path=root/relative;source=path.read_text();assert source.count(before)==1,(name,source.count(before))
 try:
  path.write_text(source.replace(before,after))
  env=dict(os.environ,CI='1',TMPDIR=str(scratch),CARGO_BUILD_JOBS='2',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',RUST_TEST_THREADS='8')
  run=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j','2','-p','campfire',test,'--','--nocapture','--test-threads=8'],cwd=root,env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  (scratch/f'{name}.log').write_text(run.stdout)
  summaries=[line for line in run.stdout.splitlines() if line.startswith('test result:')]
  assert run.returncode==101 and 'panicked at' in run.stdout and any('FAILED' in l for l in summaries),run.stdout
  print(name+': '+summaries[-1],flush=True);checked+=1
 finally:path.write_text(source)
print(f'Google time-boundary discrimination: {checked} mutations rejected',flush=True)
