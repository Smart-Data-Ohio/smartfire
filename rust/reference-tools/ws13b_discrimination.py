#!/usr/bin/env python3
"""Require compiled regressions to fail assertions, then restore the source.

Run only without a concurrent Cargo process in this worktree. No fixtures or
expected values are changed. Logs are retained under .scratch/ws13b-discrimination.
"""
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
SCRATCH = ROOT / ".scratch/ws13b-discrimination"
SCRATCH.mkdir(parents=True, exist_ok=True)
GRANT = ROOT / "rust/crates/db/src/models/huddle_grant.rs"


def replace_once(source, before, after):
    assert source.count(before) == 1, before
    return source.replace(before, after, 1)


mutations = [
    ("membership-revocation-bypassed", "Self::revoke_scope(tx, \"membership_id\", id, true, config)", "Ok(())", "membership_revokes_only_target_grants"),
    ("session-revocation-bypassed", "Self::revoke_scope(tx, \"session_id\", id, true, config)", "Ok(())", "sign_out_revokes_every_room_only_for_that_session"),
    ("bulk-deletion-user-revocation-bypassed", "Self::revoke_scope(tx, \"user_id\", id, true, config)", "Ok(())", "deactivate_revokes_after_bulk_membership_and_session_deletion"),
    ("room-deletion-cleanup-bypassed", "HuddleCleanup::create_room_deletion(tx, &name, config.admin_configured)?;", "", "room_deletion_uses_one_room_cleanup"),
    ("cleanup-failure-swallowed", "                config.admin_configured,\n            )?;", "                config.admin_configured,\n            ).ok();", "cleanup_failure_rolls_back_membership_removal"),
    ("room-switch-live-grant-retained", "(room_id!=? AND last_seen_at>?))", "(room_id!=? AND last_seen_at>? AND 0=1))", "room_switch_revokes_only_the_sessions_live_grant"),
    ("gateway-role-mismatch-accepted", "OR m.stage_role IS ?)", "OR m.stage_role IS ? OR 1=1)", "gateway_check_revokes_missed_role_mismatch"),
    ("participants-not-deduplicated", "SELECT DISTINCT u.* FROM users u JOIN huddle_grants", "SELECT u.* FROM users u JOIN huddle_grants", "participants_deduplicate_sort_and_drop_revoked_quiet_grants"),
    ("reissue-timestamp-not-refreshed", "SET last_issued_at=?,updated_at=?", "SET last_issued_at=COALESCE(last_issued_at,?),updated_at=?", "create_and_reuse_stamp_last_issued_at"),
    ("participants-not-sorted", "users.sort_by_key(|u| u.name.to_lowercase());", "let _ = &mut users;", "participants_deduplicate_sort_and_drop_revoked_quiet_grants"),
]
environment = dict(os.environ, TMPDIR=str(ROOT / ".scratch"), CARGO_TARGET_DIR=str(ROOT / "rust/target"), CI="1")
for name, before, after, test in mutations:
    original = GRANT.read_text()
    try:
        GRANT.write_text(replace_once(original, before, after))
        module = "huddle_revocation_test" if name in {entry[0] for entry in mutations[:5]} else "huddle_grant_sequences_test"
        command = ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j4", "--manifest-path", str(ROOT / "rust/Cargo.toml"), "-p", "campfire_db", f"tests::{module}::{test}", "--", "--exact", "--nocapture"]
        result = subprocess.run(command, cwd=ROOT, env=environment, capture_output=True, text=True)
        output = result.stdout + result.stderr
        (SCRATCH / f"{name}.log").write_text(output)
        summaries = re.findall(r"^test result: FAILED\..*$", output, re.M)
        assert result.returncode != 0 and summaries and "could not compile" not in output, output[-5000:]
        assert "panicked at" in output, output[-5000:]
        print(f"{name}: {summaries[-1]}", flush=True)
    finally:
        GRANT.write_text(original)
print(f"WS13b discrimination: {len(mutations)} compiled regressions detected; sources restored", flush=True)
