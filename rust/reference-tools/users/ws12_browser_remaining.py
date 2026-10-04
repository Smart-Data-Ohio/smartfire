#!/usr/bin/env python3
"""Replay all seven original browser declarations against frozen, seeded Rails."""
import pathlib,subprocess,os,hashlib,tempfile,time
root=pathlib.Path(__file__).resolve().parents[2];env=os.environ.copy();env.update(PARITY_RUNTIME='docker',PARITY_IMAGE=os.environ.get('PARITY_IMAGE','campfire-reference'),PARITY_NAMESPACE='ws12',PARITY_OWNER='ws12',PARITY_CPUS='1');env.pop('LD_LIBRARY_PATH',None)
ref=str(root/'parity/bin/reference');image='ws12-playwright:'+hashlib.sha256(b''.join((root/'parity'/n).read_bytes() for n in ['Dockerfile.playwright','package.json','package-lock.json'])).hexdigest()[:12]
cache=pathlib.Path(os.environ.get('XDG_CACHE_HOME',str(pathlib.Path.home()/'.cache')))/'rust-port/ws12';cache.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='browser-rails-',dir=cache) as temp:
 socket=pathlib.Path(temp)/'forward.sock';forward=subprocess.Popen(['node',str(root/'parity/capture/forward.ts'),str(socket)])
 try:
  for _ in range(80):
   if socket.exists():break
   time.sleep(.05)
  for i in range(221,228):
   key=f'c{i}'
   subprocess.run([ref,'up','--seed','default','--port','53410','--time','2026-03-02T16:00:00Z','--freeze','-e','DISABLE_SSL=1'],env=env,check=True)
   try:
    subprocess.run([ref,'exec','--port','53410','-e',f'WS12_BROWSER_CASE={key}','--','bin/rails','runner','--skip-executor','/work/reference-tools/users/ws12_browser_remaining.rb'],env=env,check=True)
    subprocess.run(['docker','run','--rm','--init','--network','none','--ipc','host','--cpus','1','-v',f'{root}:{root}:ro','-v',f'{temp}:{temp}','-e',f'PARITY_UPSTREAM_SOCKET={socket}','-e','WS12_BROWSER_TARGET=http://127.0.0.1:53410','-e',f'WS12_BROWSER_CASE={key}','-e',f'WS12_BROWSER_LABELS={root}/parity/.seed/default/labels.json',image,'node',str(root/'reference-tools/users/ws12_browser_remaining.mjs')],env=env,check=True)
   finally:subprocess.run([ref,'down','--port','53410'],env=env,check=True)
  print('WS12_BROWSER_RAILS 7 named declarations; 7 passed; 0 failed; 0 masks',flush=True)
 finally:
  forward.terminate();forward.wait(timeout=10)
