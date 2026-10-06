#!/usr/bin/env python3
"""Prove the final switcher/auth tests reject the compiled inherited unmapped action."""
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root.parent / ".scratch"
path = root / "crates/campfire/src/controllers.rs"
original = path.read_text()
old = '"switchers#show" => arc(switchers::show),'
assert original.count(old) == 1
env = dict(os.environ, CI="1", TMPDIR=str(scratch), CARGO_TARGET_DIR=str(root / "target"),
           CABLE_TEST_PORT_RANGE="52100-52149", MAIL_TEST_PORT_RANGE="52100-52149")
try:
    path.write_text(original.replace(old, '"switchers#show" => arc(not_yet_ported),'))
    result = subprocess.run(["cargo", "test", "--locked", "-j", "4",
        "--manifest-path", str(root / "Cargo.toml"), "-p", "campfire", "--bin", "campfire", "controllers::switchers::tests",
        "--", "--nocapture"], cwd=root.parent, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    (scratch / "switcher-discrimination.log").write_text(result.stdout)
    summaries = [line for line in result.stdout.splitlines() if line.startswith("test result:")]
    assert result.returncode == 101 and len(summaries) == 1, result.stdout
    assert "0 passed; 4 failed;" in summaries[0], result.stdout
    print(summaries[0])
finally:
    path.write_text(original)
print("Switcher discrimination: compiled unmapped action rejected by all four regressions; source restored")
