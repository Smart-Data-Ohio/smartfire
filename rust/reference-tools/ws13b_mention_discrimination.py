#!/usr/bin/env python3
"""The real message hook must discriminate preferences and recorder guards."""
import os
from pathlib import Path
import re
import subprocess
ROOT = Path(__file__).resolve().parents[2]
SCRATCH = ROOT / ".scratch/ws13b-mention-discrimination"
SCRATCH.mkdir(parents=True, exist_ok=True)
mutations = [
    ("huddle-preference-leaks", "rust/crates/db/src/models/activity_item/message_recorder.rs", "    for id in ids {", "    for id in ids {\n        if !super::super::huddle_notices::invitations_enabled(conn,id)? { continue; }"),
    ("message-hook-removed", "rust/crates/db/src/models/message.rs", "if !message.system_note { crate::ActivityItem::record_message(tx, &message)?; }", "if false { crate::ActivityItem::record_message(tx, &message)?; }"),
    ("existing-item-refreshed", "rust/crates/db/src/models/activity_item/message_recorder.rs", "ON CONFLICT(user_id,source_type,source_id) DO NOTHING", "ON CONFLICT(user_id,source_type,source_id) DO UPDATE SET read_at=NULL,handled_at=NULL"),
]
environment = dict(os.environ, CARGO_BUILD_JOBS="2", TMPDIR=str(ROOT / ".scratch"), CARGO_TARGET_DIR=str(ROOT / "rust/target"), CI="1", CABLE_TEST_PORT_RANGE="53000-53049", MAIL_TEST_PORT_RANGE="53050-53099")
for name, relative, before, after in mutations:
    path = ROOT / relative
    original = path.read_text()
    try:
        assert original.count(before) == 1, (name, original.count(before))
        path.write_text(original.replace(before, after, 1))
        result = subprocess.run(["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "--manifest-path", str(ROOT / "rust/Cargo.toml"), "-p", "campfire", "huddle_neighbor_mention_test", "--", "--test-threads=8"], cwd=ROOT, env=environment, capture_output=True, text=True)
        output = result.stdout + result.stderr
        (SCRATCH / f"{name}.log").write_text(output)
        summary = next((s for s in re.findall(r"^test result: FAILED\..*$", output, re.M) if "1 failed;" in s), None)
        assert result.returncode != 0 and summary and "could not compile" not in output and "panicked at" in output, output[-5000:]
        print(f"{name}: {summary}", flush=True)
    finally:
        path.write_text(original)
print(f"WS13b mention discrimination: {len(mutations)} compiled regressions detected; sources restored", flush=True)
