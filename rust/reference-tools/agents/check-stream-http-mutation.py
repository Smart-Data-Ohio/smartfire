#!/usr/bin/env python3
"""The full HTTP test must reject a dropped durable trailing enqueue."""
from pathlib import Path
import os,re,subprocess
root=Path(__file__).resolve().parents[2]
p=root/'crates/db/src/models/agent_streaming.rs'
scratch=root.parent/'.scratch/stream-http-mutation'
scratch.mkdir(parents=True,exist_ok=True)
original=p.read_text()
needle='tx.emit_after_commit(crate::Event::job(&StreamTrailingBroadcastJob {'
assert needle in original
try:
    p.write_text(original.replace(needle,'let _ = (crate::Event::job(&StreamTrailingBroadcastJob {',1))
    result=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j','4','-p','campfire','agent_stream_http_enqueue_failure','--','--nocapture'],cwd=root,env=dict(os.environ,CI='1',TMPDIR=str(scratch)),capture_output=True,text=True)
    output=result.stdout+result.stderr
    (scratch/'before.log').write_text(output)
    summary=re.search(r'^test result: FAILED\..*$',output,re.M)
    assert result.returncode!=0 and summary and 'assertion' in output and 'error[E' not in output,output[-2000:]
    print('WS11-api HTTP enqueue mutation: '+summary.group())
finally:
    p.write_text(original)
print('WS11-api HTTP enqueue mutation: dropped job rejected; source restored')
