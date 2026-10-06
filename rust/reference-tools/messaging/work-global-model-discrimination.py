#!/usr/bin/env python3
"""The stale-instance declaration must detect events produced for other threads."""
import argparse
import os
import shutil
from pathlib import Path
import subprocess
import sys

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--root',type=Path,default=Path(__file__).resolve().parents[3])
parser.add_argument('--expect-escape',action='store_true')
args=parser.parse_args()
root=args.root.resolve()
sys.path.insert(0,str(root/'rust/reference-tools/messaging'))
from browser_host import prepare_source
generated=prepare_source(root)
# A DB-only cfg(test) assertion reads this tracked WS17 Ruby input. The
# ordinary browser host compiles DB as a library, so it does not need it.
relative=Path('reference-tools/ws17_notification_push.rb')
assert subprocess.check_output(['git','ls-files','--error-unmatch','rust/'+str(relative)],cwd=root)
(generated/relative).parent.mkdir(parents=True,exist_ok=True)
shutil.copyfile(root/'rust'/relative,generated/relative)
# Rails' preceding omitted-owner declaration already leaves a second thread.
# Seed the equivalent foreign thread when running the old standalone Rust test.
test_source=generated/'crates/db/src/tests/work_mutations_test.rs'
test_original=test_source.read_text()
test_anchor='fn message_controller_separate_stale_work_changes_match_rails_history() {\n    let t = channel_thread_test::frozen();'
assert test_original.count(test_anchor)==1
if 'Some("Foreign work history")' not in test_original:
    test_source.write_text(test_original.replace(test_anchor,test_anchor+'\n    channel_thread_test::create_thread(&t, "designers", "jz", None, Some("Foreign work history"));'))
source=generated/'crates/db/src/models/channel_thread/work.rs'
original=source.read_text()
needle='WorkThreadEvent::create_for_change(tx, &before, &fresh, Some(actor), None)?;'
assert original.count(needle)==1
replacement=needle+'''
            let inserted = tx.conn().execute("INSERT INTO work_thread_events (actor_id,channel_thread_id,created_at,event_type,from_status,to_status,updated_at) SELECT ?1,id,CURRENT_TIMESTAMP,'work_update','planned','in_progress',CURRENT_TIMESTAMP FROM channel_threads WHERE id<>?2 LIMIT 1", (actor.id, id))?;
            assert_eq!(inserted, 1, "foreign-event mutant must insert its row");
'''
env=dict(os.environ,CARGO_BUILD_JOBS='2',RUST_TEST_THREADS='8',CAMPFIRE_REFERENCE=str(root),TMPDIR=str(root/'.scratch'))
command=['cargo','test','--locked',
    '--manifest-path',str(generated/'Cargo.toml'),'-p','campfire_db',
    'message_controller_separate_stale_work_changes_match_rails_history','--','--test-threads=8']
try:
    source.write_text(original.replace(needle,replacement))
    result=subprocess.run(command,cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
finally:
    source.write_text(original)
    test_source.write_text(test_original)
# This copy shares the caller's target. Rebuild the restored producer before
# exiting: Cargo can otherwise reuse the mutant executable across identical
# workspace package identities. A failed restored control invalidates the proof.
control=subprocess.run(command,cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
print(result.stdout,end='')
print('WS8bm restored model control:')
print(control.stdout,end='')
valid=(result.returncode==0 and '1 passed; 0 failed;' in result.stdout) if args.expect_escape else (
    result.returncode!=0 and 'global work event delta' in result.stdout and '0 passed; 1 failed;' in result.stdout)
valid &= control.returncode==0 and '1 passed; 0 failed;' in control.stdout
print('WS8bm global model discrimination: '+('foreign-event producer ESCAPED at baseline' if args.expect_escape and valid else 'foreign-event producer REJECTED at global event delta' if valid else 'INVALID or unexpected result'))
sys.exit(not valid)
