#!/usr/bin/env python3
"""The real message hook must discriminate preferences and recorder guards."""
import os
from pathlib import Path
import re
import subprocess
ROOT = Path(__file__).resolve().parents[2]
SCRATCH = ROOT / ".scratch/ws13b-mention-discrimination"
SCRATCH.mkdir(parents=True, exist_ok=True)
path = ROOT / "rust/crates/db/src/models/activity_mentions.rs"
mutations = [
    ("huddle-preference-leaks", "    for recipient in recipients {", "    for recipient in recipients {\n        if !super::huddle_notices::invitations_enabled(tx.conn(),recipient.id)? { continue; }"),
    ("system-note-records", "if message.system_note || message.streaming {", "if message.streaming {"),
    ("bot-and-inactive-record", "|| !recipient.is_active() || recipient.is_bot()", "|| false"),
    ("existing-item-refreshed", 'if ActivityItem::find_by_user_and_source(tx.conn(), recipient.id, "Message", message.id)?\n            .is_none()', "if true"),
]
environment = dict(os.environ, TMPDIR=str(ROOT / ".scratch"), CARGO_TARGET_DIR=str(ROOT / "rust/target"), CI="1", CABLE_TEST_PORT_RANGE="53000-53049", MAIL_TEST_PORT_RANGE="53050-53099")
for name, before, after in mutations:
    original = path.read_text()
    try:
        assert original.count(before) == 1, (name, original.count(before))
        path.write_text(original.replace(before, after, 1))
        result = subprocess.run(["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j4", "--manifest-path", str(ROOT / "rust/Cargo.toml"), "-p", "campfire", "huddle_neighbor_mention_test", "--", "--test-threads=4"], cwd=ROOT, env=environment, capture_output=True, text=True)
        output = result.stdout + result.stderr
        (SCRATCH / f"{name}.log").write_text(output)
        summary = next((s for s in re.findall(r"^test result: FAILED\..*$", output, re.M) if "1 failed;" in s), None)
        assert result.returncode != 0 and summary and "could not compile" not in output and "panicked at" in output, output[-5000:]
        print(f"{name}: {summary}", flush=True)
    finally:
        path.write_text(original)
print(f"WS13b mention discrimination: {len(mutations)} compiled regressions detected; sources restored", flush=True)
