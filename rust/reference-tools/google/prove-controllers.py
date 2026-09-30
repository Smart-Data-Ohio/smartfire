#!/usr/bin/env python3
"""Inject security regressions and demand an assertion failure, restoring source in finally."""
import os, subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2]
scratch=root.parent/'.scratch/ws14g/mutations'
scratch.mkdir(parents=True,exist_ok=True)
def check(name,path,before,after,test):
    source=path.read_text()
    assert before in source
    try:
        path.write_text(source.replace(before,after,1))
        result=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j','4','-p','campfire',test,'--','--nocapture'],cwd=root,env=dict(os.environ,CI='1',TMPDIR=str(scratch)),text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
        (scratch/f'{name}.log').write_text(result.stdout)
        lines=[line for line in result.stdout.splitlines() if line.startswith('test result:')]
        assert result.returncode==101 and 'assertion' in result.stdout and any('FAILED' in line for line in lines),result.stdout
        print('\n'.join(lines),flush=True)
        print(f'Google controller mutation {name}: rejected',flush=True)
    finally: path.write_text(source)
check('callback-state-bypass',root/'crates/campfire/src/controllers/google_sign_in.rs','if !valid {','if false && !valid {','app::google_tests::security_wrong_state')
check('webhook-token-bypass',root/'crates/db/src/models/google_calendar.rs','if !channel.token_matches(token)', 'if false && !channel.token_matches(token)','app::google_webhook_tests::security_wrong_token')
check('webhook-replay-bypass',root/'crates/db/src/models/google_calendar.rs','AND last_message_number<?','AND ? > 0','app::google_webhook_tests::security_replay')
print('Google controller security discrimination: 3 mutations rejected',flush=True)
