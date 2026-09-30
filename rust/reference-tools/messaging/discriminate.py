#!/usr/bin/env python3
"""Compiled HTTP regressions; run sequentially with no other edits/builds. Restore in finally."""
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
SOURCE = ROOT / "rust/crates/campfire/src/controllers/messages.rs"
SCRATCH = ROOT / ".scratch/messaging-discrimination"
SCRATCH.mkdir(parents=True, exist_ok=True)
ENV = dict(os.environ, TMPDIR=str(ROOT / ".scratch"), CARGO_TARGET_DIR=str(ROOT / "rust/target"),
           CARGO_BUILD_JOBS="4", CABLE_TEST_PORT_RANGE="52000-52049", MAIL_TEST_PORT_RANGE="52000-52049")


def check(name, old, new, test):
    original = SOURCE.read_text()
    assert old in original, f"missing mutation anchor: {name}"
    try:
        SOURCE.write_text(original.replace(old, new))
        run = subprocess.run(["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j", "4",
                              "-p", "campfire", "--bin", "campfire", f"controllers::messages::http_tests::{test}",
                              "--", "--exact", "--nocapture"], cwd=ROOT / "rust", env=ENV, capture_output=True, text=True)
        output = run.stdout + run.stderr
        (SCRATCH / f"{name}.log").write_text(output)
        summary = re.findall(r"^test result: FAILED\..*$", output, re.M)
        assert run.returncode != 0 and summary, f"{name}: no compiled test failure"
        assert f"test controllers::messages::http_tests::{test} ... FAILED" in output, f"{name}: wrong failing test"
        print(f"{name}: {summary[0]}", flush=True)
    finally:
        SOURCE.write_text(original)


check("author-edit", "message.system_note || require_current_user(c)?.id != message.creator_id",
      "message.system_note || !require_current_user(c)?.can_administer(Some(message.creator_id), false)",
      "author_only_edits_even_for_an_administrator")
check("immutable-notes", "message.system_note || ", "",
      "system_notes_are_immutable_for_author_and_administrator")
check("member-scope", "let (_, room) = concerns::set_room(c).await?;",
      "let room_id = c.param_str(\"room_id\").and_then(cast_integer).ok_or(Error::NotFound)?;\n"
      "    let room = c.app().db.read(move |conn| Room::find(conn, room_id)).await.map_err(db_error)?;",
      "removed_and_non_members_cannot_read_or_edit_messages")
check("member-delete", "user.id != message.creator_id && !user.is_administrator()", "false",
      "non_author_non_admin_cannot_edit_or_delete")
check("deleted-room", "if room.deleted_at.is_some()", "if false && room.deleted_at.is_some()",
      "deleted_room_is_inaccessible_even_with_a_lingering_membership")
check("root-thread-scope", "Message::find_in(conn, Timeline::Room(room_id), id)", "Message::find_in_room(conn, room_id, id)",
      "root_endpoint_cannot_read_edit_or_delete_thread_messages")
print("WS8bm authorization discrimination: 6 compiled regressions detected; sources restored", flush=True)
