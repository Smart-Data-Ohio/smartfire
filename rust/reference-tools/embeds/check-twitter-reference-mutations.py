#!/usr/bin/env python3
"""Compile missing-hook, repeated-enqueue and quiet-backfill defects against real WS8 writes."""
from pathlib import Path
import os
import subprocess
root=Path(__file__).resolve().parents[2]
env=os.environ.copy()
env.update(CI='1',TMPDIR=str(root.parent/'.scratch'),CARGO_TARGET_DIR=str(root/'.scratch/target'),GITHUB_TEST_PORT_RANGE='51550-51594',CABLE_TEST_PORT_RANGE='51550-51594')
mutants=[
 ('crates/campfire/src/integrations.rs','twitter::references::sync_message(tx, message, enqueue)','Ok(())','campfire','ws15e_x_ws8_create_edit_code_retry_and_quiet_references'),
 ('crates/campfire/src/integrations/twitter/references.rs','enqueue && inserted &&','enqueue &&','campfire','ws15e_x_ws8_create_edit_code_retry_and_quiet_references'),
 ('crates/campfire/src/integrations/twitter/references.rs','sync_message(tx, &message, enqueue)?','sync_message(tx, &message, true)?','campfire','ws15e_x_backfill_reconciles_markdown_legacy_and_missing_richtext_idempotently'),
]

for index,(file,old,new,package,test) in enumerate(mutants,1):
 path=root/file
 original=path.read_text();assert original.count(old)>=1,old
 try:
  path.write_text(original.replace(old,new))
  run=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','-j','4','-p',package,test,'--','--nocapture'],cwd=root,env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  (root.parent/'.scratch'/f'twitter-reference-mutation-{index}.log').write_text(run.stdout)
  assert run.returncode and 'test result: FAILED' in run.stdout,run.stdout[-2500:]
  print(f'{index} {file} {test}: '+next(line for line in run.stdout.splitlines() if line.startswith('test result:')),flush=True)
 finally:path.write_text(original)
print(f'WS15e X reference mutation checks: {len(mutants)} detected, 0 survived')
