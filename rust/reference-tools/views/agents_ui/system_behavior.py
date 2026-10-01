#!/usr/bin/env python3
"""Run named Rails system behavior on a pinned private Rails instance and Rust.
Build the binary and agents_ui seed first; npm ci --prefix rust/parity installs
its committed Playwright dependency. No images are captured or compared.
"""
import argparse, json, os, pathlib, shutil, signal, sqlite3, subprocess, tempfile, time, urllib.request
root = pathlib.Path(__file__).resolve().parents[4]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary', type=pathlib.Path, required=True)
args = p.parse_args()
seed = root / 'rust/parity/.seed/agents_ui'
labels = json.loads((seed / 'labels.json').read_text())
store = root / '.scratch/system-behavior'
store.mkdir(parents=True, exist_ok=True)
work = pathlib.Path(tempfile.mkdtemp(dir=store))
env = {**os.environ, 'PARITY_NAMESPACE':'ws11ui-system', 'PARITY_OWNER':'ws11ui', 'PARITY_SEED_DIR':str(work/'seeds'), 'PARITY_IMAGE':os.environ.get('PARITY_IMAGE','ws11ui-reference:d7c7de92')}
reference = root / 'rust/parity/bin/reference'
child = None

def fixture(db):
    c = sqlite3.connect(db)
    c.execute("UPDATE agents SET provider='OpenAI',runtime='Codex CLI 0.9',description='Does things',status='working',status_note='on it',last_seen_at='2026-03-02 16:00:00',working_presence='Running tests…',working_presence_expires_at='2026-03-02 16:02:00' WHERE id=?",(labels['agents.bender'],))
    c.execute("UPDATE messages SET created_at='2026-03-02 16:00:00',updated_at='2026-03-02 16:00:00' WHERE id=?",(labels['messages.agent_ui'],))
    for label,name,status,input_,output,duration in [('ui_done','Run tests','done',None,'All green',1500),('ui_running','Deploy','running','Ship it',None,None)]:
        c.execute('UPDATE agent_steps SET channel_thread_id=NULL,message_id=?,name=?,status=?,input_summary=?,output_summary=?,duration_ms=? WHERE id=?',(labels['messages.agent_ui'],name,status,input_,output,duration,labels['agent_steps.'+label]))
    # A public fixture, matching AgentApprovalsTest setup; boot's owner callbacks
    # handle subsequent decisions. Both runtimes start from these identical rows.
    c.execute("INSERT INTO agent_approvals(id,agent_id,room_id,action,summary,status,expires_at,created_at,updated_at) VALUES (90015001,?,?,'deploy','Ship the release','pending','2026-03-02 16:30:00','2026-03-02 16:00:00','2026-03-02 16:00:00')",(labels['agents.bender'],labels['rooms.watercooler']))
    c.execute("INSERT INTO activity_items(id,user_id,event_type,source_type,source_id,created_at,updated_at) VALUES (90015001,?,'agent_approval_request','AgentApproval',90015001,'2026-03-02 16:00:00','2026-03-02 16:00:00')",(labels['users.david'],))
    c.commit(); c.close()

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
    shutil.copytree(seed,private);fixture(private/'db/production.sqlite3')
    labels.update({'system.activity_item':90015001,'system.approval':90015001})
    labels_file = work/'labels.json'; labels_file.write_text(json.dumps(labels))
    candidate = work/'candidate';shutil.copytree(private/'db',candidate/'db');shutil.copytree(private/'storage',candidate/'files')
    subprocess.run([str(reference),'up','--seed','agents_ui','--port','52798','--time','2026-03-02T16:00:00Z','--freeze'],cwd=root,env=env,check=True)
    # Shared rules approve exactly this post-pin status-popup layout delta.
    layout = work/'application.html.erb';layout.write_bytes(subprocess.check_output(['git','show','2e20b24c3f2be9db8a646a1352c159b4afacad0e:app/views/layouts/application.html.erb'],cwd=root))
    subprocess.run(['docker','cp',str(layout),'ws11ui-system-reference-52798:/rails/app/views/layouts/application.html.erb'],check=True)
    candidate_env = dict(os.environ)
    for line in (root/'rust/parity/.env.reference').read_text().splitlines():
        if line and not line.startswith('#') and '=' in line:
            key,value=line.split('=',1);candidate_env[key]=value.strip('"\'')
    candidate_env.update({'CAMPFIRE_STORAGE_PATH':str(candidate),'CAMPFIRE_FROZEN_TIME':'2026-03-02T16:00:00Z','DISABLE_SSL':'true','HTTP_PORT':'52799','TARGET_PORT':'52797','APP_VERSION':'parity','GIT_REVISION':'parity'})
    with (work/'candidate.log').open('w') as log:
        child=subprocess.Popen([str(args.binary.resolve())],cwd=root,env=candidate_env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
        wait_up(52799)
        result_codes=[]
        for name,port,db in [('Rails',52798,work/'seeds/.instances/52798/db/production.sqlite3'),('Rust',52799,candidate/'db/production.sqlite3')]:
            print(f'{name} system behavior:',flush=True)
            result=subprocess.run(['node',str(root/'rust/reference-tools/views/agents_ui/system_cases.mjs'),f'http://127.0.0.1:{port}',str(labels_file),str(db)],cwd=root)
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
