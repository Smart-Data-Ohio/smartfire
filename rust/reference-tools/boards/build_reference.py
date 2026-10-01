#!/usr/bin/env python3
"""Layer the three approved board changes onto the pinned Rails/status reference."""
from pathlib import Path
import os
import subprocess

root = Path(__file__).resolve().parents[2]
repo = root.parent
context = repo / ".scratch/board-image"
context.mkdir(parents=True, exist_ok=True)
files = (
    "app/models/board_automations/nudge_pusher.rb",
    "app/views/channel_threads/_board_post.html.erb",
    "app/views/channel_threads/new.html.erb",
)
reference = os.environ.get("WS12_BOARD_REFERENCE", "origin/main")
image = os.environ.get("WS12_BOARD_IMAGE", "ws12-reference:boards-b908ebc2")
for name in files:
    path = context / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(subprocess.check_output(["git", "show", f"{reference}:{name}"], cwd=repo))
(context / "Dockerfile").write_text(
    'FROM ws8br2-reference:d7c7de92-status-2e20b24c\nLABEL parity.owner="ws12"\n'
    + "".join(f"COPY {name} /rails/{name}\n" for name in files)
)
subprocess.run(["docker", "build", "-t", image, str(context)], check=True)
probe = 'require "json"; require "digest"; JSON.parse(File.read("/source-hashes.json")).each { |file,hash| abort("source drift: #{file}") unless Digest::SHA256.file("/rails/#{file}").hexdigest == hash }; puts "WS12 board reference: 18 source hashes verified; d7c7de92 plus approved status/board drift"'
subprocess.run(["docker", "run", "--rm", "--label", "parity.owner=ws12", "--entrypoint", "ruby",
    "-v", f"{root}/reference-tools/boards/source-hashes.json:/source-hashes.json:ro", image, "-e", probe], check=True)
