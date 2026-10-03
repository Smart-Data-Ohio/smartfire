#!/usr/bin/env python3
"""Insert a real post after a rejected remind, preserving its error result.

The intended witness is the independently captured total-message count, not the
returned message. Restores source bytes even when the test fails.
"""
from pathlib import Path
import subprocess,sys
root=Path(__file__).resolve().parents[3]
escape='--expect-escape' in sys.argv
runner=sys.argv[1:]
if escape: runner.remove('--expect-escape')
if runner[:1]==['--']:runner=runner[1:]
p=root/'rust/crates/campfire/src/controllers/rooms/slash_commands.rs'
raw=p.read_bytes()
test=root/'rust/crates/campfire/src/controllers/message_features/slash_named_tests.rs'
test_raw=test.read_bytes()
marker='    let result = slash_commands::dispatch_in_user_time_zone(tx, context, text)?;'
assert raw.decode().count(marker)==1
try:
 if escape:
  marker='        assert_eq!(actual["message_counts"], row["message_counts"], "slash actual total-message counts differ from Rails: {name}");'
  assert test_raw.decode().count(marker)==1
  test.write_text(test_raw.decode().replace(marker,'        println!("Old returned-message comparator observed actual counts: {}", actual["message_counts"]);'))
 p.write_text(raw.decode().replace(marker,marker+'\n    if text.starts_with("/remind ") && result.kind == "error" {\n        campfire_db::Message::create(tx, campfire_db::models::message::NewMessage {\n            room_id: context.room_id, creator_id: context.user_id,\n            markdown_source: Some("producer control: rejected remind posted".into()),\n            ..Default::default()\n        })?;\n    }\n'))
 r=subprocess.run(runner+['test','--locked','-p','campfire','--bin','campfire','builtin_remind_rejects_unusable_input_without_posting','-j2','--','--test-threads=4','--nocapture'],cwd=root,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
 print(r.stdout,flush=True)
 if escape: assert r.returncode==0
 else: assert r.returncode!=0 and 'slash actual total-message counts differ from Rails' in r.stdout
 print('WS8bm2 slash count producer: '+('escaped old comparator' if escape else 'rejected at actual total-message counts'),flush=True)
finally:
 p.write_bytes(raw)
 test.write_bytes(test_raw)
