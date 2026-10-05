#!/usr/bin/env python3
"""Remaining original declarations using the original browser transport and port leases.

Mode modules own case fixture hooks; the browser sees only its isolated database
writable for the original direct fixture operations, never the repository.
"""
import argparse,atexit,hashlib,importlib,json,os,re,shutil,sqlite3,subprocess,tempfile,time,urllib.request
from pathlib import Path
from browser_port_leases import DEFAULT_BASE, reserve
from reference_runtime import ReferenceNetwork
from ledger_browser_viewports import prepare_viewports
from ledger_fixture_state import snapshot, restore
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('mode',choices=['navigation','members','surfaces','lifecycle'])
parser.add_argument('--case', help='Run only this exact original declaration')
parser.add_argument('--from-case', help='Construction probe from an exact declaration; final receipts require all cases')
parser.add_argument('--controls',action='store_true')
parser.add_argument('--mutation', help='Named producer defect; Rust-only negative control')
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
seed=root/'parity/.seed'/run.name
# Each wrapper owns a freshly loaded original fixture database. The visual
# default seed is deliberately not a starting state for these declarations.
seed_env=os.environ.copy();seed_env.update(PARITY_SEED_DIR=str(run/'seed-inputs'),PARITY_NAMESPACE='ws11ui-original-fixtures',PARITY_OWNER='ws11ui',PARITY_RUNTIME='docker',PARITY_IMAGE=os.environ.get('PARITY_IMAGE','campfire-reference'))
try:
 subprocess.run([str(root/'parity/bin/seed'),'build','ledger_originals'],env=seed_env,check=True,stdout=subprocess.DEVNULL)
 shutil.move(run/'seed-inputs/ledger_originals',seed)
except BaseException:
 shutil.rmtree(run);raise
labels=json.loads((seed/'labels.json').read_text())
with sqlite3.connect(seed/'db/production.sqlite3') as db:
 db.execute('DELETE FROM workspace_presence_leases')
 if hasattr(module,'prepare'):module.prepare(db,labels,root)
prepare_viewports(labels,root)
(seed/'labels.json').write_text(json.dumps(labels))
shutil.copytree(seed/'db',run/'db');shutil.copytree(seed/'storage',run/'files')
env=os.environ.copy()
for line in (root/'parity/.env.reference').read_text().splitlines():
 if line and not line.startswith('#') and '=' in line:k,v=line.split('=',1);env[k]=v
env.update({k.removeprefix('reference_env.'):str(v) for k,v in labels.items() if k.startswith('reference_env.')})
env.update(RAILS_ENV='test',DATABASE_URL='sqlite3:/rails/storage/db/production.sqlite3',CAMPFIRE_STORAGE_PATH=str(run),CAMPFIRE_FROZEN_TIME=labels['clock.now'],DISABLE_SSL='1',HTTP_PORT=str(ports[1]),TARGET_PORT=str(ports[2]),RAILS_LOG_LEVEL='warn')
env.pop('TLS_DOMAIN',None)
if hasattr(module,'environment'):env.update(module.environment(labels,root))
env.update(CAMPFIRE_DATABASE_PATH=str(run/'db/production.sqlite3'),CAMPFIRE_FILES_PATH=str(run/'files'))
oracle=os.environ.copy();oracle.pop('LD_LIBRARY_PATH',None)
oracle.update(PARITY_NAMESPACE='ws11ui-originals',PARITY_OWNER='ws11ui',PARITY_RUNTIME='docker',PARITY_CPUS='1',PARITY_IMAGE=os.environ.get('PARITY_IMAGE','campfire-reference'))
owned_network=ReferenceNetwork(oracle['PARITY_NAMESPACE'],run.name,oracle['PARITY_OWNER'])
oracle['PARITY_NETWORK']=owned_network.name
reference=str(root/'parity/bin/reference')
if hasattr(module,'reference_image'):oracle['PARITY_IMAGE']=module.reference_image(oracle['PARITY_IMAGE'],root)
# Install original cache cleanup in the renderer, not in the separate fixture
# runner process. Only the isolated private generation file can trigger it.
cache_initializer=root/'reference-tools/users/ledger_fixture_cache_initializer.rb'
cache_base=oracle['PARITY_IMAGE']
cache_base_id=subprocess.check_output(['docker','image','inspect','--format','{{.Id}}',cache_base],text=True).strip()
rpc_inputs=[cache_initializer,root/'reference-tools/users/ledger_fixture_actions.rb',root/'reference-tools/users/ledger_fixture_rpc_initializer.rb',root/'reference-tools/users/original-test-session-controller.rb']
cache_image='ledger-fixture-cache:'+hashlib.sha256(b''.join(p.read_bytes() for p in rpc_inputs)+cache_base_id.encode()).hexdigest()[:12]
if subprocess.run(['docker','image','inspect',cache_image],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode:
 dockerfile='ARG BASE\nFROM ${BASE}\nCOPY ledger_fixture_cache_initializer.rb /rails/config/initializers/ledger_fixture_cache.rb\nCOPY ledger_fixture_actions.rb /rails/lib/ledger_fixture_actions.rb\nCOPY ledger_fixture_rpc_initializer.rb /rails/config/initializers/ledger_fixture_rpc.rb\nCOPY original-test-session-controller.rb /rails/test/support/test_session_controller.rb\n'
 subprocess.run(['docker','build','--build-arg','BASE='+cache_base,'-f','-','-t',cache_image,str(cache_initializer.parent)],input=dockerfile,text=True,check=True,stdout=subprocess.DEVNULL)
oracle['PARITY_IMAGE']=cache_image
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
original_state={}
fixture_sequence=0
def db_path(target):return run/'db/production.sqlite3' if target=='Rust' else root/f'parity/.seed/.instances/{ports[0]}/db/production.sqlite3'
def fixture(target,case):
 global fixture_sequence
 with sqlite3.connect(db_path(target)) as db:
  restore(db,original_state[target])
  if hasattr(module,'fixture'):module.fixture(db,case,labels,root)
 directory=db_path(target).parent
 previous=directory/'ledger-request.json'
 current=json.loads(previous.read_text())['sequence'] if previous.exists() else 0
 fixture_sequence=max(fixture_sequence,current)+1
 (directory/'ledger-cache-generation').write_text(str(fixture_sequence))
 if target=='Rails':
  # This response runs the cache-reset middleware in the actual Puma renderer.
  urllib.request.urlopen(f'http://127.0.0.1:{ports[0]}/up',timeout=10).read()
  assert (directory/'ledger-cache-renderer-ack').read_text()==str(fixture_sequence), 'actual Rails renderer cache reset was not acknowledged'
 temporary=directory/'ledger-request.tmp'
 temporary.write_text(json.dumps({'sequence':fixture_sequence,'action':'reset-fixtures'}))
 temporary.replace(directory/'ledger-request.json')
 deadline=time.monotonic()+15
 while True:
  try:
   response=json.loads((directory/'ledger-response.json').read_text())
   if response['sequence']==fixture_sequence:
    assert response.get('result',{}).get('reset') is True,response;break
  except FileNotFoundError:pass
  if time.monotonic()>deadline:raise RuntimeError('original fixture cache reset timed out')
  time.sleep(.02)
 # Original test.rb defaults off; only these class/case setups enable it.
 enabled=case.startswith('channel-') or case.startswith('icon-') or case in {'workspace-icons','timezone-once'}
 fixture_sequence+=1
 temporary.write_text(json.dumps({'sequence':fixture_sequence,'action':'forgery-on' if enabled else 'forgery-off'}))
 temporary.replace(directory/'ledger-request.json')
 deadline=time.monotonic()+15
 while True:
  try:
   response=json.loads((directory/'ledger-response.json').read_text())
   if response['sequence']==fixture_sequence:
    assert response.get('result',{}).get('disabled') is (not enabled),response;break
  except FileNotFoundError:pass
  if time.monotonic()>deadline:raise RuntimeError('original forgery setup timed out')
  time.sleep(.02)
 print(f'ORIGINAL_FORGERY {target} {case}: {enabled}',flush=True)
 print(f'ORIGINAL_FIXTURE_RESET {target} {case}: all original tables/FTS/sequences and renderer cache',flush=True)
def check_state(target,requests):
 with sqlite3.connect(db_path(target)) as db:
  if hasattr(module,'check_state'):module.check_state(db,requests,labels,root)
  else:assert not requests, ('unvalidated original state requests', requests)
 for request in requests:print(f'ORIGINAL_DB {target}: {json.dumps(request,sort_keys=True)}',flush=True)
try:
 # Small lease-specific subnet: the shared host's default Docker pools are full.
 slot=(ports[0]-DEFAULT_BASE)//3
 subnet=f'10.251.{slot//32}.{(slot%32)*8}/29'
 subprocess.run(['docker','network','create','--internal','--subnet',subnet,'--label','parity.owner=ws11ui',owned_network.name],check=True,stdout=subprocess.DEVNULL)
 configuration=None
 def stop_pair():
  global server
  if hasattr(module,'stop'):module.stop()
  if server:
   server.terminate()
   try:server.wait(timeout=15)
   except subprocess.TimeoutExpired:server.kill();server.wait()
   server=None
  subprocess.run([reference,'down','--port',str(ports[0])],env=oracle,check=True)
 def start_pair(case,settings):
  stop_pair()
  for destination,source in [(run/'db',seed/'db'),(run/'files',seed/'storage')]:
   shutil.rmtree(destination);shutil.copytree(source,destination)
  case_env=dict(env)
  # Ambient integrations must not appear in a class that did not configure them.
  for key in ['LIVEKIT_URL','LIVEKIT_INTERNAL_URL','LIVEKIT_API_KEY','LIVEKIT_API_SECRET','LIVEKIT_GATEWAY_SECRET','GOOGLE_CLIENT_ID','GOOGLE_CLIENT_SECRET','GOOGLE_PICKER_API_KEY','GOOGLE_CLOUD_PROJECT_NUMBER','LEDGER_BROWSER_CLOCK']:
   case_env.pop(key,None)
  case_env.update(settings)
  reference_env=[]
  for key,value in {'RAILS_ENV':'test','DATABASE_URL':'sqlite3:/rails/storage/db/production.sqlite3',**settings}.items():reference_env.extend(['-e',key+'='+str(value)])
  subprocess.run([reference,'up','--seed',seed.name,'--port',str(ports[0]),'--time',labels['clock.now'],'--freeze','-e','DISABLE_SSL=1',*reference_env],env=oracle,check=True,timeout=75)
  with (run/'rust-server.log').open('a') as log:
   global server
   command=module.host_command(binary,case_env,root) if hasattr(module,'host_command') else [str(binary),'server']
   server=subprocess.Popen(command,cwd=root,env=case_env,stdout=log,stderr=subprocess.STDOUT)
  deadline=time.monotonic()+30
  while True:
   if server.poll() is not None:raise RuntimeError('Rust server exited: '+(run/'rust-server.log').read_text())
   try:urllib.request.urlopen(f'http://127.0.0.1:{ports[1]}/up',timeout=1).close();break
   except OSError:
    if time.monotonic()>deadline:raise RuntimeError('Rust server failed startup')
    time.sleep(.25)
  if hasattr(module,'after_start_case'):module.after_start_case(db_path,ports,case_env,oracle,root,case)
  elif hasattr(module,'after_start'):module.after_start(db_path,ports,case_env,oracle,root)
  for target in ['Rails','Rust']:
   with sqlite3.connect(db_path(target)) as db:original_state[target]=snapshot(db)
 for case in cases[args.mode]:
  provider=getattr(module,'environment_for_case',getattr(module,'case_environment',None))
  settings=provider(case,labels,root) if provider else {}
  if settings!=configuration:
   start_pair(case,settings);configuration=settings
   print('ORIGINAL_SERVER_CONFIGURATION '+json.dumps(settings,sort_keys=True),flush=True)
  for target,port in [('Rails',ports[0]),('Rust',ports[1])]:
   if (args.controls or args.mutation) and target=='Rails':continue
   fixture(target,case)
   if hasattr(module,'execute_case') and module.execute_case(target,case,port,db_path(target),labels,root,args.mutation):
    proof.append((target,case))
    continue
   script='ledger_browser_'+args.mode+'.mjs'
   cmd=['docker','run','--rm','--network','none','--cpus','1','--shm-size','256m','-v',f'{net_dir}:/upstream','-e','PARITY_UPSTREAM_SOCKET=/upstream/upstream.sock','-e',f'WS11UI_HOST_NETWORK={os.readlink("/proc/self/ns/net")}', '--label','parity.owner=ws11ui','-v',f'{root.parent}:/work:ro','-e',f'WS11UI_BROWSER_URL=http://127.0.0.1:{port}','-e',f'WS11UI_BROWSER_LABELS=/work/rust/parity/.seed/{seed.name}/labels.json','-v',f'{db_path(target).parent}:/database','-e','WS11UI_BROWSER_DATABASE=/database/production.sqlite3','-e',f'WS11UI_BROWSER_CASE={case}','-e',f'WS11UI_BROWSER_MODE={args.mode}','-e',f'WS11UI_BROWSER_CONTROL={int(args.controls)}','-e',f'WS11UI_BROWSER_MUTATION={args.mutation or ""}',image,'node','/work/rust/reference-tools/users/'+script]
   result=subprocess.run(cmd,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=getattr(module,'CASE_TIMEOUT',{}).get(case,90))
   print(f'Original {target} {case}:',flush=True);print(result.stdout,flush=True)
   if args.controls or args.mutation:
    assert result.returncode!=0 and 'ORIGINAL_MUTATION' in result.stdout and 'INVALID_CONTROL' not in result.stdout,(case,'invalid/surviving producer control',result.stdout[-2000:])
    assert ('original assertion' in result.stdout or 'TimeoutError' in result.stdout),(case,'wrong failure')
   else:
    assert result.returncode==0 and f'ORIGINAL_CASE {case}: passed' in result.stdout,(case,'browser receipt missing')
    requests=json.loads(next(s.removeprefix('ORIGINAL_STATE_REQUESTS ') for s in result.stdout.splitlines() if s.startswith('ORIGINAL_STATE_REQUESTS ')))
    check_state(target,requests)
   proof.append((target,case))
 print(f'Original browser {args.mode}: {len(proof)} '+('producer defects rejected; 0 invalid controls' if (args.controls or args.mutation) else 'paired case executions passed; 0 failures'),flush=True)
finally:
 if hasattr(module,'stop'):module.stop()
 if server:
  server.terminate()
  try:server.wait(timeout=15)
  except subprocess.TimeoutExpired:server.kill();server.wait()
 subprocess.run([reference,'down','--port',str(ports[0])],env=oracle,check=True)
 owned_network.close()
 forward.terminate();forward.wait(timeout=5)
 shutil.rmtree(net_dir)
 shutil.rmtree(seed)
 if os.environ.get('WS11UI_KEEP_BROWSER_SCRATCH')!='1':shutil.rmtree(run)
