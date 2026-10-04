#!/usr/bin/env python3
"""Verify current push tags and reject a missing-tag mutation accepted by 1021's oracle."""
from status_pin_identity import PIN, PIN_FULL, PIN_IMAGE, verify_image
import json
import os
from pathlib import Path
import shutil
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = Path(os.environ.get("PARITY_PIN_LOG_DIR", root / ".scratch/review-oracle-payload"))
scratch.mkdir(parents=True, exist_ok=True)
verify_image()
subprocess.run(["python3", "rust/reference-tools/ws17_verify_reference.py"], cwd=root, check=True)

historical = subprocess.check_output(
    ["git", "show", "1021be6a:rust/reference-tools/ws17_notification_push.rb"], cwd=root,
)
assert b"reverse_merge(tag:nil)" in historical
(scratch / "historical-1021.rb").write_bytes(historical)
current = (root / "rust/reference-tools/ws17_notification_push.rb").read_text()
constructor = "WebPush::Notification.new(**payload,"
assert current.count(constructor) == 1
queue = "  pool.define_singleton_method(:queue) do |payload,subs|\n"
assert current.count(queue) == 1
missing_tag = current.replace(
    queue, queue + '   payload=payload.except(:tag) if payload[:tag].to_s.start_with?("board-nudge-")\n',
)
permissive = missing_tag.replace(constructor, "WebPush::Notification.new(**payload.reverse_merge(tag:nil),")


def capture(name, script):
    source = scratch / f"{name}.rb"
    source.write_text(script)
    storage = scratch / name
    shutil.rmtree(storage, ignore_errors=True)
    (storage / "db").mkdir(parents=True)
    shutil.copy2(root / "rust/parity/.seed/default/db/production.sqlite3", storage / "db/production.sqlite3")
    result = subprocess.run([
        "docker", "run", "--rm", "--network", "none", "--name", f"ws17-oracle-{name}",
        "--cpus", os.environ.get("PARITY_CPUS", "1"),
        "--env-file", str(root / "rust/parity/.env.reference"),
        "-e", f"PARITY_REFERENCE_SHA={PIN_FULL}", "-e", "PARITY_REDIS=1",
        "-v", f"{storage}/db:/rails/storage/db",
        "-v", f"{root}/rust/parity/.seed/default/storage:/rails/storage/files:ro",
        "-v", f"{source}:/oracle.rb:ro", PIN_IMAGE, "bin/rails", "runner", "/oracle.rb",
    ], cwd=root, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    (scratch / f"{name}.stdout").write_text(result.stdout)
    (scratch / f"{name}.stderr").write_text(result.stderr)
    return result


def value(result):
    assert result.returncode == 0, result.stderr
    data = json.loads(next(line for line in result.stdout.splitlines() if line.startswith('{"reference":')))
    assert data["reference"] == PIN and data["board_reference"] == PIN
    assert len(data["rows"]) == 50
    return data


baseline = value(capture("current", current))
for row in baseline["rows"]:
    for delivery in row["deliveries"]:
        tag = delivery["payload"]["tag"]
        assert isinstance(tag, str) and tag, row["name"]
        for encoded in delivery["encoded"]:
            assert json.loads(encoded)["options"]["tag"] == tag, row["name"]
board = next(row for row in baseline["rows"] if row["name"] == "board_nudge")
assert len(board["deliveries"]) == 1 and len(board["deliveries"][0]["encoded"]) == 1
assert board["deliveries"][0]["payload"]["tag"].startswith("board-nudge-")

rejected = capture("missing-tag", missing_tag)
assert rejected.returncode != 0 and "missing keyword: :tag" in rejected.stderr, rejected.stderr
accepted = value(capture("historical-permissive", permissive))
board = next(row for row in accepted["rows"] if row["name"] == "board_nudge")
assert "tag" not in board["deliveries"][0]["payload"]
assert json.loads(board["deliveries"][0]["encoded"][0])["options"]["tag"] is None
print(f"Current {PIN} oracle: 50 complete rows; every delivered tag matches its encoded notification")
print("Missing board tag: current WebPush::Notification rejects mutation with ArgumentError")
print("Historical 1021 permissive constructor: accepts the same mutation and encodes a null tag")
