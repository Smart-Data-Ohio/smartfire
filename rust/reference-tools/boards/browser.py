#!/usr/bin/env python3
"""Run identical board read interactions against isolated Rails and Rust seeds."""
from pathlib import Path
import hashlib
import json
import os
import shutil
import socket
import sys
import subprocess
import tempfile
import time
import urllib.request

root = Path(__file__).resolve().parents[2]
scratch = root.parent / ".scratch"
scratch.mkdir(exist_ok=True)
script = "post_browser.mjs" if "--posts" in sys.argv else "browser.mjs"
run_dir = Path(tempfile.mkdtemp(prefix="ws12-browser-boards-",dir=scratch))
seed = root / "parity/.seed/default"
labels = json.loads((seed / "labels.json").read_text())
target = Path(os.environ.get("CARGO_TARGET_DIR",scratch / "target"))
binary = target / "debug/campfire"
assert binary.is_file(), "build the debug campfire binary first"
for port in (53410,53411,53412):
    with socket.socket() as check:
        check.bind(("127.0.0.1",port))
env = os.environ.copy()
for line in (root / "parity/.env.reference").read_text().splitlines():
    if line and not line.startswith("#") and "=" in line:
        key,value = line.split("=",1)
        env[key] = value
env.update(CAMPFIRE_STORAGE_PATH=str(run_dir),CAMPFIRE_FROZEN_TIME=labels["clock.now"],
           DISABLE_SSL="1",HTTP_PORT="53411",TARGET_PORT="53412")
env.pop("TLS_DOMAIN",None)
shutil.copytree(seed / "db",run_dir / "db")
shutil.copytree(seed / "storage",run_dir / "files")
oracle_env = os.environ.copy()
oracle_env.pop("LD_LIBRARY_PATH",None)
oracle_env.update(PARITY_NAMESPACE="ws12",PARITY_OWNER="ws12",PARITY_RUNTIME="docker",
                  PARITY_IMAGE="ws12-reference:boards-b908ebc2")
reference = str(root / "parity/bin/reference")
digest = hashlib.sha256(b"".join((root / "parity" / name).read_bytes() for name in ("Dockerfile.playwright","package.json","package-lock.json"))).hexdigest()[:12]
image = "ws12-playwright:" + digest
if subprocess.run(["docker","image","inspect",image],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode:
    existing = "ws8br2-playwright:" + digest
    if subprocess.run(["docker","image","inspect",existing],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode == 0:
        subprocess.run(["docker","tag",existing,image],check=True)
    else:
        subprocess.run(["docker","build","--label","parity.owner=ws12","-t",image,"-f",str(root / "parity/Dockerfile.playwright"),str(root / "parity")],check=True)
server = None
try:
    subprocess.run([reference,"up","--seed","default","--port","53410","--time",labels["clock.now"],"--freeze","-e","DISABLE_SSL=1"],env=oracle_env,check=True)
    with (run_dir / "rust-server.log").open("w") as log:
        server = subprocess.Popen([str(binary),"server"],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT)
        for _ in range(120):
            if server.poll() is not None:
                raise RuntimeError("Rust server exited; inspect " + str(run_dir / "rust-server.log"))
            try:
                urllib.request.urlopen("http://127.0.0.1:53411/up",timeout=1).close()
                break
            except OSError:
                time.sleep(.25)
        else:
            raise RuntimeError("Rust server did not start")
        for name,port in (("Rails",53410),("Rust",53411)):
            print(f"{name} board read browser scenarios:",flush=True)
            subprocess.run(["docker","run","--rm","--name",f"ws12-boards-browser-{name.lower()}","--network","host",
                "--label","parity.owner=ws12","-v",f"{root.parent}:/work:ro",
                "-e",f"WS12_BROWSER_URL=http://127.0.0.1:{port}","-e","WS12_BROWSER_LABELS=/work/rust/parity/.seed/default/labels.json",
                image,"node",f"/work/rust/reference-tools/boards/{script}"],check=True)
finally:
    if server is not None:
        server.terminate()
        try:
            server.wait(timeout=15)
        except subprocess.TimeoutExpired:
            server.kill()
            server.wait()
    subprocess.run([reference,"down","--port","53410"],env=oracle_env,check=True)
