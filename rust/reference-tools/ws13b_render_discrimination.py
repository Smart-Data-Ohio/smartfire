#!/usr/bin/env python3
"""Compiled regressions must fail every outstanding Rails render declaration."""
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
SCRATCH = ROOT / ".scratch/ws13b-render-discrimination"
SCRATCH.mkdir(parents=True, exist_ok=True)
mutations = [
    ("presence-html", "campfire/src/channels/huddle_effects.rs", '&room_dom_id(&room, "header_voice_participants"),', '&room_dom_id(&room, "wrong_header_voice_participants"),'),
    ("missing-configuration-guard", "campfire/src/channels/huddle_effects.rs", "if !app.config.huddle.configured() {", "if false {"),
    ("room-deletion-omitted", "db/src/models/room_delete.rs", '"DELETE FROM rooms WHERE id=?", [room.id]', '"DELETE FROM rooms WHERE id=? AND 0", [room.id]'),
    ("stream-html", "campfire/src/channels/huddle_effects.rs", '&room_dom_id(&room, "stage_live_badge"),', '&room_dom_id(&room, "wrong_stage_live_badge"),'),
    ("stream-end-not-idempotent", "db/src/models/stream.rs", "if !self.live() {", "if false {"),
    ("self-stop-broadcast", "db/src/models/stream.rs", "actor != self.user_id", "actor == self.user_id"),
    ("automatic-stop-broadcast", "db/src/models/stream.rs", "if ended_by.is_some_and(|actor| actor != self.user_id) {", "if ended_by.is_none() || ended_by.is_some_and(|actor| actor != self.user_id) {"),
]
environment = dict(os.environ, TMPDIR=str(ROOT / ".scratch"), CARGO_TARGET_DIR=str(ROOT / "rust/target"), CI="1", CABLE_TEST_PORT_RANGE="53000-53049", MAIL_TEST_PORT_RANGE="53050-53099")
detected = set()
for name, file, before, after in mutations:
    path = ROOT / "rust/crates" / file
    original = path.read_text()
    try:
        assert original.count(before) == 1, (name, original.count(before))
        path.write_text(original.replace(before, after, 1))
        result = subprocess.run(["cargo", "test", "--locked", "-j4", "--manifest-path", str(ROOT / "rust/Cargo.toml"), "-p", "campfire", "huddle_render_tests", "--", "--test-threads=4"], cwd=ROOT, env=environment, capture_output=True, text=True)
        output = result.stdout + result.stderr
        (SCRATCH / f"{name}.log").write_text(output)
        summaries = re.findall(r"^test result: FAILED\..*$", output, re.M)
        assert result.returncode != 0 and summaries and "could not compile" not in output and "panicked at" in output, output[-5000:]
        detected.update(re.findall(r"^test jobs::huddle_render_tests::(\w+) \.\.\. FAILED$", output, re.M))
        print(f"{name}: {summaries[0]}", flush=True)
    finally:
        path.write_text(original)
expected = set("mark_out issue_voice revoke_voice first_voice issue_open issue_closed issue_direct revoke_channel revoke_direct first_channel unconfigured destroyed_room issue_stage revoke_stage stream_start stream_end stream_twice host_stop self_stop automatic_stop".split())
assert detected == expected, (expected - detected, detected - expected)
print(f"WS13b render discrimination: {len(mutations)} compiled regressions detected across {len(detected)} declarations; sources restored", flush=True)
