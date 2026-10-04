#!/usr/bin/env python3
"""Run the WS8bm controller files from the pinned Rails test archive, grouped by file."""
import io
import os
from pathlib import Path
import re
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parents[3]
PIN = (ROOT / "rust/parity/reference.sha").read_text().strip()
IMAGE = os.environ.get("PARITY_IMAGE", "campfire-reference")
image_env = subprocess.check_output(
    ["docker", "image", "inspect", "--format", "{{range .Config.Env}}{{println .}}{{end}}", IMAGE], text=True,
)
assert f"GIT_REVISION={PIN}" in image_env.splitlines(), "controller reference must match parity/reference.sha"
SCRATCH = ROOT / ".scratch/messaging-controller-reference"
SCRATCH.mkdir(parents=True, exist_ok=True)
archive = subprocess.check_output(["git", "archive", PIN, "test"], cwd=ROOT)
with tarfile.open(fileobj=io.BytesIO(archive)) as bundle:
    bundle.extractall(SCRATCH, filter="data")
FILES = [
    "test/controllers/messages_controller_test.rb",
    "test/controllers/messages_drive_attachments_test.rb",
    "test/controllers/messages/cached_fragment_csrf_test.rb",
    "test/controllers/messages/legacy_presentation_cache_test.rb",
    "test/controllers/messages/boosts_controller_test.rb",
    "test/controllers/channel_threads_controller_test.rb",
    "test/controllers/channel_thread_messages_controller_test.rb",
    "test/controllers/channel_thread_messages_drive_attachments_test.rb",
    "test/controllers/message_forwards_controller_test.rb",
    "test/controllers/message_forward_sources_controller_test.rb",
]
script = "redis-server --daemonize yes; bin/rails db:prepare >/dev/null; "
for file in FILES:
    script += f"printf '%s\\n' '{file}'; bin/rails test '{file}'; "
run = subprocess.run(["docker", "run", "--rm", "--cpus", "2", "--name", "ws8bm-controller-files-final",
                      "--entrypoint", "sh", "--env-file", str(ROOT / "rust/parity/.env.reference"),
                      "-e", "RAILS_ENV=test", "-e", "PARALLEL_WORKERS=1", "-e", "RAILS_LOG_LEVEL=warn",
                      "-v", f"{SCRATCH / 'test'}:/rails/test:ro",
                      IMAGE, "-ec", script],
                     cwd=ROOT, capture_output=True, text=True)
(SCRATCH / "run.log").write_text(run.stdout + run.stderr)
assert run.returncode == 0, f"Rails controller reference failed; inspect {SCRATCH / 'run.log'}"
summaries = re.findall(r"^[0-9]+ runs, .*assertions, 0 failures, 0 errors, 0 skips$", run.stdout, re.M)
assert len(summaries) == len(FILES), "Missing per-file reference pass count"
for file, summary in zip(FILES, summaries):
    print(file)
    print(summary)
print("WS8bm Rails controller reference: 10 files passed; reference counts only", flush=True)
