"""Measure the original WebDriver outer-window contract on managed Chrome.

The requested pairs come from the pinned system declarations. Chrome owns the
inner viewport calculation; the replay never assumes a toolbar-height offset.
"""
import json, os, pathlib, shutil, socket, subprocess, sys, tempfile, time, urllib.request

# ApplicationSystemTestCase plus the remaining original files' resize_to calls
# (room_header's width loops retain each width with their original height).
ORIGINAL_WINDOWS = [(1400,1400),(1440,1000),(390,844),(375,812),(1400,1000),
                    (320,740),(1400,400),(360,800),(390,800),(500,800),
                    (700,800),(768,800),(1024,800),(1280,900),(1400,900),(640,360)]

def browser_executables():
    driver=shutil.which('chromedriver')
    chrome=os.environ.get('CHROMIUM_BINARY') or shutil.which('chromium') or shutil.which('chromium-browser')
    if not chrome and pathlib.Path('/usr/lib/chromium/chromium').is_file():chrome='/usr/lib/chromium/chromium'
    if not driver or not chrome:raise RuntimeError('Native original browser requires chromedriver and chromium')
    return driver,chrome

def prepare_viewports(labels, root):
    """Populate labels before their seed copy is written; owns only its driver."""
    pin=(root/'parity/reference.sha').read_text().strip()
    # Each run gets its own directory under the shared cache. A pid path collides
    # when CI jobs share a home and reuse pids, and rmtree then fails with
    # "Directory not empty" or deletes another job's Chrome files.
    parent=pathlib.Path.home()/'.cache'/'campfire-chrome-tmp'
    parent.mkdir(parents=True,exist_ok=True)
    worker_tmpdir=pathlib.Path(tempfile.mkdtemp(dir=parent))
    with socket.socket() as sock:
        sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
    env=os.environ.copy();env['TMPDIR']=str(worker_tmpdir)
    driver_binary,chrome_binary=browser_executables()
    driver=subprocess.Popen([driver_binary,'--port='+str(port),'--allowed-ips=127.0.0.1'],env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    endpoint=f'http://127.0.0.1:{port}';session=None
    def request(method,path,body=None):
        payload=None if body is None else json.dumps(body).encode()
        req=urllib.request.Request(endpoint+path,data=payload,method=method,headers={'Content-Type':'application/json'})
        with urllib.request.urlopen(req,timeout=15) as response:return json.load(response)['value']
    try:
        deadline=time.monotonic()+10
        while True:
            try:request('GET','/status');break
            except OSError:
                if driver.poll() is not None or time.monotonic()>deadline:raise RuntimeError('native original viewport driver failed startup')
                time.sleep(.05)
        result=request('POST','/session',{'capabilities':{'alwaysMatch':{'browserName':'chrome','goog:chromeOptions':{'binary':chrome_binary,'args':['--headless=new','--no-sandbox','--disable-gpu','--mute-audio','--window-size=1400,1400']}}}})
        session=result['sessionId'];measured={}
        for width,height in ORIGINAL_WINDOWS:
            request('POST',f'/session/{session}/window/rect',{'width':width,'height':height})
            observation=request('POST',f'/session/{session}/execute/async',{'script':'const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(()=>done({width:innerWidth,height:innerHeight,outerWidth,outerHeight,devicePixelRatio})))','args':[]})
            assert observation['width']>0 and observation['height']>0,('invalid native original viewport',width,height,observation)
            measured[f'{width}x{height}']=observation
        labels['original.window_viewports']=measured
        labels['original.window_viewport_provenance']={'reference':pin,'producer':'Chromedriver managed headless Chrome window/rect and actual window.innerWidth/innerHeight','browserVersion':result['capabilities']['browserVersion'],'source_files':['test/application_system_test_case.rb','test/system/audit_log_test.rb','test/system/icons_test.rb','test/system/workspace_icons_test.rb','test/system/mobile_layout_test.rb','test/system/content_security_policy_test.rb','test/system/member_select_mode_test.rb','test/system/channel_members_test.rb','test/system/motion_test.rb','test/system/room_header_test.rb']}
        print('ORIGINAL_WINDOW_VIEWPORTS '+json.dumps(measured,sort_keys=True),flush=True)
    finally:
        if session:
            try:request('DELETE',f'/session/{session}')
            except OSError:pass
        driver.terminate();driver.wait(timeout=10)
        # Chrome can still be releasing profile files. Report each failure and
        # continue, so cleanup does not replace the test's own exception.
        def report_cleanup_error(_function, path, exc):
            error = exc[1] if isinstance(exc, tuple) else exc
            print(f'cleanup failed: {path}: {error}', file=sys.stderr)
        if sys.version_info >= (3, 12):
            shutil.rmtree(worker_tmpdir, onexc=report_cleanup_error)
        else:
            shutil.rmtree(worker_tmpdir, onerror=report_cleanup_error)
