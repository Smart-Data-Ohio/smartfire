#!/usr/bin/env python3
"""Inject compiled invitation regressions, require assertion failures, restore."""
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
SCRATCH = ROOT / ".scratch/ws13b-sequence-discrimination"
SCRATCH.mkdir(parents=True, exist_ok=True)
MODEL = ROOT / "rust/crates/db/src/models"
mutations = [
    ("issuance-stamp-corrupted", "huddle_grant.rs", "server_muted, now, now, now],", "server_muted, now.ago(SignedDuration::from_secs(1)), now, now],", "huddle_invitation_sequences_test", 38),
    ("dedup-window-reversed", "huddle_invitations.rs", "SignedDuration::from_secs(120)", "SignedDuration::from_secs(-1)", "huddle_invitation_sequences_test::disabled_items_multiple_devices_window", 1),
    ("owned-history-ignored", "huddle_invitations.rs", "if let Some(item) = owned", "if let Some(item) = owned.filter(|_| false)", "huddle_invitation_sequences_test::original_device_owned_item_priority", 1),
    ("joining-does-not-handle", "huddle_invitations.rs", "ActivityItem::find(tx.conn(), id)?.mark_handled(tx)?;", "let _ = id;", "huddle_invitation_sequences_test::join_handles_invitation", 1),
    ("group-ring-ended-prematurely", "huddle_notices.rs", "    if others {\n        return Ok(());\n    }", "    if false && others {\n        return Ok(());\n    }", "huddle_invitation_sequences_test::group_starter_leaves_no_ended", 1),
    ("silent-ring-flag-reversed", "huddle_invitations.rs", "Value::Bool(!sound_allowed)", "Value::Bool(sound_allowed)", "huddle_invitation_sequences_test::quiet_ring_keeps_item", 1),
    ("notifier-issuance-stamp-corrupted", "huddle_grant.rs", "server_muted, now, now, now],", "server_muted, now.ago(SignedDuration::from_secs(1)), now, now],", "huddle_notifier_sequences_test", 32),
    ("bot-join-accepted", "huddle_notices.rs", "u.is_active() && !u.is_bot()", "u.is_active()", "huddle_notifier_sequences_test::bot_join_silent", 1),
    ("recent-ring-suppression-removed", "huddle_notices.rs", "|| rung.contains(&viewer.id)", "|| (false && rung.contains(&viewer.id))", "huddle_notifier_sequences_test::live_ring_suppresses_join", 1),
    ("old-revoke-marked-as-rejoin", "huddle_notices.rs", "SignedDuration::from_secs(5)", "SignedDuration::from_secs(30)", "huddle_notifier_sequences_test::ten_second_revoke_no_rejoin_mark", 1),
    ("stale-job-live-check-bypassed", "huddle_notices.rs", "if grant.revoked() || !grant.in_call(tx.now())", "if false && (grant.revoked() || !grant.in_call(tx.now()))", "huddle_notifier_sequences_test::join_job_after_leave_silent", 1),
]
environment = dict(os.environ, TMPDIR=str(ROOT / ".scratch"), CARGO_TARGET_DIR=str(ROOT / "rust/target"), CI="1")
for name, file, before, after, test, expected_failures in mutations:
    path = MODEL / file
    original = path.read_text()
    try:
        assert original.count(before) == 1, (name, original.count(before))
        path.write_text(original.replace(before, after, 1))
        command = ["cargo", "test", "--locked", "-j4", "--manifest-path", str(ROOT / "rust/Cargo.toml"), "-p", "campfire_db", f"tests::{test}", "--", "--nocapture", "--test-threads=4"]
        result = subprocess.run(command, cwd=ROOT, env=environment, capture_output=True, text=True)
        output = result.stdout + result.stderr
        (SCRATCH / f"{name}.log").write_text(output)
        summaries = re.findall(r"^test result: FAILED\..*$", output, re.M)
        assert result.returncode != 0 and summaries and "could not compile" not in output, output[-5000:]
        summary = next((s for s in summaries if f"{expected_failures} failed;" in s), None)
        assert summary and "panicked at" in output, output[-5000:]
        print(f"{name}: {summary}", flush=True)
    finally:
        path.write_text(original)
print(f"WS13b sequence discrimination: {len(mutations)} compiled regressions detected; sources restored", flush=True)
