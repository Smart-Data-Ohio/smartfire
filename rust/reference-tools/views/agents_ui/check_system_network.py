#!/usr/bin/env python3
"""In-flight HTTP must survive Docker bridge changes; the former host-browser must fail."""
import http.server, os, pathlib, subprocess, tempfile, threading, time
root=pathlib.Path(__file__).resolve().parents[4]
store=root/'.scratch/ws11ui-network-controls';store.mkdir(parents=True,exist_ok=True)
class HeldResponse(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200);self.end_headers();self.wfile.write(b'<h1>ready</h1>')
    def do_POST(self):
        # Hold the real request until the host's veth pair has appeared and disappeared.
        name=f'ws11ui-network-control-{os.getpid()}'
        subprocess.run(['docker','network','create',name],check=True,stdout=subprocess.DEVNULL)
        try: time.sleep(1) # Linux network-change notification coalescing, not an assertion deadline.
        finally: subprocess.run(['docker','network','rm',name],check=True,stdout=subprocess.DEVNULL)
        try:
            self.send_response(200);self.end_headers();self.wfile.write(b'<h1>stable network</h1>')
        except (BrokenPipeError,ConnectionResetError): pass
    def log_message(self,*_args): pass
with tempfile.TemporaryDirectory(dir=store,prefix='control.') as folder:
    labels=pathlib.Path(folder)/'labels.json';labels.write_text('{}')
    server=http.server.ThreadingHTTPServer(('127.0.0.1',52796),HeldResponse)
    thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
    try:
        for broken in [True,False]:
            env={**os.environ,'PARITY_NAMESPACE':'ws11ui-network-control'}
            if broken: env['WS11UI_BROKEN_HOST_NETWORK']='1'
            result=subprocess.run(['bash',str(root/'rust/reference-tools/views/agents_ui/system_browser.sh'),'http://127.0.0.1:52796',str(labels),str(labels),'network-probe'],cwd=root,env=env,capture_output=True,text=True)
            output=result.stdout+result.stderr
            if broken:
                assert result.returncode and 'browser must have an isolated network namespace' in output, output
                print('Host-network negative control: same network namespace; replay rejected before launch',flush=True)
            else:
                assert result.returncode==0 and 'isolated namespace' in output,output
                print('Isolated-network regression: in-flight response survived Docker bridge churn; 1 passed, 0 failed',flush=True)
    finally: server.shutdown();server.server_close();thread.join()
