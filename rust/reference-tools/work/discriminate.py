#!/usr/bin/env python3
"""Prove new work policies and transaction tests reject actual broken implementations."""
from pathlib import Path
import os
import subprocess
import time

root = Path(__file__).resolve().parents[2]
scratch = root.parent / ".scratch"
logs = scratch / "logs"
logs.mkdir(parents=True, exist_ok=True)
env = os.environ.copy()
env.update(CARGO_BUILD_JOBS="2", CARGO_TARGET_DIR=str(scratch / "target"),
           TMPDIR=str(scratch), CI="1", CABLE_TEST_PORT_RANGE="53420-53449",
           MAIL_TEST_PORT_RANGE="53400-53419", GITHUB_TEST_PORT_RANGE="53450-53499",
           LD_LIBRARY_PATH=str(scratch / "rails-media/native-libs"))
env["PATH"] = str(scratch / "rails-media/usr/bin") + ":" + env["PATH"]
mutations = [
    ("human-membership", "crates/campfire/src/controllers/work_threads.rs",
     "if !thread.work_viewable_by(conn, &viewer)? {",
     "if false && !thread.work_viewable_by(conn, &viewer)? {", "campfire",
     "human_handoff_authorization_precedes_receiver_validation_and_creates_nothing"),
    ("human-manager", "crates/campfire/src/controllers/work_threads.rs",
     "if manager && !manageable {", "if false && manager && !manageable {", "campfire",
     "human_handoff_authorization_precedes_receiver_validation_and_creates_nothing"),
    ("link-room", "crates/db/src/models/work_thread_link.rs",
     "&& thread.room_id != event", "&& false && thread.room_id != event", "campfire_db",
     "work_link_models_match_rails_validations_and_persistence"),
    ("work-active-human", "crates/db/src/models/channel_thread/work_listing.rs",
     "if !viewer.is_active() || viewer.is_bot() {",
     "if false && (!viewer.is_active() || viewer.is_bot()) {", "campfire_db",
     "workspace_work_relation_rechecks_membership_is_unbounded_and_rejects_inactive_or_bot_viewers"),
]
for name, relative, original, broken, package, test in mutations:
    path = root / relative
    source = path.read_text()
    assert source.count(original) == 1, name
    changed = source.replace(original, broken, 1)
    try:
        path.write_text(changed)
        command = ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-p", package, test, "--", "--test-threads=4"]
        result = subprocess.run(command, cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        if result.returncode != 0 and "SIGKILL" in result.stdout and "test result:" not in result.stdout:
            (logs / f"human-work-mutant-{name}-compiler-killed.log").write_text(result.stdout)
            print(f"{name}: compiler SIGKILL; retrying without changing the throttle", flush=True)
            time.sleep(20)
            result = subprocess.run(command, cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        (logs / f"human-work-mutant-{name}.log").write_text(result.stdout)
        assert result.returncode != 0 and "test result: FAILED." in result.stdout and "panicked at" in result.stdout and "assertion" in result.stdout, result.stdout[-4000:]
        assert "could not compile" not in result.stdout, result.stdout[-4000:]
        print(f"{name}: actual assertion rejected broken implementation", flush=True)
        print(next(line for line in result.stdout.splitlines() if line.startswith("test result: FAILED.")), flush=True)
    finally:
        path.write_text(source)
print("WS12 human work discriminators: 4 broken implementations rejected at actual assertions; 0 compile/setup failures; sources restored")
