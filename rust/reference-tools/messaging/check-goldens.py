#!/usr/bin/env python3
"""Re-run the pinned Rails oracles from fresh seeds and reject any changed golden bytes."""
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[3]
SCRATCH = ROOT / ".scratch/messaging-golden-check"
IMAGE = os.environ.get("PARITY_IMAGE", "triage-reference-d7c7de92")
for folder in ["db", "files", "out"]:
    (SCRATCH / folder).mkdir(parents=True, exist_ok=True)

for name in ["preview", "fragments", "root", "paging", "broadcasts", "thread-memberships", "collection", "room-list", "message-states", "thread-message-reads", "thread-message-writes", "thread-pages", "thread-lifecycle", "thread-content"]:
    shutil.copyfile(ROOT / "rust/parity/.seed/default/db/production.sqlite3", SCRATCH / "db/production.sqlite3")
    args = ["docker", "run", "--rm", "--cpus", "2", "--name", f"ws8bm-goldens-{name}",
            "--user", f"{os.getuid()}:{os.getgid()}", "--env-file", str(ROOT / "rust/parity/.env.reference"),
            "-e", "RAILS_LOG_LEVEL=warn", "-e", "PARITY_REDIS=1",
            "-v", f"{SCRATCH / 'db'}:/rails/storage/db", "-v", f"{SCRATCH / 'files'}:/rails/storage/files",
            "-v", f"{SCRATCH / 'out'}:/out", "-v", f"{ROOT / 'rust'}:/work:ro", IMAGE,
            "bash", "-c", f"bin/rails runner --skip-executor /work/reference-tools/messaging/{name}.rb /out/{name}.json"]
    run = subprocess.run(args, cwd=ROOT, capture_output=True, text=True)
    (SCRATCH / f"{name}.log").write_text(run.stdout + run.stderr)
    assert run.returncode == 0, f"{name}: reference failed; inspect {SCRATCH / (name + '.log')}"
    for line in run.stdout.splitlines():
        if line.startswith("WS8bm "):
            print(line, flush=True)
    files = [f"{name}.json"] + (["index-template-digest.txt"] if name == "paging" else [])
    for file in files:
        assert (SCRATCH / "out" / file).read_bytes() == (ROOT / "rust/vectors/messaging" / file).read_bytes(), f"{file}: golden bytes differ"
print("WS8bm golden check: 14 Rails oracles re-run; 15 golden files byte-identical", flush=True)
