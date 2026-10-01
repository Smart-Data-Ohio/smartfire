#!/usr/bin/env python3
"""Named Chromium scenarios against the real app and a private copy of the pinned seed."""
import os, shutil, subprocess, tempfile, time, urllib.request
from pathlib import Path
root=Path(__file__).resolve().parents[2]; scratch=root/'.scratch'; scratch.mkdir(exist_ok=True)
subprocess.run(['mise','exec','rust@1.98.1','--','cargo','build','--locked','-j','4','--manifest-path','rust/Cargo.toml','-p','campfire'],cwd=root,check=True)
if not (root/'rust/parity/node_modules/playwright').is_dir(): subprocess.run(['npm','ci','--prefix','rust/parity'],cwd=root,check=True)
with tempfile.TemporaryDirectory(prefix='browser-',dir=scratch) as tmp:
 storage=Path(tmp); (storage/'db').mkdir(); shutil.copy2(root/'rust/parity/.seed/default/db/production.sqlite3',storage/'db/production.sqlite3'); shutil.copytree(root/'rust/parity/.seed/default/storage',storage/'files')
 env=dict(os.environ,GOOGLE_CLIENT_ID='test-client-id',GOOGLE_CLIENT_SECRET='test-client-secret',DISABLE_SSL='true',HTTP_PORT='52471',TARGET_PORT='52471',APP_VERSION='parity',GIT_REVISION='parity',CAMPFIRE_FROZEN_TIME='2026-03-02T16:00:00Z',CAMPFIRE_STORAGE_PATH=str(storage),EVENT_REMINDERS_INTERVAL='invalid',HUDDLE_RECONCILE_INTERVAL='invalid')
 for line in (root/'rust/parity/.env.reference').read_text().splitlines():
  if line.startswith('SECRET_KEY_BASE='): env['SECRET_KEY_BASE']=line.split('=',1)[1]
 with (scratch/'browser-server.log').open('w') as log:
  server=subprocess.Popen([str(root/'rust/target/debug/campfire')],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT)
  try:
   for attempt in range(100):
    if server.poll() is not None: raise RuntimeError('browser server exited; see .scratch/browser-server.log')
    try:
     urllib.request.urlopen('http://127.0.0.1:52471/offline.html',timeout=1).close(); break
    except OSError: time.sleep(.1)
   else: raise RuntimeError('browser server did not start')
   subprocess.run(['node','rust/reference-tools/ws17_browser.mjs',str(storage/'db/production.sqlite3')],cwd=root,env=env,check=True)
  finally:
   server.terminate()
   try: server.wait(timeout=10)
   except subprocess.TimeoutExpired: server.kill();server.wait()
