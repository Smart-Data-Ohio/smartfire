#!/usr/bin/env python3
"""Original browser assertions against the Rust app, with isolated fixtures and actual DB checks."""
import argparse,atexit,hashlib,json,os,re,shutil,sqlite3,subprocess,tempfile,time,urllib.request
from pathlib import Path
from browser_port_leases import DEFAULT_BASE, reserve
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('mode',choices=['people','pickers','members','group','tours','stars','worker'])
parser.add_argument('--controls',action='store_true')
parser.add_argument('--mutation',choices=['tour-stamp','tour-auto-start','group-notice','group-navigation','member-visibility','menu-rendered','identity-visibility','picker-visibility','picker-scope','tour-key-scope','phone-first-row'])
args=parser.parse_args()
root=Path(__file__).resolve().parents[2]
labels=json.loads((root/'parity/.seed/default/labels.json').read_text())
cases={
'people':['author','author-keyboard','closed-escape','sidebar-keyboard','directory-three','member-huddle','shift-range','long-press','agent','mixed'],
'pickers':['picker-filter','picker-post','picker-enter','picker-row','picker-phone'],
'members':['mobile-escape','mobile-tab'],'group':['group-lifecycle'],
'tours':['tour-finish','tour-escape','tour-restart','tour-completed'],
'stars':['star-card','star-menu','star-escape','star-phone'],'worker':['served-worker'],}
if args.mutation:
 mutation_cases={'tour-stamp':('tours','tour-restart'),'tour-auto-start':('tours','tour-completed'),'group-notice':('group','group-lifecycle'),'group-navigation':('group','group-lifecycle'),'member-visibility':('members','mobile-tab'),'menu-rendered':('stars','star-menu'),'identity-visibility':('stars','star-card'),'picker-visibility':('pickers','picker-filter'),'picker-scope':('pickers','picker-filter'),'tour-key-scope':('tours','tour-finish'),'phone-first-row':('pickers','picker-phone')}
 mutation_mode,mutation_case=mutation_cases[args.mutation]
 assert args.mode==mutation_mode,'mutation must use its original mode'
 cases[args.mode]=[mutation_case]
# The lease survives all startup work. Its kernel names coordinate independent
# invocations/worktrees; outbound forwarder connections cannot consume this range.
lease=reserve(int(os.environ.get('WS11UI_ORIGINAL_PORT',str(DEFAULT_BASE))),list(cases).index(args.mode))
atexit.register(lease.close)
ports=lease.ports
print(f'ORIGINAL_PORT_LEASE {args.mode}: {ports}',flush=True)
scratch=root/'.scratch';scratch.mkdir(exist_ok=True)
run=Path(tempfile.mkdtemp(prefix='ws11ui-originals-',dir=scratch))
shutil.copytree(root/'parity/.seed/default/db',run/'db');shutil.copytree(root/'parity/.seed/default/storage',run/'files')
database=run/'db/production.sqlite3'
with sqlite3.connect(database) as db:
 # Original fixture list, before the parity seed's added deployment agent.
 db.execute("UPDATE users SET status=1 WHERE name='Deploy Bot'")
 db.execute('DELETE FROM workspace_presence_leases')
 # This unrelated fixture preview would dial an external image origin. Neither
 # the original assertions nor these paths exercise previews; an inert body
 # keeps offline transport failures out of the controls.
 db.execute("UPDATE action_text_rich_texts SET body='<div>Offline preview fixture</div>' WHERE body LIKE '%pbs.twimg.com/profile_images%'")
 for user in ['david','jz','jason','kevin']:db.execute('UPDATE users SET tour_completed_at=? WHERE id=?',[labels['clock.now'],labels['users.'+user]])
 if args.mode=='pickers':
  for id,name,email in [(9100000001,'Chad Puterbaugh','chad@example.test'),(9100000002,'Renée Dupont','renee@example.test')]:
   db.execute("INSERT INTO users(id,name,email_address,created_at,updated_at) VALUES (?,?,?,'2026-03-02 16:00:00','2026-03-02 16:00:00')",[id,name,email])
  labels.update({'users.chad':9100000001,'users.renee':9100000002})
(run/'labels.json').write_text(json.dumps(labels))
env=os.environ.copy()
for line in (root/'parity/.env.reference').read_text().splitlines():
 if line and not line.startswith('#') and '=' in line:k,v=line.split('=',1);env[k]=v
env.update({k.removeprefix('reference_env.'):str(v) for k,v in labels.items() if k.startswith('reference_env.')})
env.update(CAMPFIRE_STORAGE_PATH=str(run),CAMPFIRE_FROZEN_TIME=labels['clock.now'],DISABLE_SSL='1',HTTP_PORT=str(ports[1]),TARGET_PORT=str(ports[2]),RAILS_LOG_LEVEL='warn')
env.pop('TLS_DOMAIN',None)
hash=hashlib.sha256(b''.join((root/'parity'/n).read_bytes() for n in ['Dockerfile.playwright','package.json','package-lock.json'])).hexdigest()[:12]
image='ws12-playwright:'+hash
if subprocess.run(['docker','image','inspect',image],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode:
 image='ws8br2-playwright:'+hash
subprocess.run(['docker','image','inspect',image],check=True,stdout=subprocess.DEVNULL)
binary=Path(os.environ.get('WS11UI_BROWSER_BINARY',str(Path(os.environ.get('CARGO_TARGET_DIR',str(root/'target')))/'debug/campfire')))
server=None
net_root=scratch/'originals-network';net_root.mkdir(parents=True,exist_ok=True)
net_dir=Path(tempfile.mkdtemp(prefix='net-',dir=net_root));sock=net_dir/'upstream.sock'
# Relative socket paths avoid Linux's 108-byte Unix-address limit in deep worktrees.
forward=subprocess.Popen(['node',str(root/'parity/capture/forward.ts'),str(sock.relative_to(root))],cwd=root,stdout=subprocess.DEVNULL)
deadline=time.monotonic()+10
while not sock.exists():
 if forward.poll() is not None or time.monotonic()>deadline:raise RuntimeError('forwarder did not start')
 time.sleep(.02)
proof=[]
def fixture(case):
 with sqlite3.connect(database) as db:
  if args.mode=='tours':db.execute('UPDATE users SET tour_completed_at=? WHERE id=?',[labels['clock.now'] if case=='tour-completed' else None,labels['users.jz']])
  if args.mode=='stars':db.execute('DELETE FROM user_stars WHERE user_id=?',[labels['users.jz']])
def check_state(requests):
 target='Rust'
 with sqlite3.connect(database) as db:
  for request in requests:
   if request['kind']=='direct':
    row=db.execute("SELECT type FROM rooms WHERE id=?",[request['room']]).fetchone()
    assert row==('Rooms::Direct',),(target,request,row)
    actual=sorted(r[0] for r in db.execute('SELECT user_id FROM memberships WHERE room_id=?',[request['room']]))
    expected=sorted(labels['users.'+u] for u in request['users'])
    assert actual==expected,(target,'exact persisted group recipients',actual,expected)
    # Rooms::Direct.find_for first uses this exact canonical member key.
    key='dm:'+hashlib.sha256(','.join(map(str,expected)).encode()).hexdigest()
    found=db.execute("SELECT id FROM rooms WHERE type='Rooms::Direct' AND deleted_at IS NULL AND direct_member_key=?",[key]).fetchone()
    assert found==(request['room'],),(target,'original current path resolves canonical member set',found,request)
    print(f'ORIGINAL_PATH {target}: canonical group {request["room"]}',flush=True)
   elif request['kind']=='left':
    actual=[r[0] for r in db.execute('SELECT user_id FROM memberships WHERE room_id=?',[request['room']])]
    assert labels['users.david'] not in actual,(target,'original leaver absent',actual)
   elif request['kind']=='tour':
    marker=db.execute('SELECT tour_completed_at FROM users WHERE id=?',[labels['users.'+request['user']]]).fetchone()[0]
    assert marker is not None,(target,'original persisted tour stamp',request)
   else:raise AssertionError(request)
   print(f'ORIGINAL_DB {target}: {json.dumps(request,sort_keys=True)}',flush=True)
try:
 with (run/'rust-server.log').open('w') as log:
  server=subprocess.Popen([str(binary),'server'],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT)
  deadline=time.monotonic()+30
  while True:
   if server.poll() is not None:raise RuntimeError('Rust server exited: '+(run/'rust-server.log').read_text())
   try:urllib.request.urlopen(f'http://127.0.0.1:{ports[1]}/up',timeout=1).close();break
   except OSError:
    if time.monotonic()>deadline:raise RuntimeError('Rust server failed startup')
    time.sleep(.25)
  for case in cases[args.mode]:
   fixture(case)
   script='original_worker_assertions.mjs' if args.mode=='worker' else 'original_browser_assertions.mjs'
   cmd=['docker','run','--rm','--network','none','--cpus','1','--shm-size','256m','-v',f'{net_dir}:/upstream','-e','PARITY_UPSTREAM_SOCKET=/upstream/upstream.sock','-e',f'WS11UI_HOST_NETWORK={os.readlink("/proc/self/ns/net")}', '--label','parity.owner=ws11ui','-v',f'{root.parent}:/work:ro','-e',f'WS11UI_BROWSER_URL=http://127.0.0.1:{ports[1]}','-e',f'WS11UI_BROWSER_LABELS=/work/{(run/"labels.json").relative_to(root.parent)}','-e',f'WS11UI_BROWSER_DATABASE=/work/{database.relative_to(root.parent)}','-e',f'WS11UI_BROWSER_CASE={case}','-e',f'WS11UI_BROWSER_MODE={args.mode}','-e',f'WS11UI_BROWSER_CONTROL={int(args.controls)}','-e',f'WS11UI_BROWSER_MUTATION={args.mutation or ""}',image,'node','/work/rust/reference-tools/users/'+script]
   result=subprocess.run(cmd,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=90)
   print(f'Original Rust {case}:',flush=True);print(result.stdout,flush=True)
   if args.controls:
    assert result.returncode!=0 and 'ORIGINAL_MUTATION' in result.stdout and 'INVALID_CONTROL' not in result.stdout,(case,'invalid/surviving producer control',result.stdout[-2000:])
    assert ('original assertion' in result.stdout or 'TimeoutError' in result.stdout or (args.mode=='worker' and 'only static entries are ever cached' in result.stdout)),(case,'wrong failure')
   else:
    assert result.returncode==0 and f'ORIGINAL_CASE {case}: passed' in result.stdout,(case,'browser receipt missing')
    requests=json.loads(next(s.removeprefix('ORIGINAL_STATE_REQUESTS ') for s in result.stdout.splitlines() if s.startswith('ORIGINAL_STATE_REQUESTS ')))
    check_state(requests)
   proof.append(case)
 print(f'Original browser {args.mode}: {len(proof)} '+('producer defects rejected; 0 invalid controls' if args.controls else 'Rust case executions passed; 0 failures'),flush=True)
finally:
 if server:
  server.terminate()
  try:server.wait(timeout=15)
  except subprocess.TimeoutExpired:server.kill();server.wait()
 forward.terminate();forward.wait(timeout=5)
 shutil.rmtree(net_dir)
 if os.environ.get('WS11UI_KEEP_BROWSER_SCRATCH')!='1':shutil.rmtree(run)
