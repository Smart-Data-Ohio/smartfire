#!/usr/bin/env python3
"""Run the committed security regressions against 008fe489, restoring fixed source."""
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / ".scratch/auth/profile-security-proof"
BASE = "008fe4892feb742e32c76e23169ce0ecdcd8dddf"
FILES = [
    "rust/crates/db/src/slash_commands.rs",
    "rust/crates/db/src/models/user.rs",
    "rust/crates/campfire/src/authentication.rs",
    "rust/crates/campfire/src/controllers/users/profiles.rs",
    "rust/crates/campfire/src/controllers/sessions.rs",
    "rust/crates/campfire/src/controllers/sessions/transfers.rs",
    "rust/crates/campfire/src/controllers/accounts/users.rs",
    "rust/crates/campfire/src/controllers/users/bans.rs",
    "rust/crates/views/src/users.rs",
    "rust/crates/views/templates/users/profiles/show.html",
]
OUT.mkdir(parents=True, exist_ok=True)
(ROOT / ".scratch/tmp").mkdir(parents=True, exist_ok=True)
fixture = OUT / "profile.sqlite3"
for suffix in ("", "-wal", "-shm"):
    Path(str(fixture) + suffix).unlink(missing_ok=True)
fixed = {name: (ROOT / name).read_bytes() for name in FILES}
env = os.environ.copy()
env.update(CI="1", CABLE_TEST_PORT_RANGE="51600-51699", TMPDIR=str(ROOT / ".scratch/tmp"), WS9_PROFILE_ROLLBACK_DB=str(fixture))
try:
    for name in FILES:
        (ROOT / name).write_bytes(subprocess.check_output(["git", "show", f"{BASE}:{name}"], cwd=ROOT))
    result = subprocess.run([
        "cargo", "test", "--manifest-path", "rust/Cargo.toml", "--locked", "-j", "4", "-p", "campfire",
        "app::profile_security_tests", "--", "--test-threads=4",
    ], cwd=ROOT, env=env, capture_output=True, text=True)
    output = result.stdout + result.stderr
    (OUT / "native-red.log").write_text(output)
    summary = re.search(r"test result: FAILED\. 0 passed; 8 failed; 0 ignored;[^\n]*", output)
    assert result.returncode != 0 and summary, f"Expected eight executed security failures, not a compile error: {output[-4000:]}"
    print(f"Against {BASE}: {summary.group()}")
    result = subprocess.run([
        "docker", "run", "--rm", "--name", "ws9-profile-security-proof", "--network", "none", "--entrypoint", "",
        "--user", f"{os.getuid()}:{os.getgid()}", "--env-file", str(ROOT / "rust/parity/.env.reference"),
        "-e", "RAILS_LOG_LEVEL=fatal", "-e", "DATABASE_URL=sqlite3:/work/review/profile.sqlite3",
        "-v", f"{OUT}:/work/review", "-v", f"{ROOT}/rust/reference-tools/auth/profile_rollback.rb:/work/profile_rollback.rb:ro",
        env.get("WS9_REFERENCE_IMAGE", "ws9-reference:d7c7de92"), "bin/rails", "runner", "/work/profile_rollback.rb",
    ], cwd=ROOT, capture_output=True, text=True)
    output = result.stdout + result.stderr
    (OUT / "rails-red.log").write_text(output)
    expected = "ROLLBACK FAILURE: Rails auto-linked the new Google subject to Rust user 127326141; marker=nil"
    assert result.returncode != 0 and expected in output, output[-4000:]
    print(expected)
finally:
    for name, content in fixed.items():
        (ROOT / name).write_bytes(content)
    print(f"Restored all {len(FILES)} fixed production files")
