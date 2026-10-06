#!/usr/bin/env python3
"""Remaining original declarations against the Rust app, using the original browser transport and port leases.

Mode modules own case fixture hooks; the browser sees only its isolated database
writable for the original direct fixture operations, never the repository.
"""
import argparse,atexit,hashlib,importlib,json,os,shutil,sqlite3,subprocess,tempfile,time,urllib.request
from pathlib import Path
from browser_port_leases import DEFAULT_BASE, reserve
from ledger_browser_viewports import prepare_viewports
from ledger_fixture_state import snapshot, restore
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('mode',choices=['navigation','members','surfaces','lifecycle'])
parser.add_argument('--case', help='Run only this exact original declaration')
parser.add_argument('--from-case', help='Construction probe from an exact declaration; final receipts require all cases')
parser.add_argument('--controls',action='store_true')
parser.add_argument('--mutation', help='Named producer defect; negative control')
args=parser.parse_args()
root=Path(__file__).resolve().parents[2]
module=importlib.import_module('ledger_browser_'+args.mode)
cases={args.mode:list(module.CONTROL_CASES if args.controls and hasattr(module,'CONTROL_CASES') else module.CASES)}
if args.from_case:
 assert args.from_case in cases[args.mode]
 cases[args.mode]=cases[args.mode][cases[args.mode].index(args.from_case):]
if args.case:
 assert args.case in cases[args.mode], 'unknown original case'
 cases[args.mode]=[args.case]
if args.mutation:
 assert args.case, 'mutations must name their original declaration'
# The lease survives all startup work. Its kernel names coordinate independent
# invocations/worktrees; outbound forwarder connections cannot consume this range.
lease=reserve(int(os.environ.get('WS11UI_ORIGINAL_PORT',str(DEFAULT_BASE))),16+['navigation','members','surfaces','lifecycle'].index(args.mode))
atexit.register(lease.close)
ports=lease.ports
print(f'ORIGINAL_PORT_LEASE {args.mode}: {ports}',flush=True)
scratch=root/'.scratch';scratch.mkdir(exist_ok=True)
run=Path(tempfile.mkdtemp(prefix='ws11ui-originals-',dir=scratch))
# Each wrapper owns a fresh copy of the original fixture database (recorded from
# Rails' fixture load). The visual default seed is deliberately not a starting
# state for these declarations.
frozen=root/'parity/.seed/ledger_originals'
if not (frozen/'db/production.sqlite3').is_file():
 shutil.rmtree(run);raise SystemExit(f'missing {frozen}: run python3 parity/bin/frozen-seeds restore')
seed=run/'seed';shutil.copytree(frozen,seed)
labels=json.loads((seed/'labels.json').read_text())
with sqlite3.connect(seed/'db/production.sqlite3') as db:
 db.execute('DELETE FROM workspace_presence_leases')
 if hasattr(module,'prepare'):module.prepare(db,labels,root)
prepare_viewports(labels,root)
(seed/'labels.json').write_text(json.dumps(labels))
shutil.copytree(seed/'db',run/'db');shutil.copytree(seed/'storage',run/'files')
database=run/'db/production.sqlite3'
env=os.environ.copy()
for line in (root/'parity/.env.reference').read_text().splitlines():
 if line and not line.startswith('#') and '=' in line:k,v=line.split('=',1);env[k]=v
env.update({k.removeprefix('reference_env.'):str(v) for k,v in labels.items() if k.startswith('reference_env.')})
env.update(RAILS_ENV='test',CAMPFIRE_STORAGE_PATH=str(run),CAMPFIRE_FROZEN_TIME=labels['clock.now'],DISABLE_SSL='1',HTTP_PORT=str(ports[1]),TARGET_PORT=str(ports[2]),RAILS_LOG_LEVEL='warn')
env.pop('TLS_DOMAIN',None)
if hasattr(module,'environment'):env.update(module.environment(labels,root))
env.update(CAMPFIRE_DATABASE_PATH=str(database),CAMPFIRE_FILES_PATH=str(run/'files'))
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
original_state=None
fixture_sequence=0
def host_request(action,check,timeout_message):
 """One file-IPC fixture action, answered by the tools-only Rust host."""
 global fixture_sequence
 directory=database.parent
 previous=directory/'ledger-request.json'
 current=json.loads(previous.read_text())['sequence'] if previous.exists() else 0
 fixture_sequence=max(fixture_sequence,current)+1
 temporary=directory/'ledger-request.tmp'
 temporary.write_text(json.dumps({'sequence':fixture_sequence,'action':action}))
 temporary.replace(previous)
 deadline=time.monotonic()+15
 while True:
  try:
   response=json.loads((directory/'ledger-response.json').read_text())
   if response['sequence']==fixture_sequence:
    assert check(response.get('result',{})),response;return
  except FileNotFoundError:pass
  if time.monotonic()>deadline:raise RuntimeError(timeout_message)
  time.sleep(.02)
def fixture(case):
 with sqlite3.connect(database) as db:
  restore(db,original_state)
  if hasattr(module,'fixture'):module.fixture(db,case,labels,root)
 host_request('reset-fixtures',lambda result:result.get('reset') is True,'original fixture cache reset timed out')
 # Original test.rb defaults off; only these class/case setups enable it.
 enabled=case.startswith('channel-') or case.startswith('icon-') or case in {'workspace-icons','timezone-once'}
 host_request('forgery-on' if enabled else 'forgery-off',lambda result:result.get('disabled') is (not enabled),'original forgery setup timed out')
 print(f'ORIGINAL_FORGERY Rust {case}: {enabled}',flush=True)
 print(f'ORIGINAL_FIXTURE_RESET Rust {case}: all original tables/FTS/sequences and renderer cache',flush=True)
def check_state(requests):
 with sqlite3.connect(database) as db:
  if hasattr(module,'check_state'):module.check_state(db,requests,labels,root)
  else:assert not requests, ('unvalidated original state requests', requests)
 for request in requests:print(f'ORIGINAL_DB Rust: {json.dumps(request,sort_keys=True)}',flush=True)
def stop_server():
 global server
 if server:
  server.terminate()
  try:server.wait(timeout=15)
  except subprocess.TimeoutExpired:server.kill();server.wait()
  server=None
def start_server(case,settings):
 global server,original_state
 stop_server()
 for destination,source in [(run/'db',seed/'db'),(run/'files',seed/'storage')]:
  shutil.rmtree(destination);shutil.copytree(source,destination)
 case_env=dict(env)
 # Ambient integrations must not appear in a class that did not configure them.
 for key in ['LIVEKIT_URL','LIVEKIT_INTERNAL_URL','LIVEKIT_API_KEY','LIVEKIT_API_SECRET','LIVEKIT_GATEWAY_SECRET','GOOGLE_CLIENT_ID','GOOGLE_CLIENT_SECRET','GOOGLE_PICKER_API_KEY','GOOGLE_CLOUD_PROJECT_NUMBER','LEDGER_BROWSER_CLOCK']:
  case_env.pop(key,None)
 case_env.update(settings)
 with (run/'rust-server.log').open('a') as log:
  command=module.host_command(binary,case_env,root) if hasattr(module,'host_command') else [str(binary),'server']
  server=subprocess.Popen(command,cwd=root,env=case_env,stdout=log,stderr=subprocess.STDOUT)
 deadline=time.monotonic()+30
 while True:
  if server.poll() is not None:raise RuntimeError('Rust server exited: '+(run/'rust-server.log').read_text())
  try:urllib.request.urlopen(f'http://127.0.0.1:{ports[1]}/up',timeout=1).close();break
  except OSError:
   if time.monotonic()>deadline:raise RuntimeError('Rust server failed startup')
   time.sleep(.25)
 if hasattr(module,'after_start_case'):module.after_start_case(database,root,case)
 with sqlite3.connect(database) as db:original_state=snapshot(db)
try:
 configuration=None
 for case in cases[args.mode]:
  provider=getattr(module,'environment_for_case',getattr(module,'case_environment',None))
  settings=provider(case,labels,root) if provider else {}
  if settings!=configuration:
   start_server(case,settings);configuration=settings
   print('ORIGINAL_SERVER_CONFIGURATION '+json.dumps(settings,sort_keys=True),flush=True)
  fixture(case)
  script='ledger_browser_'+args.mode+'.mjs'
  cmd=['docker','run','--rm','--network','none','--cpus','1','--shm-size','256m','-v',f'{net_dir}:/upstream','-e','PARITY_UPSTREAM_SOCKET=/upstream/upstream.sock','-e',f'WS11UI_HOST_NETWORK={os.readlink("/proc/self/ns/net")}', '--label','parity.owner=ws11ui','-v',f'{root}:/work:ro','-e',f'WS11UI_BROWSER_URL=http://127.0.0.1:{ports[1]}','-e',f'WS11UI_BROWSER_LABELS=/work/{(seed/"labels.json").relative_to(root)}','-v',f'{database.parent}:/database','-e','WS11UI_BROWSER_DATABASE=/database/production.sqlite3','-e',f'WS11UI_BROWSER_CASE={case}','-e',f'WS11UI_BROWSER_MODE={args.mode}','-e',f'WS11UI_BROWSER_CONTROL={int(args.controls)}','-e',f'WS11UI_BROWSER_MUTATION={args.mutation or ""}',image,'node','/work/reference-tools/users/'+script]
  result=subprocess.run(cmd,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=getattr(module,'CASE_TIMEOUT',{}).get(case,90))
  print(f'Original Rust {case}:',flush=True);print(result.stdout,flush=True)
  if args.controls or args.mutation:
   assert result.returncode!=0 and 'ORIGINAL_MUTATION' in result.stdout and 'INVALID_CONTROL' not in result.stdout,(case,'invalid/surviving producer control',result.stdout[-2000:])
   assert ('original assertion' in result.stdout or 'TimeoutError' in result.stdout),(case,'wrong failure')
  else:
   assert result.returncode==0 and f'ORIGINAL_CASE {case}: passed' in result.stdout,(case,'browser receipt missing')
   requests=json.loads(next(s.removeprefix('ORIGINAL_STATE_REQUESTS ') for s in result.stdout.splitlines() if s.startswith('ORIGINAL_STATE_REQUESTS ')))
   check_state(requests)
  proof.append(case)
 print(f'Original browser {args.mode}: {len(proof)} '+('producer defects rejected; 0 invalid controls' if (args.controls or args.mutation) else 'Rust case executions passed; 0 failures'),flush=True)
finally:
 stop_server()
 forward.terminate();forward.wait(timeout=5)
 shutil.rmtree(net_dir)
 if os.environ.get('WS11UI_KEEP_BROWSER_SCRATCH')!='1':shutil.rmtree(run)
