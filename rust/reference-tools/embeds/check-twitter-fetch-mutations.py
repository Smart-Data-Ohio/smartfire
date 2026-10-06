#!/usr/bin/env python3
"""Compile media-host, streaming-cap and strict-claim defects against the real domain/client."""
from pathlib import Path
import os
import subprocess
root=Path(__file__).resolve().parents[2]
env=os.environ.copy()
env.update(CI='1',TMPDIR=str(root.parent/'.scratch'),CARGO_TARGET_DIR=str(root/'.scratch/target'),GITHUB_TEST_PORT_RANGE='51550-51594',CABLE_TEST_PORT_RANGE='51550-51594')
mutants=[
 ('crates/campfire/src/integrations/twitter/fetcher.rs','parsed.host.as_deref(),','Some("pbs.twimg.com"),','campfire','ws15e_x_fetch_matches_pinned_persisted_values_over_verified_tls'),
 ('crates/campfire/src/integrations/twitter/fetcher.rs','response.read_body(MAX_BODY_BYTES)','response.read_body(usize::MAX)','campfire','ws15e_x_fetch_caps_header_streamed_and_decoded_bodies'),
 ('crates/campfire/src/integrations/twitter/post.rs','fetch_requested_at<?','fetch_requested_at<=?','campfire','ws15e_x_post_identity_fetch_windows_and_quiet_claims'),
]

for index,(file,old,new,package,test) in enumerate(mutants,1):
 path=root/file
 original=path.read_text();assert original.count(old)>=1,old
 try:
  path.write_text(original.replace(old,new))
  run=subprocess.run(['cargo','test','-j','4','-p',package,test,'--','--nocapture'],cwd=root,env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  (root.parent/'.scratch'/f'twitter-fetch-mutation-{index}.log').write_text(run.stdout)
  assert run.returncode and 'test result: FAILED' in run.stdout,run.stdout[-2500:]
  print(f'{index} {file} {test}: '+next(line for line in run.stdout.splitlines() if line.startswith('test result:')),flush=True)
 finally:path.write_text(original)
print(f'WS15e X fetch mutation checks: {len(mutants)} detected, 0 survived')
