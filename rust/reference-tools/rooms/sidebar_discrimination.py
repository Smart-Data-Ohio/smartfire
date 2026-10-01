#!/usr/bin/env python3
"""Compile wrong cache/nonce implementations, assert regression failures, restore source."""
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[2]
source = root / "crates/views/src/users/sidebar.rs"
scratch = root.parent / ".scratch"
original = source.read_text()
env = dict(os.environ, CI="1", TMPDIR=str(scratch), CARGO_TARGET_DIR=str(root / "target"))
base = ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j", "4",
        "--manifest-path", str(root / "Cargo.toml"), "-p", "campfire_views", "--test", "sidebar"]

def reject(name, changed, test):
    assert changed != original, "mutation did not apply"
    source.write_text(changed)
    result = subprocess.run(base + [test, "--", "--nocapture"], cwd=root.parent, env=env,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    (scratch / f"sidebar-{name}-discrimination.log").write_text(result.stdout)
    summaries = [line for line in result.stdout.splitlines() if line.startswith("test result:")]
    assert result.returncode == 101 and summaries, result.stdout
    assert len(summaries) == 1 and "0 passed; 1 failed;" in summaries[0], result.stdout
    source.write_text(original)
    print(summaries[0])
    print(f"Sidebar discrimination: compiled {name} mutation rejected; source restored", flush=True)

try:
    wrong_key = original.replace("row.participant_ids.as_deref(),\n        row.viewer_administrator,",
                                 "None,\n        false,")
    reject("cache-key", wrong_key, "cached_rows_partition_administrator_and_huddle_participants")
    leak = original.replace('.expect("direct row renders")',
        '.map(|html| format!("{}{}", html, h::request_forgery::csp_nonce().unwrap_or_default()))\n'
        '    .expect("direct row renders")', 1)
    reject("request-nonce", leak, "rows_do_not_capture_request_tokens_nonces_or_actor_permissions")
finally:
    source.write_text(original)
