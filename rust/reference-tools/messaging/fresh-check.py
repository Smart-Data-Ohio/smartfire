#!/usr/bin/env python3
"""Verify the committed branch in a fresh clone with generated seeds and a new Cargo target."""
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[3]
scratch = ROOT / ".scratch"
scratch.mkdir(exist_ok=True)
clone = Path(tempfile.mkdtemp(prefix="ws8bm-fresh-", dir=scratch))
subprocess.run(["git", "clone", "--quiet", "--no-local", "--single-branch", "--branch",
                "rust/ws8bm-messages-http", str(ROOT), str(clone)], check=True)
assert not (clone / ".scratch").exists() and not (clone / "rust/target").exists()
(clone / ".scratch").mkdir()
env = dict(os.environ, CI="1", TMPDIR=str(clone / ".scratch"), CARGO_TARGET_DIR=str(clone / "rust/target"),
           CARGO_BUILD_JOBS="4", CABLE_TEST_PORT_RANGE="52050-52099", MAIL_TEST_PORT_RANGE="52050-52099",
           CAMPFIRE_REFERENCE=str(clone), PARITY_NAMESPACE="ws8bm-fresh", PARITY_OWNER="ws8bm",
           PARITY_CPUS="2", PARITY_IMAGE=os.environ.get("PARITY_IMAGE", "triage-reference-d7c7de92"))
env.pop("RUST_TEST_THREADS", None)  # Normal test concurrency; timing failures stay failures.
revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=clone, text=True).strip()
print(f"WS8bm fresh checkout: {revision}; no pre-existing scratch or Cargo target; {clone}", flush=True)
print("WS8bm fresh concurrency: default test threads; four build jobs; no timing threshold changes", flush=True)
commands = [
    ("metadata", ["mise", "exec", "rust@1.98.1", "--", "cargo", "metadata", "--locked", "--manifest-path", "rust/Cargo.toml", "--format-version", "1"]),
    ("seeds", ["bash", "rust/parity/bin/seed", "build", "default", "first_run"]),
    ("app", ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j4", "--manifest-path", "rust/Cargo.toml", "-p", "campfire", "--bin", "campfire"]),
    ("db", ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j4", "--manifest-path", "rust/Cargo.toml", "-p", "campfire_db"]),
    ("views", ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j4", "--manifest-path", "rust/Cargo.toml", "-p", "campfire_views", "--test", "core"]),
    ("clippy", ["mise", "exec", "rust@1.98.1", "--", "cargo", "clippy", "--locked", "-j4", "--manifest-path", "rust/Cargo.toml", "--workspace", "--all-targets", "--", "-D", "warnings"]),
]
failures = []
for name, command in commands:
    with (clone / ".scratch" / f"{name}.log").open("w") as log:
        result = subprocess.run(command, cwd=clone, env=env, stdout=log, stderr=subprocess.STDOUT)
    output = (clone / ".scratch" / f"{name}.log").read_text()
    for line in output.splitlines():
        if line.startswith(("seed:", "test result:", "    Finished")):
            print(line, flush=True)
    if result.returncode:
        failures.append(name)
        print(f"{name}: exit {result.returncode}; inspect {clone / '.scratch' / (name + '.log')}", flush=True)
        for line in output.splitlines():
            if line.startswith("test ") and line.endswith("FAILED"):
                print(line, flush=True)
        if name in ["metadata", "seeds"]:
            break  # Without setup, later checks cannot be valid.
assert not failures, f"WS8bm fresh-check failures: {', '.join(failures)}; all available checks were run"
print("WS8bm fresh-check: committed inputs only; generated default/first_run seeds; app/db/views/clippy passed", flush=True)
