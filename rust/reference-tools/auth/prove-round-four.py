#!/usr/bin/env python3
"""Execute the new regressions with 5e2aee1e production bytes; restore every byte in finally."""
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
BASE = "5e2aee1e1f8601a2442c6126c9f7a666790bc389"
FILES = [
    "rust/crates/db/src/slash_commands/time_parser.rs",
    "rust/crates/db/src/slash_commands/user_settings.rs",
    "rust/crates/db/src/models/user.rs",
    "rust/crates/campfire/src/controllers/accounts.rs",
    "rust/crates/campfire/src/controllers/accounts/join_codes.rs",
    "rust/crates/campfire/src/controllers/accounts/custom_styles.rs",
    "rust/crates/campfire/src/controllers/accounts/logos.rs",
]
OUT = ROOT / ".scratch/auth/round-four-proof"
OUT.mkdir(parents=True, exist_ok=True)
(ROOT / ".scratch/tmp").mkdir(parents=True, exist_ok=True)
fixture = OUT / "zone.sqlite3"
for suffix in ("", "-wal", "-shm"):
    Path(str(fixture) + suffix).unlink(missing_ok=True)
fixed = {name: (ROOT / name).read_bytes() for name in FILES}
env = os.environ.copy()
env.update(CI="1", CABLE_TEST_PORT_RANGE="51600-51699", TMPDIR=str(ROOT / ".scratch/tmp"), WS9_ZONE_ROLLBACK_DB=str(fixture))
try:
    for name in FILES:
        (ROOT / name).write_bytes(subprocess.check_output(["git", "show", f"{BASE}:{name}"], cwd=ROOT))
    for package, filter_name, count in [("campfire", "app::round_four_security_tests", 6), ("campfire_db", "slash_commands::time_zone_writer_tests", 4)]:
        result = subprocess.run(["cargo", "test", "--manifest-path", "rust/Cargo.toml", "--locked", "-j", "4", "-p", package, filter_name, "--", "--test-threads=4"], cwd=ROOT, env=env, capture_output=True, text=True)
        output = result.stdout + result.stderr
        (OUT / f"{package}-red.log").write_text(output)
        summary = re.search(rf"test result: FAILED\. 0 passed; {count} failed; 0 ignored;[^\n]*", output)
        assert result.returncode != 0 and summary, f"Expected {count} executed failures, not a compile error: {output[-4000:]}"
        print(f"Against {BASE} ({package}): {summary.group()}")
    assert fixture.is_file(), "HTTP regression must export the actual Rust-written wrong-case row before asserting"
    result = subprocess.run(["docker", "run", "--rm", "--name", "ws9-round-four-proof", "--network", "none", "--entrypoint", "", "--user", f"{os.getuid()}:{os.getgid()}", "--env-file", str(ROOT / "rust/parity/.env.reference"), "-e", "RAILS_LOG_LEVEL=fatal", "-e", "DATABASE_URL=sqlite3:/work/review/zone.sqlite3", "-v", f"{OUT}:/work/review", "-v", f"{ROOT}/rust/reference-tools/auth/round_four_rollback.rb:/work/round_four_rollback.rb:ro", env.get("WS9_REFERENCE_IMAGE", env.get("PARITY_IMAGE", "campfire-reference")), "bin/rails", "runner", "/work/round_four_rollback.rb"], cwd=ROOT, env=env, capture_output=True, text=True)
    output = result.stdout + result.stderr
    (OUT / "rails-red.log").write_text(output)
    assert result.returncode != 0 and "ROLLBACK FAILURE: Rails rejected Rust's time zone" in output, output
    print(next(line for line in output.splitlines() if line.startswith("WS9 zone readback:")))
    print("Against baseline Rust data: Rails readback failed as expected (invalid time_zone)")
finally:
    for name, content in fixed.items():
        (ROOT / name).write_bytes(content)
    print(f"Restored all {len(FILES)} fixed production files")
