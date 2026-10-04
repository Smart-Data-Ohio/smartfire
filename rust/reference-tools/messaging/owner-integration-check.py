#!/usr/bin/env python3
"""Check published owner integration without merging main or changing the worker branch.

The small owned schedule patch is intended for the lead after the owner merge. The
normal branch deliberately does not reference an absent M2 module. This check uses
the real provider and fails every byte difference; it is not full live-page signoff.
"""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[3]
OWNER = "27990da2851f4c056db71c6b430c894307bc6bfe"
parser = argparse.ArgumentParser()
parser.add_argument("--target-dir", type=Path, help="optional reusable build output; not fresh-build evidence")
args = parser.parse_args()
scratch = ROOT / ".scratch"
scratch.mkdir(exist_ok=True)
clone = Path(tempfile.mkdtemp(prefix="ws8bm-owner-", dir=scratch))
subprocess.run(["git", "clone", "--quiet", "--no-local", "--single-branch", "--branch",
                "rust/ws8bm-messages-http", str(ROOT), str(clone)], check=True)
worker = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=clone, text=True).strip()
subprocess.run(["git", "fetch", "--quiet", str(ROOT), OWNER], cwd=clone, check=True)
merge = subprocess.run(["git", "merge", "--no-commit", "--no-ff", "FETCH_HEAD"], cwd=clone,
                       text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
if merge.returncode:
    # Exactly the known module-list and attachment/agent-policy overlap. Preserve both
    # owners. Refuse unfamiliar overlaps instead of selecting an entire side.
    conflicts = subprocess.check_output(["git", "diff", "--name-only", "--diff-filter=U"], cwd=clone, text=True).splitlines()
    assert conflicts == ["rust/crates/campfire/src/controllers/messages.rs"], merge.stdout
    source = clone / conflicts[0]
    text = source.read_text()
    while "<<<<<<< HEAD\n" in text:
        start = text.index("<<<<<<< HEAD\n")
        middle = text.index("=======\n", start)
        end = text.index(">>>>>>>", middle)
        end_line = text.index("\n", end) + 1
        ours = text[start + len("<<<<<<< HEAD\n"):middle]
        theirs = text[middle + len("=======\n"):end]
        if theirs == "pub mod pins;\n":
            assert "mod boosts_tests;" in ours and "mod upload_tests;" in ours
            resolved = ours + theirs
        else:
            old = "            let blob = attachment.map(|staged| save_staged(tx, staged)).transpose()?;\n"
            assert "if agent_policy {" in theirs and old in theirs
            assert ours == "            let blob = attributes.existing_attachment.or(attachment.map(|staged| save_staged(tx, staged)).transpose()?);\n"
            resolved = theirs.replace(old, ours)
        text = text[:start] + resolved + text[end_line:]
    source.write_text(text)
    subprocess.run(["git", "add", conflicts[0]], cwd=clone, check=True)
subprocess.run(["git", "apply", str(ROOT / "rust/reference-tools/messaging/owner-schedule-integration.patch")], cwd=clone, check=True)
(clone / ".scratch").mkdir()
env = dict(os.environ, CI="1", TMPDIR=str(clone / ".scratch"),
           CARGO_TARGET_DIR=str(args.target_dir.resolve() if args.target_dir else clone / "rust/target"),
           CABLE_TEST_PORT_RANGE="52000-52049", MAIL_TEST_PORT_RANGE="52000-52049",
           CAMPFIRE_REFERENCE=str(clone), PARITY_NAMESPACE="ws8bm-owner", PARITY_OWNER="ws8bm",
           PARITY_IMAGE=os.environ.get("PARITY_IMAGE", "campfire-reference"))
env.pop("RUST_TEST_THREADS", None)
print(f"WS8bm owner integration: worker {worker}; shell {OWNER}; isolated merge {clone}; main unmerged", flush=True)
commands = [
    ("metadata", ["mise", "exec", "rust@1.98.1", "--", "cargo", "metadata", "--locked", "--manifest-path", "rust/Cargo.toml", "--format-version", "1"]),
    ("seeds", ["bash", "rust/parity/bin/seed", "build", "default", "first_run"]),
]
for name, command in commands:
    with (clone / ".scratch" / f"{name}.log").open("w") as log:
        subprocess.run(command, cwd=clone, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    if name == "metadata":
        import tomllib
        manifests = [clone / "rust/Cargo.toml", *sorted((clone / "rust/crates").glob("*/Cargo.toml"))]
        for manifest in manifests:
            tomllib.loads(manifest.read_text())
        print(f"WS8bm owner manifests: {len(manifests)} parsed; zero duplicate workspace dependency keys", flush=True)
failures = []
for name, selector in [("native", "controllers::rooms::native_integration_tests"),
                       ("pane", "controllers::channel_threads::content_tests"),
                       ("show", "controllers::channel_threads::page_tests")]:
    command = ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j4", "--manifest-path", "rust/Cargo.toml",
               "-p", "campfire", "--bin", "campfire", selector, "--", "--nocapture"]
    path = clone / ".scratch" / f"{name}.log"
    with path.open("w") as log:
        result = subprocess.run(command, cwd=clone, env=env, stdout=log, stderr=subprocess.STDOUT)
    for line in path.read_text().splitlines():
        if line.startswith(("test result:", "    Finished")):
            print(f"{name}: {line}", flush=True)
    if result.returncode:
        failures.append(name)
        print(f"{name}: exit {result.returncode}; {path}", flush=True)
    elif name == "native":
        result = subprocess.run(["python3", "rust/reference-tools/rooms/native_components_check.py", "--capture-log", str(path)], cwd=clone, env=env)
        if result.returncode:
            failures.append("native component bytes")
assert not failures, f"Owner integration failures: {failures}; all available checks attempted"
print("WS8bm owner integration: live root/around/unread/token checks, 9 exact room components and complete real-child panes passed; full live chrome/system acceptance remains deferred", flush=True)
