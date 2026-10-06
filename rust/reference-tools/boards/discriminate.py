#!/usr/bin/env python3
"""Require board policy and pagination assertions to reject broken implementations."""
from pathlib import Path
import os
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root.parent / ".scratch"
env = os.environ.copy()
env.update(CARGO_BUILD_JOBS="2", TMPDIR=str(scratch), CI="1")
env.setdefault("CARGO_TARGET_DIR", str(scratch / "target"))
env.update(CABLE_TEST_PORT_RANGE="53420-53449", MAIL_TEST_PORT_RANGE="53400-53419", GITHUB_TEST_PORT_RANGE="53450-53499")
mutations = [
    ("manager", "crates/campfire/src/controllers/rooms/boards.rs", "    ensure_can_administer(c, &room)?;", "    let _ = ensure_can_administer(c, &room);", "campfire", "boards_rails_cases::only_admins_or_creators_can_update"),
    ("room-scope", "crates/campfire/src/controllers/rooms.rs", "Scope::Boards => room.room_type == RoomType::Board", "Scope::Boards => true", "campfire", "boards_rails_cases::open_and_closed_rooms_cannot_be_converted_to_boards"),
    ("page-probe", "crates/db/src/models/channel_thread/board.rs", "* BOARD_POSTS_PER_PAGE + 1", "* BOARD_POSTS_PER_PAGE", "campfire_db", "board_pages_are_cumulative_clamped_and_ordered_with_one_probe_row"),
]
for name, relative, old, new, package, test in mutations:
    path = root / relative
    source = path.read_text()
    assert source.count(old) == 1, f"mutation anchor moved: {name}"
    try:
        path.write_text(source.replace(old, new))
        result = subprocess.run(["cargo", "test", "--manifest-path", str(root / "Cargo.toml"),
            "--locked", "-p", package, test, "--", "--test-threads=8"], env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (scratch / "logs" / f"boards-mutation-{name}.log").write_text(result.stdout)
        summaries = [line for line in result.stdout.splitlines() if line.startswith("test result:")]
        assert result.returncode and any("1 failed" in line for line in summaries), f"mutation did not reach a failing assertion: {name}\n{result.stdout[-4000:]}"
        print(f"WS12 board mutation {name}: {summaries[-1]}", flush=True)
    finally:
        path.write_text(source)
print("WS12 board discrimination: 3 mutations rejected; sources restored", flush=True)
