#!/usr/bin/env python3
"""Check the former JPEG gap against the committed pinned-Rails regression."""
import os
from pathlib import Path
import subprocess
ROOT = Path(__file__).resolve().parents[3]
env = dict(os.environ, CI="1", CARGO_BUILD_JOBS="2", RUST_TEST_THREADS="8",
           CABLE_TEST_PORT_RANGE="52000-52049", MAIL_TEST_PORT_RANGE="52000-52049")
subprocess.run(["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j2",
                "--manifest-path", "rust/Cargo.toml", "-p", "campfire", "--bin", "campfire",
                "initial_jpeg_after_commit_matches_rails", "--", "--nocapture"], cwd=ROOT, env=env, check=True)
print("WS8bm JPEG: pinned Rails 500 and committed rows match; normal suite regression", flush=True)
