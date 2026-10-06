#!/usr/bin/env python3
"""Run real assertions against broken freshness, grouping and atomic queue writes."""
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
           MAIL_TEST_PORT_RANGE="53400-53419", GITHUB_TEST_PORT_RANGE="53450-53499")

mutations = [
    ("stale-owner", "crates/db/src/models/channel_thread/work.rs",
     "let mut fresh = Self::find(tx.conn(), id)?;", "let mut fresh = self.clone(); let _ = id;",
     "campfire_db", "fresh_ownership_is_rechecked_and_stale_other_columns_survive"),
    ("group-repoint", "crates/db/src/models/activity_item/recorder.rs",
     "WHERE user_id=? AND handled_at IS NULL AND event_type=?", "WHERE user_id=? AND handled_at IS NOT NULL AND event_type=?",
     "campfire_db", "work_updates_repoint_one_unhandled_item_and_handled_updates_start_a_new_item"),
    ("atomic-job", "crates/db/src/database.rs",
     '    conn.execute_batch("BEGIN IMMEDIATE TRANSACTION")?;', '    conn.execute_batch("BEGIN IMMEDIATE TRANSACTION; COMMIT TRANSACTION")?;',
     "campfire", "board_creation_rolls_back_every_row_when_the_atomic_job_insert_fails"),
]
for name, relative, original, broken, package, test in mutations:
    path = root / relative
    source = path.read_text()
    assert source.count(original) == 1, name
    try:
        changed = source.replace(original, broken, 1)
        if name == "atomic-job":
            # Keep setup writes operational while removing the enclosing transaction.
            # Model savepoints still work; releasing one now commits the primary rows
            # before run_write sees the durable enqueue's recorded error.
            commit = 'if let Err(error) = conn.execute_batch("COMMIT TRANSACTION") {'
            assert changed.count(commit) == 1
            changed = changed.replace(commit, 'if let Err(error) = if conn.is_autocommit() { Ok(()) } else { conn.execute_batch("COMMIT TRANSACTION") } {', 1)
        path.write_text(changed)
        command = ["cargo", "test", "--locked", "-p", package, test, "--", "--test-threads=4"]
        result = subprocess.run(command, cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        (logs / f"write-mutant-{name}.log").write_text(result.stdout)
        assert result.returncode != 0 and "test result: FAILED." in result.stdout and "panicked at" in result.stdout and "assertion" in result.stdout, result.stdout[-4000:]
        assert "could not compile" not in result.stdout, result.stdout[-4000:]
        print(f"{name}: actual assertion rejected broken implementation", flush=True)
        print(next(line for line in result.stdout.splitlines() if line.startswith("test result: FAILED.")), flush=True)
    finally:
        path.write_text(source)
print("WS12 board write discriminators: 3 broken implementations rejected at actual assertions; 0 compile/setup failures; sources restored")
