#!/usr/bin/env python3
"""Compile each broken owner API, require its actual parity regression to fail."""
from pathlib import Path
import os, re, subprocess
root=Path(__file__).resolve().parents[3]
env=dict(os.environ,CARGO_BUILD_JOBS='2',TMPDIR=str(root/'.scratch'),CARGO_TARGET_DIR=str(root/'rust/target'))
cases=[
 ('icon','user/icon.rs',"let name = strip(strip(name.unwrap_or_default()).trim_matches(':'));","let name = strip(name.unwrap_or_default());",'ws11_ui_owner_icons_match_rails_create_update_clear_and_avatar_stamp'),
 ('cap','agent/cap_input.rs','if value.is_null()', 'if true || value.is_null()','ws11_ui_owner_raw_caps_preserve_rails_cast_errors_and_form_values'),
 ('secret','agent.rs',r'conn\.query_row\(\s*"SELECT webhook_signing_secret FROM agents WHERE id=\?",\s*\[self\.id\],\s*\|r\|\s*r\.get\(0\),?\s*\)\?', '{ conn.execute("UPDATE agents SET updated_at=updated_at WHERE id=?", [self.id])?; conn }.query_row("SELECT webhook_signing_secret FROM agents WHERE id=?", [self.id], |r|r.get(0))?', 'ws11_ui_owner_signing_secret_getter_never_writes_even_blank'),
]
for name,file,before,after,test in cases:
 p=root/'rust/crates/db/src/models'/file;original=p.read_text(); pattern=before if name=='secret' else re.escape(before);assert len(re.findall(pattern,original))==1,name
 log=root/'.scratch'/f'ui-owner-mutation-{name}.log'
 try:
  p.write_text(re.sub(pattern,lambda _:after,original))
  with log.open('w') as out:
   run=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j2','--manifest-path','rust/Cargo.toml','-p','campfire_db',test,'--','--test-threads=4'],cwd=root,env=env,stdout=out,stderr=subprocess.STDOUT)
  summaries=[line for line in log.read_text().splitlines() if line.startswith('test result:')]
  assert run.returncode==101 and any('FAILED. 0 passed; 1 failed;' in line for line in summaries),(name,log)
  print(f'WS11 UI owner mutation: {name}; exit={run.returncode}; '+summaries[-1],flush=True)
 finally:p.write_text(original)
print('WS11 UI owner mutations: 3 compiled mutations caught; source restored')
