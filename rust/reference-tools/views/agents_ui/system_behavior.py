#!/usr/bin/env python3
"""Run named Rails system behavior against Rust, on fixtures recorded from Rails.
Build the binary and restore the frozen agents_ui seed first; each scenario applies
test-support/agents-ui-fixtures/SCENARIO (the recorded Rails fixture step) to a copy.
The pinned Playwright Docker image supplies its committed browser dependencies.
No images are captured or compared.
"""
import argparse, json, os, pathlib, shutil, signal, sqlite3, subprocess, sys, tempfile, time, urllib.request
root = pathlib.Path(__file__).resolve().parents[4]
sys.path.insert(0, str(root/'rust/reference-tools/users'))
from browser_port_leases import reserve_system_ports
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary', type=pathlib.Path, required=True)
p.add_argument('--scenario', choices=['all','pages','budget','work','inbox','inbox-filter'], default='all')
p.add_argument('--inject-work-status', action='store_true', help='Discrimination probe: make the candidate writer retain planned status')
p.add_argument('--inject-stream-finalize', action='store_true', help='Discrimination probe: make the candidate writer retain streaming state')
p.add_argument('--test-host', type=pathlib.Path, help='Current-source campfire test binary (inbox producer runs only in cfg(test))')
p.add_argument('--inject-inbox-handle', action='store_true')
p.add_argument('--inject-inbox-preference', action='store_true')
args = p.parse_args()
if args.scenario.startswith('inbox') and not args.test_host: p.error('inbox requires --test-host')
if args.inject_stream_finalize and args.scenario != 'pages': p.error('--inject-stream-finalize requires --scenario pages')
if args.inject_work_status and args.scenario != 'work': p.error('--inject-work-status requires --scenario work')
if args.scenario == 'all':
    codes = [subprocess.run(['python3', str(pathlib.Path(__file__).resolve()), '--binary', str(args.binary.resolve()), '--scenario', scenario], cwd=root).returncode for scenario in ['pages', 'budget', 'work']]
    print(f'Agent behavior scenarios: {len(codes) - sum(bool(c) for c in codes)} completed; {sum(bool(c) for c in codes)} failed; 0 deferred', flush=True)
    raise SystemExit(1 if any(codes) else 0)
frozen_time = '2026-03-03T16:00:00Z' if args.scenario == 'budget' else '2026-03-02T16:00:00Z'
seed = root / 'rust/parity/.seed/agents_ui'
fixtures = root / 'rust/test-support/agents-ui-fixtures' / args.scenario
labels = json.loads((seed / 'labels.json').read_text())
store = root / '.scratch/system-behavior'
store.mkdir(parents=True, exist_ok=True)
work = pathlib.Path(tempfile.mkdtemp(dir=store))
child = None
# Keep simultaneous worktrees' servers and teardown isolated.
lease = reserve_system_ports(args.scenario)
_, candidate_port, target_port = lease.ports
print(f'AGENT_SYSTEM_PORT_LEASE {args.scenario}: {lease.ports}', flush=True)

def wait_up(port):
    for _ in range(240):
        try:
            with urllib.request.urlopen(f'http://127.0.0.1:{port}/up',timeout=1) as reply:
                if reply.status == 200: return
        except OSError: pass
        time.sleep(.25)
    raise RuntimeError(f'owned server {port} did not boot within 60s')
try:
    labels_file = work/'labels.json'
    # The browser asks the Rust test host for ActivityInboxTest's interleaved item here.
    control = work/'Rust-control'
    candidate = work/'candidate'; (candidate/'db').mkdir(parents=True); shutil.copytree(seed/'storage',candidate/'files')
    # The Rails fixture script's effect on the seed and the labels it printed.
    labels.update(json.loads((fixtures/'labels.json').read_text())); labels_file.write_text(json.dumps(labels))
    shutil.copyfile(seed/'db/production.sqlite3', candidate/'db/production.sqlite3')
    with sqlite3.connect(candidate/'db/production.sqlite3') as target: target.executescript((fixtures/'patch.sql').read_text())
    if args.inject_work_status:
        # Exercise the real UI, endpoint and writer. A deliberately broken SQLite
        # writer loses the status update, while still returning its real response.
        with sqlite3.connect(candidate/'db/production.sqlite3') as target:
            target.execute("CREATE TRIGGER ws11ui_broken_work_status AFTER UPDATE OF work_status ON channel_threads WHEN NEW.work_status='in_progress' BEGIN UPDATE channel_threads SET work_status='planned' WHERE id=NEW.id; END")
    if args.inject_stream_finalize:
        with sqlite3.connect(candidate/'db/production.sqlite3') as target:
            target.execute("CREATE TRIGGER ws11ui_broken_stream_finalize AFTER UPDATE OF streaming ON messages WHEN NEW.streaming=0 AND OLD.streaming=1 BEGIN UPDATE messages SET streaming=1 WHERE id=NEW.id; END")
    if args.inject_inbox_handle:
        with sqlite3.connect(candidate/'db/production.sqlite3') as target:
            target.execute("CREATE TRIGGER ws11ui_broken_handle AFTER UPDATE OF handled_at ON activity_items BEGIN UPDATE activity_items SET handled_at=NULL,read_at=NULL WHERE id=NEW.id; END")
    if args.inject_inbox_preference:
        with sqlite3.connect(candidate/'db/production.sqlite3') as target:
            target.execute("CREATE TRIGGER ws11ui_broken_preference AFTER UPDATE OF inbox_preferences ON users BEGIN UPDATE users SET inbox_preferences='{}' WHERE id=NEW.id; END")
    candidate_env = dict(os.environ)
    for line in (root/'rust/parity/.env.reference').read_text().splitlines():
        if line and not line.startswith('#') and '=' in line:
            key,value=line.split('=',1);candidate_env[key]=value.strip('"\'')
    candidate_env.update({'CAMPFIRE_STORAGE_PATH':str(candidate),'CAMPFIRE_FROZEN_TIME':frozen_time,'DISABLE_SSL':'true','HTTP_PORT':str(candidate_port),'TARGET_PORT':str(target_port),'APP_VERSION':'parity','GIT_REVISION':'parity'})
    with (work/'candidate.log').open('w') as log:
        host_command = [str(args.binary.resolve())]
        if args.scenario.startswith('inbox'):
            host_command = [str(args.test_host.resolve()), 'controllers::presenters::test_support::ws8bm_browser_host_without_jobs', '--exact', '--ignored', '--nocapture']
            candidate_env.update(WS8BM_BROWSER_HOST='1', WS11UI_ACTIVITY_CONTROL=str(control), WS11UI_ACTIVITY_USER=str(labels['users.david']), WS11UI_ACTIVITY_SOURCE=str(labels['messages.second']))
        child=subprocess.Popen(host_command,cwd=root,env=candidate_env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
        wait_up(candidate_port)
        print('Rust system behavior:',flush=True)
        browser_env = dict(os.environ, WS11UI_ACTIVITY_CONTROL=str(control))
        result=subprocess.run(['bash',str(root/'rust/reference-tools/views/agents_ui/system_browser.sh'),f'http://127.0.0.1:{candidate_port}',str(labels_file),str(candidate/'db/production.sqlite3'),args.scenario],cwd=root,env=browser_env)
        raise SystemExit(result.returncode and 1)
finally:
    failure = sys.exc_info()[1]
    if failure is not None and not (isinstance(failure, SystemExit) and failure.code in (None, 0)):
        log = work/'candidate.log'
        if log.exists(): print('CANDIDATE_SERVER_DIAGNOSTICS\n'+log.read_text()[-12000:], flush=True)
    if child is not None:
        if child.poll() is None:
            os.killpg(child.pid,signal.SIGTERM)
            try: child.wait(timeout=5)
            except subprocess.TimeoutExpired: os.killpg(child.pid,signal.SIGKILL);child.wait()
    lease.close()
    shutil.rmtree(work)
