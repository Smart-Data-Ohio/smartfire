#!/usr/bin/env python3
"""Prove round-2 WS8 regressions fail real tests; restore sources after every mutation.

Run from the WS8 worktree with no other build/edit process active. Logs and temp databases use
the worktree's .scratch. A compilation failure never counts as detecting a regression.
"""
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
SCRATCH = ROOT / ".scratch" / "discrimination"
SCRATCH.mkdir(parents=True, exist_ok=True)
ENV = dict(os.environ, TMPDIR=str(ROOT / ".scratch"), CARGO_TARGET_DIR=str(ROOT / "rust/target"))
MODELS = ROOT / "rust/crates/db/src/models"


def replace_body(source, marker, body):
    start = source.index("{", source.index(marker))
    depth = 1
    end = start + 1
    while depth:
        depth += (source[end] == "{") - (source[end] == "}")
        end += 1
    return source[:start] + "{\n" + body + "\n}" + source[end:]


def check(name, changes, test_filter, must_fail, package="campfire_db"):
    original = {path: path.read_text() for path in changes}
    try:
        for path, mutate in changes.items():
            broken = mutate(original[path])
            assert broken != original[path], f"mutation did not change {path}"
            path.write_text(broken)
        run = subprocess.run(
            ["cargo", "test", "-j", "4", "-p", package, test_filter],
            cwd=ROOT / "rust", env=ENV, capture_output=True, text=True,
        )
        output = run.stdout + run.stderr
        (SCRATCH / f"{name}.log").write_text(output)
        failed = re.findall(r"^test (\S+) \.\.\. FAILED$", output, re.M)
        summary = re.findall(r"^test result: FAILED\..*$", output, re.M)
        assert run.returncode != 0 and summary, f"{name}: did not produce failing tests (see log)"
        for suffix in must_fail:
            assert any(test.endswith("::" + suffix) for test in failed), f"{name}: {suffix} did not fail"
        print(f"{name}: detected ({len(failed)} failing tests)", flush=True)
        print(summary[0], flush=True)
    finally:
        for path, source in original.items():
            path.write_text(source)



check("round2-sync-dependencies", {MODELS/"room_delete.rs": lambda s: s.replace('for table in ["messages", "channel_threads", "events"] {', 'tx.conn().execute_cached("UPDATE huddle_cleanups SET huddle_grant_id=NULL WHERE huddle_grant_id IN (SELECT id FROM huddle_grants WHERE room_id=?)",[room.id])?;\n    for table in CHILDREN {')}, "synchronous_room_destroy_matches_rails_dependencies", ["synchronous_room_destroy_matches_rails_dependencies"])
check("round2-async-dependencies", {MODELS/"room_delete.rs": lambda s: replace_body(s,"pub async fn perform_with_config(","Ok(())")}, "asynchronous_room_destroy_matches_rails_preliminary_deletions", ["asynchronous_room_destroy_matches_rails_preliminary_deletions"])
check("round2-room-resume", {MODELS/"room_delete.rs": lambda s: replace_body(s,"pub async fn perform_with_config(","Ok(())")}, "room_501_messages_resume_matches_rails_progress", ["room_501_messages_resume_matches_rails_progress"])
check("round2-retention-resume", {MODELS/"retention.rs": lambda s: replace_body(s,"pub async fn perform(","Ok(())")}, "retention_1001_grants_resume_matches_rails_progress", ["retention_1001_grants_resume_matches_rails_progress"])
check("round2-create-postcommit", {MODELS/"message.rs": lambda s: s.replace("message.create_in_index(tx)?;\n            message.receive_in_conversation(tx)?;\n            crate::models::message_reference::sync(tx, &message)?;", "let committed=message.clone();\n            tx.after_commit(move |tx| { committed.create_in_index(tx)?; committed.receive_in_conversation(tx)?; crate::models::message_reference::sync(tx, &committed) });")}, "round2_test", ["renderer_failure_after_preflight_rolls_back_message", "unread_failure_rolls_back_message_and_index", "reference_failure_rolls_back_message_index_and_unread"])
check("round2-edit-postcommit", {MODELS/"message.rs": lambda s: s.replace("self.update_in_index(tx)?;\n            if references_changed { crate::models::message_reference::sync(tx, self)?; }", "let committed=self.clone();\n            tx.after_commit(move |tx| { committed.update_in_index(tx)?; if references_changed { crate::models::message_reference::sync(tx, &committed)?; } Ok(()) });")}, "edit_index_failure_rolls_back_body_and_timestamps", ["edit_index_failure_rolls_back_body_and_timestamps"])
check("round2-touch-postcommit", {MODELS/"message.rs": lambda s: s.replace("if !self.streaming { self.update_in_index(tx)?; }", "if !self.streaming { let committed=self.clone(); tx.after_commit(move |tx| committed.update_in_index(tx)); }")}, "touch_index_failure_rolls_back_timestamps", ["touch_index_failure_rolls_back_timestamps"])
check("round2-callback-order", {MODELS/"message.rs": lambda s: s.replace("message.push_later_in_conversation(tx);", "").replace("Ok(message)\n    }", "if !message.streaming { message.push_later_in_conversation(tx); }\n        Ok(message)\n    }")}, "successful_bookkeeping_and_callback_order_match_rails", ["successful_bookkeeping_and_callback_order_match_rails"])
print("WS8 round-2 discrimination: 8 mutations detected; sources restored", flush=True)
