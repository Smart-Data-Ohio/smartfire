#!/usr/bin/env python3
"""Run the same browser scenarios against an isolated Rails seed and the debug Rust binary."""
from pathlib import Path
import hashlib
import json
import os
import shutil
import socket
import sqlite3
import sys
import subprocess
import tempfile
import time
import urllib.request

root = Path(__file__).resolve().parents[2]
scratch = root.parent / ".scratch"
scratch.mkdir(exist_ok=True)
run_dir = Path(tempfile.mkdtemp(prefix="browser-people-", dir=scratch))
seed = root / "parity/.seed/default"
picker_mode = sys.argv[1:] == ["--picker"]
assert not sys.argv[1:] or picker_mode, "expected --picker or no arguments"
browser_seed = None
if picker_mode:
    browser_seed = root / f"parity/.seed/ws8br2-browser-{os.getpid()}"
    shutil.copytree(seed, browser_seed)
    seed = browser_seed
    with sqlite3.connect(seed / "db/production.sqlite3") as db:
        for id, name, email in ((9100000001, "Chad Puterbaugh", "chad@example.test"), (9100000002, "Renée Dupont", "renee@example.test")):
            db.execute("INSERT INTO users(id,name,email_address,created_at,updated_at) VALUES (?,?,?,'2026-03-02 16:00:00','2026-03-02 16:00:00')", (id,name,email))
    input_labels = json.loads((seed / "labels.json").read_text())
    input_labels.update({"users.chad":9100000001,"users.renee":9100000002})
    (seed / "labels.json").write_text(json.dumps(input_labels))
assert (seed / "db/production.sqlite3").is_file(), "build the default seed first"
target = Path(os.environ.get("CARGO_TARGET_DIR", root / "target"))
binary = target / "debug/campfire"
assert binary.is_file(), "build the debug campfire binary first"
for port in (52610, 52611, 52612):
    with socket.socket() as check:
        check.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        check.bind(("127.0.0.1", port))
labels = json.loads((seed / "labels.json").read_text())
env = os.environ.copy()
for line in (root / "parity/.env.reference").read_text().splitlines():
    if line and not line.startswith("#") and "=" in line:
        key, value = line.split("=", 1)
        env[key] = value
env.update({key.removeprefix("reference_env."): str(value) for key, value in labels.items() if key.startswith("reference_env.")})
env.update(CAMPFIRE_STORAGE_PATH=str(run_dir), CAMPFIRE_FROZEN_TIME=labels["clock.now"],
           DISABLE_SSL="1", HTTP_PORT="52611", TARGET_PORT="52612", RAILS_LOG_LEVEL="warn")
env.pop("TLS_DOMAIN", None)
shutil.copytree(seed / "db", run_dir / "db")
shutil.copytree(seed / "storage", run_dir / "files")
oracle_env = os.environ.copy()
# The media library overlay belongs to the Rust process, not host curl/Python forwarding.
oracle_env.pop("LD_LIBRARY_PATH", None)
oracle_env.update(PARITY_NAMESPACE="ws8br2-browser", PARITY_OWNER="ws8br2", PARITY_RUNTIME="docker", PARITY_IMAGE="ws8br2-reference:d7c7de92")
reference = str(root / "parity/bin/reference")
dockerfile = root / "parity/Dockerfile.playwright"
digest = hashlib.sha256(b"".join((root / "parity" / name).read_bytes() for name in ("Dockerfile.playwright", "package.json", "package-lock.json"))).hexdigest()[:12]
image = "ws8br2-playwright:" + digest
if subprocess.run(["docker", "image", "inspect", image], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode:
    subprocess.run(["docker", "build", "--label", "parity.owner=ws8br2", "-t", image, "-f", str(dockerfile), str(root / "parity")], check=True)
server = None
try:
    subprocess.run([reference, "up", "--seed", seed.name, "--port", "52610", "--time", labels["clock.now"], "--freeze", "-e", "DISABLE_SSL=1"], env=oracle_env, check=True, timeout=75)
    with (run_dir / "rust-server.log").open("w") as log:
        server = subprocess.Popen([str(binary), "server"], cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT)
        for _ in range(120):
            if server.poll() is not None:
                raise RuntimeError("Rust server exited; inspect " + str(run_dir / "rust-server.log"))
            try:
                urllib.request.urlopen("http://127.0.0.1:52611/up", timeout=1).close()
                break
            except OSError:
                time.sleep(0.25)
        else:
            raise RuntimeError("Rust server did not start")
        for name, port in (("Rails", 52610), ("Rust", 52611)):
            print(f"{name} directory browser scenarios:", flush=True)
            subprocess.run(["docker", "run", "--rm", "--network", "host", "--label", "parity.owner=ws8br2",
                            "-v", f"{root.parent}:/work:ro", "-e", f"WS8BR2_BROWSER_URL=http://127.0.0.1:{port}",
                            "-e", f"WS8BR2_BROWSER_LABELS=/work/rust/parity/.seed/{seed.name}/labels.json", image,
                            "node", "/work/rust/reference-tools/users/" + ("browser_picker.mjs" if picker_mode else "browser_people.mjs")], check=True)
finally:
    if server is not None:
        server.terminate()
        try:
            server.wait(timeout=15)
        except subprocess.TimeoutExpired:
            server.kill()
            server.wait()
    subprocess.run([reference, "down", "--port", "52610"], env=oracle_env, check=True)
    if browser_seed is not None:
        shutil.rmtree(browser_seed)
