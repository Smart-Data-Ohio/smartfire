"""Muted delivery and real Calendar refresh with an injected server clock."""
import hashlib,json,os,sqlite3,subprocess,threading,time,urllib.request
from pathlib import Path
CASES=['muted-delivery','calendar-lifecycle']
_stop=threading.Event()
_thread=None

def prepare(db,labels,root):
    pass

def environment_for_case(case,labels,root):
    return {'LEDGER_BROWSER_CLOCK':'1','GOOGLE_CLIENT_ID':'test-client-id','GOOGLE_CLIENT_SECRET':'test-client-secret'} if case=='calendar-lifecycle' else {}

def reference_image(base,root):
    source=root/'reference-tools/users/ledger_browser_lifecycle_initializer.rb'
    digest=hashlib.sha256(source.read_bytes()+base.encode()).hexdigest()[:12]
    name='ledger-lifecycle-reference:'+digest
    if subprocess.run(['docker','image','inspect',name],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode:
        dockerfile='ARG BASE\nFROM ${BASE}\nCOPY ledger_browser_lifecycle_initializer.rb /rails/config/initializers/ledger_browser_lifecycle.rb\n'
        subprocess.run(['docker','build','--build-arg','BASE='+base,'-f','-','-t',name,str(source.parent)],input=dockerfile,text=True,check=True,stdout=subprocess.DEVNULL)
    return name

def host_command(binary,env,root):
    env['WS11UI_LEDGER_HOST']='1'
    return [os.environ['WS11UI_LEDGER_TEST_HOST'],'controllers::ledger_browser_tests::lifecycle_host::original_ledger_injected_clock_host','--exact','--ignored','--nocapture']

def start_bridge(db_path,ports,env,oracle,root,setup_calendar=False,setup_mention=False):
    global _thread
    _stop.clear()
    reference=root/'parity/bin/reference'
    script=root/'reference-tools/users/ledger_browser_lifecycle.rb'
    def invoke(request):
        if setup_calendar and request['action'] in ['refresh','advance']:
            directory=db_path('Rails').parent
            temporary=directory/'ledger-calendar-request.tmp'
            temporary.write_text(json.dumps(request));temporary.replace(directory/'ledger-calendar-request.json')
            urllib.request.urlopen(f'http://127.0.0.1:{ports[0]}/up',timeout=10).read()
            response=json.loads((directory/'ledger-calendar-response.json').read_text())
            assert response['sequence']==request['sequence'],response
            assert 'error' not in response,response
            return response['result']
        if request['action']=='setup':
            result=subprocess.check_output([str(reference),'runner','--port',str(ports[0]),'--time','2026-03-02T16:00:00Z','--freeze',str(script),json.dumps(request)],env=oracle,text=True)
            return json.loads(result.splitlines()[-1])
        # Execute model/broadcast operations in the same process as the original
        # test ActionCable adapter; /up only pumps private file IPC.
        directory=db_path('Rails').parent
        temporary=directory/'ledger-rpc-request.tmp'
        temporary.write_text(json.dumps(request));temporary.replace(directory/'ledger-rpc-request.json')
        urllib.request.urlopen(f'http://127.0.0.1:{ports[0]}/up',timeout=10).read()
        response=json.loads((directory/'ledger-rpc-response.json').read_text())
        assert response['sequence']==request['sequence'],response
        assert 'error' not in response,response
        return response['result']
    if setup_calendar or setup_mention:
        setup=invoke({'action':'setup','user':127326141,'connect_calendar':setup_calendar})
        # Signed attachment bytes are produced by the unchanged Rails mention partial.
        for target in ['Rails','Rust']:
            (db_path(target).parent/'ledger-mention.json').write_text(json.dumps(setup['mention']))
        if setup_calendar:
            # The reference-created encrypted account is the same fixture on Rust.
            with sqlite3.connect(db_path('Rails')) as source,sqlite3.connect(db_path('Rust')) as target:
                cols=[x[1] for x in source.execute('pragma table_info(google_accounts)')]
                rows=source.execute('SELECT '+','.join(cols)+' FROM google_accounts').fetchall()
                target.execute('DELETE FROM google_accounts')
                target.executemany('INSERT INTO google_accounts('+','.join(cols)+') VALUES('+','.join('?' for c in cols)+')',rows)
    def follow():
        seen=0;directory=db_path('Rails').parent
        while not _stop.wait(.02):
            request=directory/'ledger-request.json'
            if not request.exists():continue
            data=json.loads(request.read_text())
            if data['sequence']<=seen:continue
            seen=data['sequence']
            try:answer={'sequence':seen,'result':invoke(data)}
            except Exception as e:answer={'sequence':seen,'error':str(e)}
            tmp=directory/'ledger-response.tmp';tmp.write_text(json.dumps(answer));tmp.replace(directory/'ledger-response.json')
    _thread=threading.Thread(target=follow);_thread.start()

def after_start_case(db_path,ports,env,oracle,root,case):
    start_bridge(db_path,ports,env,oracle,root,setup_calendar=case=='calendar-lifecycle',setup_mention=case=='muted-delivery')

def after_start(db_path,ports,env,oracle,root):
    start_bridge(db_path,ports,env,oracle,root)

def stop():
    _stop.set()
    if _thread:_thread.join()

def fixture(db,case,labels,root):
    pass

def check_state(db,requests,labels,root):
    for r in requests:
        if r['kind']=='calendar-enabled':
            assert db.execute('SELECT meeting_status_enabled FROM users WHERE id=?',(labels['users.david'],)).fetchone()==(1,)
        elif r['kind']=='muted':
            assert db.execute('SELECT involvement FROM memberships WHERE room_id=? AND user_id=?',(labels['rooms.designers'],labels['users.david'])).fetchone()==('muted',)
        else:raise AssertionError(r)
