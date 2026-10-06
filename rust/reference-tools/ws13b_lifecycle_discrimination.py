#!/usr/bin/env python3
"""Show lifecycle, reachability and new membership validations failing."""
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
SCRATCH = ROOT / ".scratch/ws13b-lifecycle-discrimination"
SCRATCH.mkdir(parents=True, exist_ok=True)
MODEL = ROOT / "rust/crates/db/src/models"
mutations = [
    ("stream-start-stamp-corrupted", "stream.rs", "started_at.unwrap_or(tx.now())", "started_at.unwrap_or(tx.now().ago(jiff::SignedDuration::from_secs(1)))", "huddle_domain_lifecycle_sequences_test", 12),
    ("room-type-scope-reversed", "room.rs", 'r#"SELECT * FROM "rooms" WHERE "rooms"."deleted_at" IS NULL AND "rooms"."type" = ?"#,\n            [room_type],\n            Self::from_row,', 'r#"SELECT * FROM "rooms" WHERE "rooms"."deleted_at" IS NULL AND "rooms"."type" != ?"#,\n            [room_type],\n            Self::from_row,', "huddle_domain_lifecycle_sequences_test", 2),
    ("new-member-stage-default-removed", "membership.rs", "let stage_role = room.as_ref().filter(|room| room.stage()).map(|_| StageRole::Listener);", "let stage_role: Option<StageRole> = None;", "huddle_domain_lifecycle_sequences_test::stage_later_member", 1),
    ("message-reachability-crosses-membership", "message.rs", 'r#"{SELECT_REACHABLE} WHERE "rooms"."deleted_at" IS NULL AND "memberships"."user_id" = ? AND "messages"."id" = ? LIMIT 1"#', 'r#"{SELECT_REACHABLE} WHERE "rooms"."deleted_at" IS NULL AND (? IS NOT NULL) AND "messages"."id" = ? LIMIT 1"#', "huddle_domain_lifecycle_sequences_test", 2),
    ("deactivation-keeps-call-memberships", "user.rs", 'WHERE "memberships"."user_id" = ? AND "room"."type" != ?)', 'WHERE "memberships"."user_id" = ? AND "room"."type" != ? AND 0=1)', "huddle_domain_lifecycle_sequences_test", 6),
    ("membership-association-validation-bypassed", "membership.rs", "errors.into_result()?; // Single-membership creation validates both associations.", "let _ = errors; // Deliberate regression: accept missing associations.", "huddle_membership_creation_test", 3),
    ("inactive-direct-create-callback-added", "membership.rs", "        Self::find(tx.conn(), id)\n    }\n\n    pub fn count", "        if let Some(room) = room.filter(Room::direct) { room.refresh_direct_member_key(tx)?; }\n        Self::find(tx.conn(), id)\n    }\n\n    pub fn count", "huddle_membership_creation_test::direct", 1),
    ("admin-successor-preference-removed", "stage.rs", "if user.is_active() && user.is_administrator()", "if false && user.is_active() && user.is_administrator()", "huddle_domain_lifecycle_sequences_test::destroy_admin_successor", 1),
]
environment = dict(os.environ, TMPDIR=str(ROOT / ".scratch"), CARGO_TARGET_DIR=str(ROOT / "rust/target"), CI="1")
detected = set()
for name, file, before, after, test, expected in mutations:
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
        summary = next((s for s in summaries if f"{expected} failed;" in s), None)
        assert result.returncode != 0 and summary and "could not compile" not in output and "panicked at" in output, output[-5000:]
        detected.update(re.findall(r"test tests::huddle_domain_lifecycle_sequences_test::(\w+) \.\.\. FAILED",output))
        print(f"{name}: {summary}", flush=True)
    finally:
        path.write_text(original)
assert len(detected) == 20, detected
print(f"WS13b lifecycle discrimination: {len(mutations)} compiled regressions detected; all 20 lifecycle traces discriminated; sources restored", flush=True)
