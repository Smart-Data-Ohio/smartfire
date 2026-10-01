#!/usr/bin/env python3
from pathlib import Path
import os,subprocess
root=Path(__file__).resolve().parents[3];p=root/'rust/crates/db/src/models/user.rs';original=p.read_text()
before='secure_compare(digest.as_bytes(), digest_bot_token(token).as_bytes()).then_some(bot)';assert original.count(before)==1
log=root/'.scratch/bot-case-mutation.log'
try:
 p.write_text(original.replace(before,'(secure_compare(digest.as_bytes(), digest_bot_token(token).as_bytes()) || true).then_some(bot)'))
 with log.open('w') as out:
  run=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j4','--manifest-path','rust/Cargo.toml','-p','campfire_db','ws11_bot_case_wrong_empty_and_malformed_keys_refused','--','--test-threads=4'],cwd=root,env=dict(os.environ,CI='1',TMPDIR=str(root/'.scratch'),CARGO_TARGET_DIR=str(root/'rust/target')),stdout=out,stderr=subprocess.STDOUT)
 assert run.returncode and 'test result: FAILED. 0 passed; 1 failed;' in log.read_text(),log
 print('WS11 bot mutation: wrong digest accepted; 1 test failed; original restored')
finally:p.write_text(original)
