#!/usr/bin/env python3
"""Run the real HTTP concurrency regression 50 times at CI's four-test concurrency."""
import concurrent.futures,json,os,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[3]
env=dict(os.environ,CI='1',TMPDIR=str(root/'.scratch'),CARGO_BUILD_JOBS='2',CARGO_INCREMENTAL='0',
 CARGO_PROFILE_DEV_DEBUG='line-tables-only',CARGO_PROFILE_TEST_DEBUG='line-tables-only')
built=subprocess.run(['cargo','test','--locked','--manifest-path','rust/Cargo.toml','-p','campfire','--bin','campfire','--no-run','--message-format=json'],cwd=root,env=env,text=True,stdout=subprocess.PIPE,check=True)
executables=[json.loads(line)['executable'] for line in built.stdout.splitlines() if line.startswith('{') and json.loads(line).get('executable') and json.loads(line).get('profile',{}).get('test')]
assert len(executables)==1,executables
name='app::tests::concurrent_message_posts_all_complete'
def run(n):
 p=subprocess.run([executables[0],name,'--exact','--test-threads=4','--nocapture'],cwd=root/'rust/crates/campfire',env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
 return n,p.returncode,p.stdout
failed=[]
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as workers:
 for n,code,log in workers.map(run,range(1,51)):
  print(f'WS11 concurrent HTTP posts repetition {n}/50; exit={code}',flush=True)
  print(log,flush=True)
  if code:failed.append(n)
print(f'WS11 concurrent HTTP posts: {50-len(failed)}/50 passed; test-process concurrency=4; libtest threads=4; Tokio workers=8; timeout unchanged at 60s',flush=True)
raise SystemExit(bool(failed))
