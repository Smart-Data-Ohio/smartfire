#!/usr/bin/env python3
"""Compile agent authority/identity/outcome defects against the real domain and HTTP client."""
from pathlib import Path
import os
import subprocess
root=Path(__file__).resolve().parents[2]
env=os.environ.copy()
env.update(CI='1',TMPDIR=str(root.parent/'.scratch'),CARGO_TARGET_DIR=str(root/'.scratch/target'),GITHUB_TEST_PORT_RANGE='51550-51594',CABLE_TEST_PORT_RANGE='51550-51594')
mutants=[
 ('crates/campfire/src/controllers/presenters/link_embeds.rs','url: reference.display_url().into(),','url: embed.normalized_url.clone(),','campfire','ws15e_linkedin_real_fetches_and_room_html_use_each_reference'),
 ('crates/views/templates/linkedin/posts/_card.html','{{ title }}','{{ title|safe }}','campfire_views','ws15e_linkedin_cards_match_pinned_rails_bytes'),
]

for index,(file,old,new,package,test) in enumerate(mutants,1):
 path=root/file
 original=path.read_text();assert original.count(old)>=1,old
 try:
  path.write_text(original.replace(old,new))
  run=subprocess.run(['cargo','test','-j','4','-p',package,test,'--','--nocapture'],cwd=root,env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  (root.parent/'.scratch'/f'linkedin-mutation-{index}.log').write_text(run.stdout)
  assert run.returncode and 'test result: FAILED' in run.stdout,run.stdout[-2500:]
  print(f'{index} {file} {test}: '+next(line for line in run.stdout.splitlines() if line.startswith('test result:')),flush=True)
 finally:path.write_text(original)
print(f'WS15e LinkedIn mutation checks: {len(mutants)} detected, 0 survived')
