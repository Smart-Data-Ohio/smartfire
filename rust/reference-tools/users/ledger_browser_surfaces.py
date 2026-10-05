"""Original surface browser fixtures; no production responses are synthesized."""
import hashlib,json
CASES=['audit-filter','audit-phone','icon-colon','icon-room','icon-search','icon-lobehub','mobile-outside','mobile-destinations','group-ringing','worker-cache','worker-retry','timezone-once','unread-few','unread-many','unread-pill','unread-offpage','unread-menu','workspace-icons','csp-login','csp-main','csp-turbo','csp-inline','csp-surfaces']
CSP_ENV={'GOOGLE_CLIENT_ID':'test-client-id','GOOGLE_CLIENT_SECRET':'test-client-secret','LIVEKIT_URL':'ws://127.0.0.1:9','LIVEKIT_INTERNAL_URL':'http://127.0.0.1:10','LIVEKIT_API_KEY':'csp-test-key','LIVEKIT_API_SECRET':'csp-test-secret-csp-test-secret-0000','LIVEKIT_GATEWAY_SECRET':'csp-test-gateway-secret','GOOGLE_PICKER_API_KEY':'test-picker-key','GOOGLE_CLOUD_PROJECT_NUMBER':'123456789012'}

def prepare(db,labels,root):
    assert {r[0] for r in db.execute('SELECT id FROM messages WHERE room_id=?',(labels['rooms.designers'],))}=={labels['messages.'+name] for name in ['first','second','third']}, 'original Designers fixture must contain only first/second/third'
    labels['surface.stage']=9100900101;labels['surface.board']=9100900102
    labels['surface.group_huddle_environment']=dict(GROUP_HUDDLE_ENV)

def check_state(db,requests,labels,root):
    for r in requests:
        file,line=r['file'],r['line']
        if r['kind']=='message':
            row=db.execute('SELECT id FROM messages WHERE markdown_source=? ORDER BY id DESC LIMIT 1',(r['body'],)).fetchone()
            assert row and row[0]==r['id'],(file,line,'persisted original markdown',row,r)
        elif r['kind']=='message-absent':
            assert not db.execute('SELECT 1 FROM messages WHERE markdown_source=?',(r['body'],)).fetchone(),(file,line,r)
        elif r['kind']=='timezone':
            assert db.execute('SELECT time_zone FROM users WHERE id=?',(labels['users.jz'],)).fetchone()==(r['zone'],),(file,line,r)
        elif r['kind']=='huddle-activity':
            assert db.execute('SELECT 1 FROM activity_items WHERE user_id=? AND event_type="huddle_started"',(labels['users.'+r['user']],)).fetchone(),(file,line,r)
        else:raise AssertionError(r)
        print(f'ORIGINAL_ASSERTION {file}:{line}',flush=True)

GROUP_HUDDLE_ENV={'LIVEKIT_URL':'wss://huddle.example.test','LIVEKIT_INTERNAL_URL':'ws://livekit.example.test:7880','LIVEKIT_API_KEY':'test-api-key','LIVEKIT_API_SECRET':'test-api-secret','LIVEKIT_GATEWAY_SECRET':'test-gateway-secret'}
CASE_ENV={case:(dict(CSP_ENV) if case.startswith('csp-') else {}) for case in CASES}

def environment(labels,root):
    return {}

def case_environment(case,labels,root):
    return CASE_ENV.get(case,{})

def execute_case(target,case,port,database_path,labels,root,mutation=''):
    """WebDriver-managed launch policy needs Chromedriver, as the original does."""
    if case!='browser-profile':return False
    import os,pathlib,re,socket,subprocess,tempfile,time,urllib.request,shutil,sys
    if '--controls' in sys.argv and not mutation:mutation='browser-profile-explicit'
    print(f'Original {target} {case}:',flush=True)
    file='test/system/browser_launch_profile_test.rb'
    worker_tmpdir=pathlib.Path.home()/'.cache/campfire-chrome-tmp'/str(os.getpid())
    worker_tmpdir.mkdir(parents=True,exist_ok=True)
    with socket.socket() as sock:
        sock.bind(('127.0.0.1',0));driver_port=sock.getsockname()[1]
    env=os.environ.copy();env['TMPDIR']=str(worker_tmpdir)
    from ledger_browser_viewports import browser_executables
    driver_binary,chrome_binary=browser_executables()
    driver=subprocess.Popen([driver_binary,'--port='+str(driver_port),'--allowed-ips=127.0.0.1'],env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    endpoint=f'http://127.0.0.1:{driver_port}'
    session=None
    def request(method,path,body=None):
        payload=None if body is None else json.dumps(body).encode()
        r=urllib.request.Request(endpoint+path,data=payload,method=method,headers={'Content-Type':'application/json'})
        with urllib.request.urlopen(r,timeout=15) as response:return json.load(response)
    try:
        deadline=time.monotonic()+10
        while True:
            try:request('GET','/status');break
            except OSError:
                if driver.poll() is not None or time.monotonic()>deadline:raise RuntimeError('INVALID_CONTROL Chromedriver startup')
                time.sleep(.05)
        arguments=['--headless=new','--no-sandbox','--disable-gpu','--mute-audio','--window-size=1400,1400']
        if mutation=='browser-profile-explicit':
            arguments.append('--user-data-dir='+str(worker_tmpdir/'explicit'))
            print('ORIGINAL_MUTATION Chromedriver explicit user-data-dir launcher',flush=True)
        session=request('POST','/session',{'capabilities':{'alwaysMatch':{'browserName':'chrome','goog:chromeOptions':{'binary':chrome_binary,'args':arguments}}}})['value']['sessionId']
        request('POST','/session/'+session+'/url',{'url':f'http://127.0.0.1:{port}/'})
        request('POST','/session/'+session+'/url',{'url':'chrome://version/'})
        info=request('POST','/session/'+session+'/execute/sync',{'script':'return document.body.innerText','args':[]})['value']
        def info_line(label):
            line=next((x for x in info.splitlines() if x.startswith(label+'\t')),None)
            assert line is not None,(file,31,'original assertion expected info row',label)
            print(f'ORIGINAL_ASSERTION {file}:31',flush=True)
            return line.removeprefix(label+'\t')
        command_line=info_line('Command Line');profile_path=info_line('Profile Path')
        assert '--user-data-dir='+str(worker_tmpdir)+'/' in command_line,(file,22,'original assertion managed worker TMPDIR',command_line)
        print(f'ORIGINAL_ASSERTION {file}:22',flush=True)
        try:
            assert re.search(r'org\.chromium\.Chromium\.',profile_path),f'{file}:24: original assertion Chromedriver managed temp profile {profile_path}'
        except AssertionError as error:
            if mutation!='browser-profile-explicit':raise
            print(repr(error),flush=True)
            print('ORIGINAL_NATIVE_CONTROL browser-profile-explicit: original assertion 24 rejected',flush=True)
            return True
        if mutation=='browser-profile-explicit':raise AssertionError('INVALID_CONTROL explicit-profile launcher survived')
        print(f'ORIGINAL_ASSERTION {file}:24',flush=True)
        print(f'ORIGINAL_CASE {case}: passed',flush=True)
        return True
    finally:
        if session:
            try:request('DELETE','/session/'+session)
            except OSError:pass
        driver.terminate();driver.wait(timeout=10)
        shutil.rmtree(worker_tmpdir)
        if mutation:print('ORIGINAL_PRODUCER_RESTORED browser-profile: disposable explicit profile removed; managed launcher retained',flush=True)

CASES.append('browser-profile')

from ledger_browser_lifecycle import host_command,start_bridge,stop

def after_start(db_path,ports,env,oracle,root):
    start_bridge(db_path,ports,env,oracle,root)

def reference_image(base,root):
    import subprocess
    source=root/'reference-tools/users/ledger_browser_surfaces_initializer.rb'
    digest=hashlib.sha256(source.read_bytes()+base.encode()).hexdigest()[:12]
    name='ledger-surfaces-reference:'+digest
    if subprocess.run(['docker','image','inspect',name],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode:
        dockerfile='ARG BASE\nFROM ${BASE}\nCOPY ledger_browser_surfaces_initializer.rb /rails/config/initializers/ledger_browser_surfaces.rb\n'
        subprocess.run(['docker','build','--build-arg','BASE='+base,'-f','-','-t',name,str(source.parent)],input=dockerfile,text=True,check=True,stdout=subprocess.DEVNULL)
    return name

CASES.insert(CASES.index('timezone-once'),'timezone-no-csrf')

CONTROL_CASES=['audit-filter','icon-colon','icon-room','icon-search','icon-lobehub','workspace-icons','unread-pill','worker-retry','group-ringing','timezone-no-csrf','browser-profile']
