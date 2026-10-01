#!/usr/bin/env python3
"""Show that real HTTP role/owner gates and byte comparisons reject grouped faults."""
from pathlib import Path
import os
import subprocess
root=Path(__file__).resolve().parents[2]
paths=[root/'crates/campfire/src/controllers/slack/runs.rs',root/'crates/views/templates/accounts/slack_import_runs/plan.html']
original=[path.read_text() for path in paths]
try:
    faults=[[("if admin {", "if admin && false {"),('admin || (r.user_id == uid && r.kind == "personal")','admin || r.kind == "personal"')],[(">Import plan</h1>",">Faulty import plan</h1>")]]
    for path,source,changes in zip(paths,original,faults):
        for before,after in changes:
            if before not in source:raise RuntimeError('missing mutation anchor: '+before)
            source=source.replace(before,after)
        path.write_text(source)
    for case,test in [('admin-member-index','slack_run_http_actions_sessions_csrf_rows_audits_and_jobs_match_rails'),('personal-foreign-show','slack_run_http_actions_sessions_csrf_rows_audits_and_jobs_match_rails'),(None,'slack_run_views_match_every_rails_body_byte')]:
        command=os.environ.get('WS16_CARGO','mise exec rust@1.98.1 -- cargo').split()+['test','--offline','--locked','--manifest-path',str(root/'Cargo.toml'),'-p','campfire',test,'--','--test-threads=8']
        env=dict(os.environ,CARGO_BUILD_JOBS='2',CABLE_TEST_PORT_RANGE='53300-53399',INTEGRATION_TEST_PORT_RANGE='53300-53399')
        if case:env['WS16_RUN_HTTP_CASE']=case
        result=subprocess.run(command,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
        failure=next((line for line in result.stdout.splitlines() if line.startswith('test ') and test+' ... FAILED' in line),None)
        summary=next((line for line in result.stdout.splitlines() if line.startswith('test result:')),None)
        if result.returncode==0 or not failure or not summary:
            print(result.stdout);raise RuntimeError('mutation did not fail its assertion: '+str(case))
        print(failure);print(summary)
finally:
    for path,source in zip(paths,original):path.write_text(source)
print('Slack run mutation guards: member-admin and foreign-personal access, and plan bytes rejected; sources restored')
