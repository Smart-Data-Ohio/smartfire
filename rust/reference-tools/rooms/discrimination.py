#!/usr/bin/env python3
"""Re-run the new HTTP tests against the original compiled controllers; always restore source."""
import os
from pathlib import Path
import re
import subprocess

repo = Path(__file__).resolve().parents[3]
rust = repo / "rust"
scratch = repo / ".scratch"
scratch.mkdir(exist_ok=True)
paths = [
    "crates/campfire/src/concerns.rs",
    "crates/campfire/src/controllers.rs",
    "crates/campfire/src/controllers/rooms.rs",
    "crates/campfire/src/controllers/rooms/directs.rs",
    "crates/campfire/src/controllers/rooms/involvements.rs",
]
saved = {path: (rust / path).read_bytes() for path in paths}
env = dict(os.environ, TMPDIR=str(scratch), CARGO_TARGET_DIR=str(rust / "target"),
           CABLE_TEST_PORT_RANGE="52100-52149", MAIL_TEST_PORT_RANGE="52100-52149")
command = ["cargo", "test", "--locked", "-j", "4",
           "-p", "campfire", "--bin", "campfire", "rooms::parity_tests", "--", "--nocapture"]
try:
    for path in paths:
        source = subprocess.check_output(["git", "show", f"bb6c5d78:rust/{path}"], cwd=repo)
        if path.endswith("controllers/rooms.rs"):
            # Compile the old controllers against the new row API; this only supplies the
            # render context, leaving all old authorization and mutation logic unchanged.
            source = source.replace(b'&base_url, |_| {', b'&base_url, |ctx| {')
            source = source.replace(b'SidebarSharedPartial { room: sidebar_room }', b'SidebarSharedPartial { ctx, room: sidebar_room }')
            source = source.replace(b'Ok(campfire_views::rooms::ShowView {', b'Ok(campfire_views::rooms::ShowView { shell: Default::default(), scroll_to_unread_divider: None, jump_to_unread_url: None, unread_divider_message_id: None, unread_count: 0,')
            source += b"\n#[cfg(test)]\nmod parity_tests;\n"
        (rust / path).write_bytes(source)
    result = subprocess.run(command, cwd=rust, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    output = result.stdout.decode()
    (scratch / "rooms-discrimination.log").write_text(output)
    summary = re.search(r"^test result: .*", output, re.MULTILINE)
    if summary:
        print(summary.group())
    assert result.returncode != 0 and summary, "must fail assertions, not compilation"
    assert "17 failed" in summary.group() and "2 passed" in summary.group(), output[-5000:]
    assert "error: could not compile" not in output, "compilation is not discrimination"
    print("WS8br discrimination: 17 HTTP regressions rejected bb6c5d78; 1 existing guard and 1 independent unread-presenter test passed; source restored")
    for path, content in saved.items():
        (rust / path).write_bytes(content)
    path = "crates/campfire/src/controllers/rooms.rs"
    source = saved[path].decode()
    correct = '    let destroyed = room.clone();\n    c.app().db.write(move |tx| destroyed.begin_destroy(tx)).await.map_err(db_error)?;\n    audit_room(c, &room, "room.destroy", serde_json::json!({"name": room.name})).await?;'
    broken = '    let context = audit_context(c)?;\n    let destroyed = room.clone();\n    c.app().db.write(move |tx| { destroyed.begin_destroy(tx)?; record_room_audit(tx, &destroyed, "room.destroy", serde_json::json!({"name": destroyed.name}), &context) }).await.map_err(db_error)?;'
    assert correct in source, "audit mutation no longer matches source"
    (rust / path).write_text(source.replace(correct, broken, 1))
    probe = command.copy()
    probe[probe.index("rooms::parity_tests")] = "parity_audit_failure"
    result = subprocess.run(probe, cwd=rust, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    output = result.stdout.decode()
    (scratch / "audit-discrimination.log").write_text(output)
    summary = re.search(r"^test result: .*", output, re.MULTILINE)
    assert result.returncode != 0 and summary and "1 failed" in summary.group(), output[-5000:]
    assert "error: could not compile" not in output
    print(summary.group())
    print("WS8br audit discrimination: compiled transactional-audit regression rejected; source restored")
finally:
    for path, content in saved.items():
        (rust / path).write_bytes(content)
