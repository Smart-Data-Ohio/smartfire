#!/usr/bin/env python3
"""Require compiled push gate/throttle regressions to fail, then restore."""
import os
from pathlib import Path
import re
import subprocess
ROOT = Path(__file__).resolve().parents[2]
SCRATCH = ROOT / ".scratch/ws13b-push-discrimination"
SCRATCH.mkdir(parents=True,exist_ok=True)
path = ROOT / "rust/crates/db/src/models/notification_push.rs"
mutations = [
 ("throttle-stamp-corrupted", "SET last_huddle_join_push_at=? WHERE id=?", "SET last_huddle_join_push_at=datetime(?,'-1 second') WHERE id=?", "huddle_join_push_sequences_test", 5),
 ("throttle-window-shortened", "now.since(-SignedDuration::from_secs(10*60))", "now.since(SignedDuration::from_secs(1))", "huddle_join_push_sequences_test::second_push_throttled", 1),
 ("connected-scope-bypassed", "connected_at IS NULL OR connected_at<?", "connected_at IS NULL OR connected_at<? OR 1=1", "huddle_join_push_sequences_test::connected_does_not_claim", 1),
 ("disabled-invitation-gate-bypassed", "!eligible || !huddle_inbox_enabled(conn, recipient_id)?", "!eligible || false", "huddle_join_push_sequences_test::disabled_invitations_do_not_claim", 1),
 ("empty-subscriptions-claim-window", "!join || !subscriptions.is_empty()", "true", "huddle_join_push_sequences_test::empty_subscriptions_do_not_claim", 1),
]
environment=dict(os.environ,CARGO_BUILD_JOBS="2",TMPDIR=str(ROOT/".scratch"),CARGO_TARGET_DIR=str(ROOT/"rust/target"),CI="1")
for name,before,after,test,expected in mutations:
 original=path.read_text()
 try:
  assert original.count(before)==1,(name,original.count(before))
  path.write_text(original.replace(before,after,1))
  result=subprocess.run(["cargo","test","--locked","--manifest-path",str(ROOT/"rust/Cargo.toml"),"-p","campfire_db",f"tests::{test}","--","--nocapture","--test-threads=8"],cwd=ROOT,env=environment,capture_output=True,text=True)
  output=result.stdout+result.stderr;(SCRATCH/f"{name}.log").write_text(output)
  summaries=re.findall(r"^test result: FAILED\..*$",output,re.M)
  summary=next((s for s in summaries if f"{expected} failed;" in s),None)
  assert result.returncode!=0 and summary and "could not compile" not in output and "panicked at" in output,output[-5000:]
  print(f"{name}: {summary}",flush=True)
 finally:path.write_text(original)
print(f"WS13b push discrimination: {len(mutations)} compiled regressions detected; sources restored",flush=True)
