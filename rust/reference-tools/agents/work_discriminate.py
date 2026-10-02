#!/usr/bin/env python3
"""Prove new work policies and transaction tests reject actual broken implementations."""
from pathlib import Path
import os
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root.parent / ".scratch"
logs = scratch / "logs"
logs.mkdir(parents=True, exist_ok=True)
env = os.environ.copy()
env.update(CARGO_BUILD_JOBS="2", CARGO_TARGET_DIR=str(scratch / "target"),
           TMPDIR=str(scratch), CI="1", CABLE_TEST_PORT_RANGE="53420-53449",
           MAIL_TEST_PORT_RANGE="53400-53419", GITHUB_TEST_PORT_RANGE="53450-53499",
           LD_LIBRARY_PATH=str(scratch / "rails-media/native-libs"))
env["PATH"] = str(scratch / "rails-media/usr/bin") + ":" + env["PATH"]
vector_test = "ws12_agent_work_writes_and_denials_match_rails_services"
mutations = [
    ("stale-owner", "crates/db/src/models/channel_thread/agent_work.rs",
     "if thread.work() && thread.work_owner_id == Some(agent.user_id) {",
     "if thread.work() && agent.user_id != 0 {", "campfire_db",
     "agent_model_rechecks_stale_ownership_for_writes_and_keeps_the_rails_result_noop"),
    ("sender-manage", "crates/db/src/models/agent_work.rs",
     'if !agent.can(conn, "manage_threads", Some(thread.room_id))? {',
     'if false && !agent.can(conn, "manage_threads", Some(thread.room_id))? {',
     "campfire_db", vector_test),
    ("receiver-read", "crates/db/src/models/work_handoff.rs",
     'if !agent.can(conn, "read_messages", Some(thread.room_id))? {',
     'if false && !agent.can(conn, "read_messages", Some(thread.room_id))? {',
     "campfire_db", vector_test),
    ("rule-read", "crates/db/src/models/board_tag_assignment.rs",
     '&& agent.can(conn, "read_messages", Some(room_id))?',
     '&& (true || agent.can(conn, "read_messages", Some(room_id))?)',
     "campfire_db", "auto_assignment_rechecks_agent_membership_activity_and_both_grants"),
    ("rule-dirty-columns", "crates/db/src/models/board_tag_assignment.rs",
     '(false, true) => tx.conn().execute(\n                    "UPDATE board_tag_assignments SET assignee_id=?,updated_at=? WHERE id=?",\n                    params![self.assignee_id, self.updated_at, self.id],',
     '(false, true) => tx.conn().execute(\n                    "UPDATE board_tag_assignments SET tag=?,assignee_id=?,updated_at=? WHERE id=?",\n                    params![self.tag, self.assignee_id, self.updated_at, self.id],',
     "campfire_db", "stale_rule_edits_preserve_the_other_writers_dirty_columns"),
    ("opener-order", "crates/db/src/models/channel_thread/work.rs",
     '                        true,\n                    )',
     '                        false,\n                    )', "campfire_db", vector_test),
    ("atomic-handoff-job", "crates/db/src/database.rs",
     '    conn.execute_batch("BEGIN IMMEDIATE TRANSACTION")?;',
     '    conn.execute_batch("BEGIN IMMEDIATE TRANSACTION; COMMIT TRANSACTION")?;',
     "campfire", "handoff_package_owner_history_ledger_audit_and_webhook_jobs_are_atomic"),
]
for name, relative, original, broken, package, test in mutations:
    path = root / relative
    source = path.read_text()
    assert source.count(original) == 1, name
    changed = source.replace(original, broken, 1)
    if name == "opener-order":
        enqueue = '                crate::models::agent_delivery::enqueue_for_message(tx, &message)?;\n'
        assert changed.count(enqueue) == 1
        changed = changed.replace(enqueue, "", 1)
    if name == "atomic-handoff-job":
        commit = 'if let Err(error) = conn.execute_batch("COMMIT TRANSACTION") {'
        assert changed.count(commit) == 1
        changed = changed.replace(commit, 'if let Err(error) = if conn.is_autocommit() { Ok(()) } else { conn.execute_batch("COMMIT TRANSACTION") } {', 1)
    try:
        path.write_text(changed)
        command = ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-p", package, test, "--", "--test-threads=4"]
        result = subprocess.run(command, cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        (logs / f"agent-work-mutant-{name}.log").write_text(result.stdout)
        assert result.returncode != 0 and "test result: FAILED." in result.stdout and "panicked at" in result.stdout and "assertion" in result.stdout, result.stdout[-4000:]
        assert "could not compile" not in result.stdout, result.stdout[-4000:]
        print(f"{name}: actual assertion rejected broken implementation", flush=True)
        print(next(line for line in result.stdout.splitlines() if line.startswith("test result: FAILED.")), flush=True)
    finally:
        path.write_text(source)
print("WS12 agent work discriminators: 7 broken implementations rejected at actual assertions; 0 compile/setup failures; sources restored")
