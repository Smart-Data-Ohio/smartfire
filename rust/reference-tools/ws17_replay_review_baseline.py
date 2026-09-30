#!/usr/bin/env python3
"""Replay committed regressions on the reviewed implementation, without changing its code."""
import re, subprocess, tarfile
from pathlib import Path
root=Path(__file__).resolve().parents[2]
dest=root/'.scratch/review-baseline-1021'
dest.mkdir(parents=True,exist_ok=True)
revision='1021be6a150c9c8ab6c04482108d6e9ad9606f6d'
archive=subprocess.Popen(['git','archive',revision],cwd=root,stdout=subprocess.PIPE)
with tarfile.open(fileobj=archive.stdout,mode='r|') as files:
    files.extractall(dest,filter='data')
assert archive.wait()==0
owned=['rust/crates/db/src/models/keyword_alert.rs','rust/crates/db/src/models/user_status_settings/writes.rs','rust/crates/db/src/models/push_subscription.rs','rust/crates/db/src/models/notification_push.rs']
for path in owned:
    assert (dest/path).read_bytes()==subprocess.check_output(['git','show',f'{revision}:{path}'],cwd=root)
print(f'Reviewed implementation verified: {revision}; 4 affected production files unchanged',flush=True)
for path in ['rust/crates/db/src/tests/ws17_review_test.rs','rust/crates/db/src/tests/ws17_endpoint_review_test.rs','rust/vectors/ws17_unicode.json','rust/vectors/ws17_endpoint_urls.json','rust/vectors/ws17_notification_push.json']:
    (dest/path).write_bytes((root/path).read_bytes())
modules=dest/'rust/crates/db/src/tests.rs'
modules.write_text(modules.read_text().replace('mod keyword_alert_test;', 'mod keyword_alert_test;\nmod ws17_review_test;\nmod ws17_endpoint_review_test;'))
command=['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j4','--manifest-path',str(dest/'rust/Cargo.toml'),'-p','campfire_db','ws17_review_','--','--test-threads=4']
result=subprocess.run(command,cwd=root,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
print(result.stdout,end='')
assert result.returncode==101 and re.search(r'test result: FAILED\. 0 passed; 7 failed; 0 ignored;', result.stdout), 'baseline must fail seven runtime assertions'
print('Failing-first gate: 7 real assertion failures against 1021be6a; no production-code mutation')
