#!/usr/bin/env python3
"""Prove selected WS8 regressions fail real tests; restore sources after every mutation.

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



APP=ROOT/"rust/crates/campfire/src"
check("room-start", {MODELS/"room_delete.rs": lambda s: replace_body(s,"pub fn begin_destroy(","Ok(())")}, "room_delete_test", ["room_begin_destroy_matches_rails"])
check("room-worker", {MODELS/"room_delete.rs": lambda s: replace_body(s,"pub async fn perform_with_config(","Ok(())")}, "room_delete_test", ["room_destroy_cascades_match_rails","room_workers_ignore_live_missing_and_already_destroyed_rooms","room_destroy_commits_progress_and_refreshes_claim_before_retry"])
check("room-live-guard", {MODELS/"room_delete.rs": lambda s: s.replace("?.filter(Room::deleted)","?")}, "room_workers_ignore_live_missing_and_already_destroyed_rooms", ["room_workers_ignore_live_missing_and_already_destroyed_rooms"])
check("room-sweep", {MODELS/"room_delete.rs": lambda s: replace_body(s,"pub fn reenqueue_stuck(","Ok(0)")}, "stuck_room_claims_match_rails_and_competing_writers", ["stuck_room_claims_match_rails_and_competing_writers"])
check("room-readers", {MODELS/"room.rs": lambda s: s.replace('WHERE "deleted_at" IS NULL','').replace('WHERE "rooms"."deleted_at" IS NULL AND "rooms"."type"','WHERE "rooms"."type"')}, "room_directory_scopes_exclude_deleted_rooms", ["room_directory_scopes_exclude_deleted_rooms"])
check("retention", {MODELS/"retention.rs": lambda s: replace_body(s,"pub async fn perform(","Ok(())")}, "retention_test", ["retention_survivors_and_unlinked_grants_match_rails"])
check("room-enqueue-atomicity", {MODELS/"room_delete.rs": lambda s: s.replace("tx.emit_after_commit(Event::job(&DestroyJob { room_id: room.id }));","").replace("tx.emit_after_commit(Event::job(&DestroyJob{room_id:id}));","")}, "ws8_messaging_test", ["room_mark_and_memberships_roll_back_with_a_failed_destroy_enqueue","stuck_room_claim_rolls_back_with_a_failed_destroy_enqueue"], "campfire_jobs")
check("maintenance-registry", {APP/"jobs/messaging.rs": lambda s: replace_body(s,"pub(super) fn register(","")}, "ws8_room_and_retention_workers_run_in_the_real_app", ["ws8_room_and_retention_workers_run_in_the_real_app"], "campfire")
check("cleanup-enqueue-atomicity", {MODELS/"room_delete.rs": lambda s: s.replace("tx.emit_after_commit(Event::job(&CleanupJob { cleanup_id: id }));","")}, "room_mark_and_memberships_roll_back_with_a_failed_destroy_enqueue", ["room_mark_and_memberships_roll_back_with_a_failed_destroy_enqueue"], "campfire_jobs")
check("sla-dependent-inbox", {MODELS/"channel_thread.rs": lambda s: s.replace("source_type='BoardSlaNudge'","source_type='NoNudges'")}, "room_destroy_cascades_match_rails", ["room_destroy_cascades_match_rails"])
print("WS8 maintenance discrimination: 10 mutations detected; sources restored",flush=True)
