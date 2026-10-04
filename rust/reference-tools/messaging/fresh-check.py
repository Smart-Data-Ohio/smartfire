#!/usr/bin/env python3
"""Verify the committed branch in a fresh clone with generated seeds and a new Cargo target."""
import os
from pathlib import Path
import subprocess
import tempfile
import shutil
import argparse

ROOT = Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--native", action="store_true", help="host media versions must match the pinned vectors")
parser.add_argument("--toolchain-image", default=os.environ.get("RUST_CI_IMAGE", "campfire-toolchain"))
parser.add_argument("--behavior-files", nargs="*", default=[],
                    help="also run these paired browser groups and their served mutants from the clone")
options = parser.parse_args()
scratch = ROOT / ".scratch"
scratch.mkdir(exist_ok=True)
clone = Path(tempfile.mkdtemp(prefix="ws8bm-fresh-", dir=scratch))
subprocess.run(["git", "clone", "--quiet", "--no-local", "--single-branch", "--branch",
                "rust/ws8bm-messages-http", str(ROOT), str(clone)], check=True)
assert not (clone / ".scratch").exists() and not (clone / "rust/target").exists()
(clone / ".scratch").mkdir()
env = dict(os.environ, CI="1", TMPDIR=str(clone / ".scratch"), CARGO_TARGET_DIR=str(clone / "rust/target"),
           CARGO_BUILD_JOBS="2", RUST_TEST_THREADS="8", CABLE_TEST_PORT_RANGE="52050-52099", MAIL_TEST_PORT_RANGE="52050-52099",
           CAMPFIRE_REFERENCE=str(clone), PARITY_NAMESPACE="ws8bm-fresh", PARITY_OWNER="ws8bm",
           PARITY_CPUS="2", PARITY_IMAGE=os.environ.get("PARITY_IMAGE", "campfire-reference"))

revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=clone, text=True).strip()
print(f"WS8bm fresh checkout: {revision}; no pre-existing scratch or Cargo target; {clone}", flush=True)
print("WS8bm fresh concurrency: eight test threads; two build jobs; native timing unchanged; audited browser deadlines match Rails", flush=True)
if not options.native:
    # Docker's ancestry does not include Codex. Always claim the same host flock slots
    # instead of relying on the host wrapper's ancestry detection inside the container.
    wrapper = clone / ".scratch/rustc-throttle.sh"
    wrapper.write_text("""#!/bin/bash
set -euo pipefail
compiler="$1"; shift
case " $* " in *" --crate-name "*) ;; *) exec "$compiler" "$@" ;; esac
slots=$(cat /machine-rustc-slots)
while :; do
  for ((i = 0; i < slots; i++)); do
    exec {fd}>>"/tmp/rust-port-rustc-slots/$i"
    if flock -n "$fd"; then exec "$compiler" "$@"; fi
    exec {fd}>&-
  done
  sleep 0.3
done
""")
    wrapper.chmod(0o755)
    (clone / ".scratch/cargo-home").mkdir()
    Path("/tmp/rust-port-rustc-slots").mkdir(exist_ok=True)
    print(f"WS8bm pinned processing: {options.toolchain_image}; shared machine rustc flock slots", flush=True)

def pinned_command(command, name):
    cargo = command[command.index("cargo"):]
    result = ["docker", "run", "--rm", "--name", f"{clone.name}-{name}", "--cpus", "2", "--user", f"{os.getuid()}:{os.getgid()}",
              "--workdir", "/src", "-v", f"{clone}:/src",
              "-v", f"{Path.home() / '.cargo/registry'}:/src/.scratch/cargo-home/registry",
              "-v", "/tmp/rust-port-rustc-slots:/tmp/rust-port-rustc-slots",
              "-v", f"{Path.home() / '.cache/rust-port/rustc-slots'}:/machine-rustc-slots:ro"]
    processing_env = dict(CI="1", TMPDIR="/src/.scratch", CARGO_TARGET_DIR="/src/rust/target",
                          CARGO_HOME="/src/.scratch/cargo-home", CARGO_BUILD_JOBS="2", RUST_TEST_THREADS="8",
                          RUSTC_WRAPPER="/src/.scratch/rustc-throttle.sh", CAMPFIRE_REFERENCE="/src",
                          CABLE_TEST_PORT_RANGE="52050-52099", MAIL_TEST_PORT_RANGE="52050-52099")
    for key, value in processing_env.items():
        result.extend(["-e", f"{key}={value}"])
    return [*result, options.toolchain_image, *cargo]
commands = [
    ("metadata", ["mise", "exec", "rust@1.98.1", "--", "cargo", "metadata", "--locked", "--manifest-path", "rust/Cargo.toml", "--format-version", "1"]),
    ("seeds", ["bash", "rust/parity/bin/seed", "build", "default", "first_run"]),
    ("workspace", ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j2", "--manifest-path", "rust/Cargo.toml", "--workspace", "--exclude", "html5ever", "--no-fail-fast"]),
    ("clippy", ["mise", "exec", "rust@1.98.1", "--", "cargo", "clippy", "--locked", "-j2", "--manifest-path", "rust/Cargo.toml", "--workspace", "--all-targets", "--", "-D", "warnings"]),
]
if options.behavior_files:
    for name, extra in [("behavior", []), ("discrimination", ["--negative"])]:
        commands.append((name, ["python3", "rust/reference-tools/messaging/behavior-check.py",
                               *options.behavior_files, *extra, "--keep-going"]))
failures = []
for name, command in commands:
    if name in ["workspace", "clippy"] and not options.native:
        command = pinned_command(command, name)
    with (clone / ".scratch" / f"{name}.log").open("w") as log:
        result = subprocess.run(command, cwd=clone, env=env, stdout=log, stderr=subprocess.STDOUT)
    output = (clone / ".scratch" / f"{name}.log").read_text()
    for line in output.splitlines():
        if line.startswith(("seed:", "test result:", "    Finished", "WS8bm behaviour", "WS8bm discrimination")):
            print(line, flush=True)
    if result.returncode:
        failures.append(name)
        print(f"{name}: exit {result.returncode}; inspect {clone / '.scratch' / (name + '.log')}", flush=True)
        for line in output.splitlines():
            if line.startswith("test ") and line.endswith("FAILED"):
                print(line, flush=True)
        if name in ["metadata", "seeds"]:
            break  # Without setup, later checks cannot be valid.
shutil.rmtree(clone / "rust/target", ignore_errors=True)
print("WS8bm fresh target removed", flush=True)
assert not failures, f"WS8bm fresh-check failures: {', '.join(failures)}; all available checks were run"
print("WS8bm fresh-check: committed inputs only; generated default/first_run seeds; workspace tests/doctests/clippy passed"
      + ("; requested browser groups and their mutants passed" if options.behavior_files else ""), flush=True)
