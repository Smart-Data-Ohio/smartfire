#!/usr/bin/env python3
"""Seeded Rails/Rust behaviour checks. Builds its own seeds, binary and browser inputs.

Each named case gets independent copies of the seed, two real viewer sessions and
real HTTP/Action Cable. No pre-existing target/scratch, screenshot or response mask.
"""
import argparse
import hashlib
import os
from pathlib import Path
import shutil
import socket
import sqlite3
import subprocess
import tempfile
import time
import textwrap
import urllib.request

ROOT = Path(__file__).resolve().parents[3]
RUST = ROOT / "rust"
SCRATCH = ROOT / ".scratch"
PIN = "d7c7de9264c63015be398001d7a1094e7695a6db"
CASES = {
    "sending_messages": ["sending messages between two users", "editing messages", "deleting messages"],
    "workspace_markdown": [
        "Markdown messages reach other users and editing preserves the original source",
        "desktop keyboard composition keeps line breaks and sends once after composition ends",
        "untrusted markup stays inert in the delivered message",
    ],
}
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("files", nargs="*", choices=CASES)
args = parser.parse_args()
files = args.files or list(CASES)
SCRATCH.mkdir(exist_ok=True)
env = dict(os.environ, CARGO_BUILD_JOBS="2", RUST_TEST_THREADS="8", PARITY_CPUS="2",
           PARITY_NAMESPACE="ws8bm-behavior", PARITY_OWNER="ws8bm", TMPDIR=str(SCRATCH))
image = os.environ.get("PARITY_IMAGE", "triage-reference-d7c7de92")
revision = subprocess.check_output(["docker", "image", "inspect", "--format", "{{range .Config.Env}}{{println .}}{{end}}", image], text=True)
assert any(f"GIT_REVISION={value}" in revision.splitlines() for value in [PIN, PIN[:8]]), "browser reference must be the pinned Rails image"
env["PARITY_IMAGE"] = image
subprocess.run(["bash", "rust/parity/bin/seed", "build", "default", "first_run"], cwd=ROOT, env=env, check=True)
subprocess.run(["mise", "exec", "rust@1.98.1", "--", "cargo", "build", "--locked", "-j2", "--manifest-path", "rust/Cargo.toml", "-p", "campfire", "--bin", "campfire"], cwd=ROOT, env=env, check=True)
subprocess.run(["npm", "ci", "--prefix", "rust/parity"], cwd=ROOT, check=True)
subprocess.run(["npm", "exec", "--prefix", "rust/parity", "--", "playwright", "install", "chromium"], cwd=ROOT, check=True)
browser_image = "ws8bm-browser-reference-d7c7de92"
subprocess.run(["docker", "build", "--build-arg", f"BASE_IMAGE={image}", "-f", str(RUST / "reference-tools/rooms/browser.Dockerfile"), "-t", browser_image, str(RUST / "parity/docker")], cwd=ROOT, check=True)
env["PARITY_IMAGE"] = browser_image
for line in (RUST / "parity/.env.reference").read_text().splitlines():
    if line and not line.startswith("#"):
        key, value = line.split("=", 1)
        env[key] = value
env.update(CAMPFIRE_FROZEN_TIME="2026-03-02T16:00:00Z", CAMPFIRE_LOG="error", TARGET_BIND="127.0.0.1")
target = Path(env.get("CARGO_TARGET_DIR", RUST / "target"))
reference = str(RUST / "parity/bin/reference")
ports = [52120, 52121, 52122]
# Refuse occupied ports; never stop another worker's listener.
reservations = []
try:
    for port in ports:
        reservation = socket.socket()
        reservation.bind(("127.0.0.1", port))
        reservations.append(reservation)
finally:
    for reservation in reservations:
        reservation.close()

passed = 0
for file in files:
    source = subprocess.check_output(["git", "show", f"{PIN}:test/system/{file}_test.rb"], cwd=ROOT)
    for case in CASES[file]:
        assert f'test "{case}"'.encode() in source, "case must be named in the pin"
        with tempfile.TemporaryDirectory(prefix="ws8bm-behavior-", dir=SCRATCH) as directory:
            work = Path(directory)
            shutil.copytree(RUST / "parity/.seed/default/db", work / "db")
            shutil.copytree(RUST / "parity/.seed/default/storage", work / "files")
            run_env = dict(env, CAMPFIRE_STORAGE_PATH=str(work), HTTP_PORT=str(ports[1]), TARGET_PORT=str(ports[2]))
            process = None
            with (SCRATCH / "ws8bm-behavior-servers.log").open("a") as log:
                try:
                    subprocess.run([reference, "up", "--seed", "default", "--port", str(ports[0]), "--time", "2026-03-02T16:00:00Z", "--freeze"], cwd=ROOT, env=run_env, stdout=log, stderr=log, check=True)
                    process = subprocess.Popen([str(target / "debug/campfire"), "server"], cwd=ROOT, env=run_env, stdout=log, stderr=log)
                    deadline = time.monotonic() + 120
                    while True:
                        if process.poll() is not None:
                            raise RuntimeError("candidate stopped; see .scratch/ws8bm-behavior-servers.log")
                        try:
                            with urllib.request.urlopen(f"http://127.0.0.1:{ports[1]}/up", timeout=2) as response:
                                if response.status == 200:
                                    break
                        except OSError:
                            pass
                        if time.monotonic() > deadline:
                            raise TimeoutError("candidate not ready")
                        time.sleep(.2)
                    subprocess.run(["node", str(RUST / "reference-tools/messaging/behavior.mjs"), f"http://127.0.0.1:{ports[0]}", f"http://127.0.0.1:{ports[1]}", file, case], cwd=ROOT, env=run_env, check=True)
                    databases = [RUST / f"parity/.seed/.instances/{ports[0]}/db/production.sqlite3", work / "db/production.sqlite3"]
                    for database in databases:
                        with sqlite3.connect(database) as conn:
                            if case == "sending messages between two users":
                                for body in ["Is this thing on?", "👍👍"]:
                                    assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source=?", (body,)).fetchone()[0] == 1
                            elif case == "editing messages":
                                assert conn.execute("SELECT markdown_source FROM messages WHERE id=607264868").fetchone() == ("Redacted!",)
                            elif case == "deleting messages":
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE id=607264868").fetchone()[0] == 0
                            elif case == CASES["workspace_markdown"][0]:
                                markdown = textwrap.dedent(source.decode().split("MARKDOWN = <<~'MARKDOWN'.freeze\n")[1].split("  MARKDOWN")[0])
                                # The actual browser edit uses multipart FormData, whose
                                # wire serialization preserves CRLF in the saved string.
                                # This is observed in pinned Rails; compare raw saved bytes
                                # on Rust as well, without normalizing either database.
                                edited = markdown.replace("Design review", "Review complete").replace("\n", "\r\n")
                                actual = conn.execute("SELECT markdown_source FROM messages WHERE markdown_source LIKE '## Review complete%'").fetchall()
                                assert actual == [(edited,)], f"{database}: saved source {actual!r}; expected {edited!r}"
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source=?", (markdown,)).fetchone()[0] == 0
                            elif case == CASES["workspace_markdown"][1]:
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source=?", ("First line\nSecond line",)).fetchone()[0] == 1
                            elif case == CASES["workspace_markdown"][2]:
                                payload = textwrap.dedent(source.decode().split("payload = <<~'MARKDOWN'\n")[1].split("    MARKDOWN")[0])
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source=?", (payload,)).fetchone()[0] == 1
                    passed += 1
                    print(f"WS8bm behaviour: {file}: {case}: Rails PASS; Rust PASS; persisted rows PASS", flush=True)
                finally:
                    if process is not None:
                        process.terminate()
                        try:
                            process.wait(timeout=15)
                        except subprocess.TimeoutExpired:
                            process.kill()
                            process.wait()
                    subprocess.run([reference, "down", "--port", str(ports[0])], cwd=ROOT, env=run_env, stdout=log, stderr=log, check=True)
    print(f"WS8bm behaviour source: test/system/{file}_test.rb SHA256 {hashlib.sha256(source).hexdigest()}", flush=True)
print(f"WS8bm behaviour check: {passed} named cases passed on Rails and Rust; 0 failed; no pixel checks", flush=True)
