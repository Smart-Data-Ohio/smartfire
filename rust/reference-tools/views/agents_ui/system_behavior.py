#!/usr/bin/env python3
"""Run named Rails system behavior on a pinned private Rails instance and Rust.
Build the binary and agents_ui seed first; npm ci --prefix rust/parity installs
its committed Playwright dependency. No images are captured or compared.
"""
import argparse, json, os, pathlib, shutil, signal, sqlite3, subprocess, tempfile, time, urllib.request
root = pathlib.Path(__file__).resolve().parents[4]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary', type=pathlib.Path, required=True)
p.add_argument('--scenario', choices=['all','pages','budget','work'], default='all')
args = p.parse_args()
if args.scenario == 'all':
    codes = [subprocess.run(['python3', str(pathlib.Path(__file__).resolve()), '--binary', str(args.binary.resolve()), '--scenario', scenario], cwd=root).returncode for scenario in ['pages', 'budget', 'work']]
    print(f'Agent behavior scenarios: {len(codes) - sum(bool(c) for c in codes)} completed; {sum(bool(c) for c in codes)} failed; 2 deferred', flush=True)
    raise SystemExit(1 if any(codes) else 0)
frozen_time = '2026-03-03T16:00:00Z' if args.scenario == 'budget' else '2026-03-02T16:00:00Z'
seed = root / 'rust/parity/.seed/agents_ui'
labels = json.loads((seed / 'labels.json').read_text())
store = root / '.scratch/system-behavior'
store.mkdir(parents=True, exist_ok=True)
work = pathlib.Path(tempfile.mkdtemp(dir=store))
env = {**os.environ, 'PARITY_NAMESPACE':'ws11ui-system', 'PARITY_OWNER':'ws11ui', 'PARITY_SEED_DIR':str(work/'seeds'), 'PARITY_IMAGE':os.environ.get('PARITY_IMAGE','ws11ui-reference:d7c7de92')}
reference = root / 'rust/parity/bin/reference'
child = None

def wait_up(port):
    for _ in range(240):
        try:
            with urllib.request.urlopen(f'http://127.0.0.1:{port}/up',timeout=1) as reply:
                if reply.status == 200: return
        except OSError: pass
        time.sleep(.25)
    raise RuntimeError(f'owned server {port} did not boot within 60s')
try:
    private = work/'seeds/agents_ui'
    shutil.copytree(seed,private)
    labels_file = work/'labels.json'
    candidate = work/'candidate'; (candidate/'db').mkdir(parents=True);shutil.copytree(private/'storage',candidate/'files')
    subprocess.run([str(reference),'up','--seed','agents_ui','--port','52798','--time',frozen_time,'--freeze'],cwd=root,env=env,check=True)
    # Shared rules approve exactly this post-pin status-popup layout delta.
    layout = work/'application.html.erb';layout.write_bytes(subprocess.check_output(['git','show','2e20b24c3f2be9db8a646a1352c159b4afacad0e:app/views/layouts/application.html.erb'],cwd=root))
    subprocess.run(['docker','cp',str(layout),'ws11ui-system-reference-52798:/rails/app/views/layouts/application.html.erb'],check=True)
    fixture = subprocess.check_output([str(reference),'runner','--port','52798','--time',frozen_time,'--freeze',str(root/'rust/reference-tools/views/agents_ui'/({'budget':'budget_fixture.rb','work':'work_fixture.rb'}.get(args.scenario,'system_fixture.rb')))],cwd=root,env=env,text=True)
    labels.update(json.loads(fixture)); labels_file.write_text(json.dumps(labels))
    source_db = work/'seeds/.instances/52798/db/production.sqlite3'
    with sqlite3.connect(source_db) as source, sqlite3.connect(candidate/'db/production.sqlite3') as target: source.backup(target)
    candidate_env = dict(os.environ)
    for line in (root/'rust/parity/.env.reference').read_text().splitlines():
        if line and not line.startswith('#') and '=' in line:
            key,value=line.split('=',1);candidate_env[key]=value.strip('"\'')
    candidate_env.update({'CAMPFIRE_STORAGE_PATH':str(candidate),'CAMPFIRE_FROZEN_TIME':frozen_time,'DISABLE_SSL':'true','HTTP_PORT':'52799','TARGET_PORT':'52797','APP_VERSION':'parity','GIT_REVISION':'parity'})
    with (work/'candidate.log').open('w') as log:
        child=subprocess.Popen([str(args.binary.resolve())],cwd=root,env=candidate_env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
        wait_up(52799)
        result_codes=[]
        for name,port,db in [('Rails',52798,work/'seeds/.instances/52798/db/production.sqlite3'),('Rust',52799,candidate/'db/production.sqlite3')]:
            print(f'{name} system behavior:',flush=True)
            result=subprocess.run(['node',str(root/'rust/reference-tools/views/agents_ui/system_cases.mjs'),f'http://127.0.0.1:{port}',str(labels_file),str(db),args.scenario],cwd=root)
            result_codes.append(result.returncode)
        raise SystemExit(1 if any(result_codes) else 0)
finally:
    if child is not None:
        if child.poll() is None:
            os.killpg(child.pid,signal.SIGTERM)
            try: child.wait(timeout=5)
            except subprocess.TimeoutExpired: os.killpg(child.pid,signal.SIGKILL);child.wait()
    subprocess.run([str(reference),'down','--port','52798'],cwd=root,env=env,stdout=subprocess.DEVNULL,check=False)
    shutil.rmtree(work)
