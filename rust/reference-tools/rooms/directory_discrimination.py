#!/usr/bin/env python3
"""Reject permissive room access and actor-specific header rendering with compiled tests."""
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root.parent / ".scratch"
env = dict(os.environ, CI="1", TMPDIR=str(scratch), CARGO_TARGET_DIR=str(root / "target"),
           CABLE_TEST_PORT_RANGE="52100-52149", MAIL_TEST_PORT_RANGE="52100-52149")
base = ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j", "4",
        "--manifest-path", str(root / "Cargo.toml"), "-p", "campfire", "--bin", "campfire"]

def reject(name, path, old, new, test):
    original = path.read_text()
    assert original.count(old) == 1, "mutation must apply once"
    try:
        path.write_text(original.replace(old, new))
        result = subprocess.run(base + [test, "--", "--nocapture"], cwd=root.parent, env=env,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        (scratch / f"directory-{name}-discrimination.log").write_text(result.stdout)
        summaries = [line for line in result.stdout.splitlines() if line.startswith("test result:")]
        assert result.returncode == 101 and len(summaries) == 1, result.stdout
        assert "0 passed; 1 failed;" in summaries[0], result.stdout
        print(summaries[0])
    finally:
        path.write_text(original)
    print(f"Directory discrimination: compiled {name} mutation rejected; source restored", flush=True)

reject("unscoped-access", root / "crates/campfire/src/controllers/rooms.rs",
       "Room::find_for_user(conn, user_id, id)", "Room::find_by_id(conn, id)",
       "directory_http_guards_leave_room_and_recipients_unchanged")
reject("actor-header", root / "crates/campfire/src/channels/rooms_directory.rs",
       "User::find(conn, *for_user_id)?", "User::find(conn, 712064548)?",
       "group_directory_callbacks_match_rails_recipient_frames")
reject("user-join-order", root / "crates/campfire/src/controllers/presenters/accounts.rs",
       ") ORDER BY memberships.id", ") ORDER BY users.id",
       "seeded_group_row_matches_rails_membership_order")
reject("unicode-whitespace", root / "crates/campfire/src/controllers/presenters/accounts.rs",
       "name.split([' ', '\\t', '\\n', '\\r', '\\x0b', '\\x0c'])", "name.split_whitespace()",
       "direct_labels_match_rails_ascii_word_splitting")
