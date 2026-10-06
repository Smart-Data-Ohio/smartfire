#!/usr/bin/env python3
"""Exercise real job registration, phase ordering and exception-class logs."""
import os
from pathlib import Path
import re
import subprocess
ROOT=Path(__file__).resolve().parents[2]
SCRATCH=ROOT/".scratch/ws13b-job-discrimination"
SCRATCH.mkdir(parents=True,exist_ok=True)
mutations=[
 ("invitation-payload-is-join", "db/src/models/huddle_notices.rs", "payload: push_payload(&caller, room.id, false)", "payload: push_payload(&caller, room.id, true)", "campfire_db", "tests::huddle_invitation_job_test", 3),
 ("missing-invitation-errors", "db/src/models/huddle_notices.rs", "Err(crate::Error::RecordNotFound(_)) => return Ok(())", 'Err(crate::Error::RecordNotFound(_)) => return Err(crate::Error::RecordNotFound("ActivityItem"))', "campfire_db", "tests::huddle_invitation_job_test::missing", 1),
 ("presence-handler-unregistered", "campfire/src/jobs/huddle.rs", "    registry.register(presence);", "    // Deliberately omit the presence handler.", "campfire", "channels::huddle_effects_tests::presence_job_fanout_and_missing_room_match_rails_counts", 1),
 ("join-handler-unregistered", "campfire/src/jobs/huddle.rs", "    registry.register(join);", "    // Deliberately omit the join handler.", "campfire", "jobs::tests::huddle_join_and_invitation_workers_persist_the_payload_for_ws17", 1),
 ("invitation-handler-unregistered", "campfire/src/jobs/huddle.rs", "    registry.register(invitation);", "    // Deliberately omit the invitation handler.", "campfire", "jobs::tests::huddle_join_and_invitation_workers_discard_missing_sources_successfully", 1),
 ("stale-phase-omitted", "campfire/src/jobs/huddle.rs", "if let Err(error) = end_stale_streams(db).await", "if let Err(error) = Result::<(), campfire_db::Error>::Ok(())", "campfire", "huddle::tests::one_huddle_pass_runs_all_three_phases_in_rails_order", 1),
 ("sql-error-class-lost", "campfire/src/jobs/huddle.rs", 'campfire_db::Error::Sqlite(_) => "ActiveRecord::StatementInvalid"', 'campfire_db::Error::Sqlite(_) => "StandardError"', "campfire", "huddle::tests::resolver_failure_preserves_prior_items_and_does_not_stop_cleanup", 1),
 ("exception-detail-logged", "campfire/src/jobs/huddle.rs", 'format!("Huddle {phase} failed: {class}")', 'format!("Huddle {phase} failed: {class}: {error}")', "campfire", "huddle::tests::reconciler_standard_error_messages_match_the_rails_oracle", 1),
]
environment=dict(os.environ,TMPDIR=str(ROOT/".scratch"),CARGO_TARGET_DIR=str(ROOT/"rust/target"),CI="1",CABLE_TEST_PORT_RANGE="53000-53049",MAIL_TEST_PORT_RANGE="53050-53099")
for name,file,before,after,package,test,expected in mutations:
 path=ROOT/"rust/crates"/file;original=path.read_text()
 try:
  assert original.count(before)==1,(name,original.count(before));path.write_text(original.replace(before,after,1))
  result=subprocess.run(["cargo","test","--locked","-j4","--manifest-path",str(ROOT/"rust/Cargo.toml"),"-p",package,test,"--","--nocapture","--test-threads=4"],cwd=ROOT,env=environment,capture_output=True,text=True)
  output=result.stdout+result.stderr;(SCRATCH/f"{name}.log").write_text(output)
  summaries=re.findall(r"^test result: FAILED\..*$",output,re.M);summary=next((s for s in summaries if f"{expected} failed;" in s),None)
  assert result.returncode!=0 and summary and "could not compile" not in output and "panicked at" in output,output[-5000:]
  print(f"{name}: {summary}",flush=True)
 finally:path.write_text(original)
print(f"WS13b job discrimination: {len(mutations)} compiled regressions detected; sources restored",flush=True)
