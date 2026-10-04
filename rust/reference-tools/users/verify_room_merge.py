#!/usr/bin/env python3
"""Generate or compare room-owner goldens from the plain pinned Rails image."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("--only", nargs="+")
parser.add_argument("--write", action="store_true")
args = parser.parse_args()

root = Path(__file__).resolve().parents[3]
scratch = Path(os.environ.get("WS8BR2_ROOM_ORACLE_DIR", root / "rust/.scratch/verified-room-merge"))
scratch.mkdir(parents=True, exist_ok=True)
env = os.environ.copy()
env.pop("LD_LIBRARY_PATH", None)
env.update(PARITY_RUNTIME="docker", PARITY_OWNER="ws8br2",
           PARITY_NAMESPACE="ws8br2-room-merge")
image = os.environ.get("PARITY_IMAGE", "campfire-reference")
env["PARITY_IMAGE"] = image
reference = (root / "rust/parity/reference.sha").read_text().strip()
environment = subprocess.check_output(["docker", "image", "inspect", "--format",
                                      "{{range .Config.Env}}{{println .}}{{end}}", image], text=True)
assert [line.removeprefix("GIT_REVISION=") for line in environment.splitlines()
        if line.startswith("GIT_REVISION=")] == [reference], "room oracles require the pinned reference image"

sources = [
    "app/views/rooms/show.html.erb",
    "app/views/rooms/show/_nav.html.erb",
    "app/views/messages/_message.html.erb",
    "app/views/layouts/application.html.erb",
    "app/views/rooms/show/_thread_panel.html.erb",
    "app/views/work_threads/_guide.html.erb",
    "app/views/polls/_builder.html.erb",
    "app/views/rooms/pins/_panel.html.erb",
    "app/views/rooms/pins/_count.html.erb",
    "app/models/rooms/direct.rb",
]
actual = subprocess.check_output([
    "docker", "run", "--rm", "--network", "none", "--name", "ws8br2-merge-source",
    "--entrypoint", "sha256sum", image,
    *["/rails/" + path for path in sources],
], env=env, text=True)
for line, path in zip(actual.splitlines(), sources, strict=True):
    wanted = subprocess.check_output(["git", "show", reference + ":" + path], cwd=root)
    assert line.split()[0] == hashlib.sha256(wanted).hexdigest(), path
print(f"WS8br2 room merge sources: 10 plain pinned files verified; reference {reference}", flush=True)

cases = [
    ("full-pages", "rooms/full_pages.rb", "crates/campfire/src/controllers/rooms/full_pages.json", None),
    ("empty-shell", "rooms/empty_shell_http.rb", "crates/campfire/src/controllers/rooms/empty_shell_http.json", None),
    ("panels", "rooms/owner_panels.rb", "crates/campfire/src/controllers/rooms/owner_panels.json", None),
    ("pins", "rooms/owner_pins.rb", "crates/campfire/src/controllers/rooms/owner_pins.json", None),
    ("pr-thread", "rooms/thread_review_http.rb", "vectors/messaging/pr-thread-http.json", None),
    ("picker", "rooms/picker_components.rb", "crates/campfire/src/controllers/rooms/picker_config.json", None),
    ("direct-forms", "rooms/direct_forms.rb", "vectors/direct_forms.json", None),
    ("native-components", "rooms/native_components.rb", "crates/views/tests/golden/rooms/native_components.json", None),
    ("direct-selection", "rooms/direct_selection.rb", "vectors/direct_selection.json", None),
    ("direct-rename", "rooms/direct_rename.rb", "vectors/direct_rename.json", None),
    ("inbound", "rooms/inbound_section.rb", "vectors/inbound_section.json", None),
    ("icons", "rooms/icons.rb", "vectors/room_icons.json", None),
    ("coercions", "rooms/coercions.rb", "vectors/room_coercions.json", None),
    ("forms", "rooms/forms.rb", "vectors/room_access_forms.json", None),
    ("reads", "rooms/reads_refresh.rb", "vectors/reads_refresh.json", None),
    ("members", "rooms/members.rb", "crates/views/tests/golden/rooms/members.json", None),
    ("directory", "rooms/directory.rb", "crates/views/tests/golden/rooms/directory.json", None),
    ("pin-refresh", "rooms/pin_refresh.rb", "crates/views/tests/golden/rooms/pin_refresh.json", None),
    ("join", "rooms/join.rb", "vectors/rooms_join.json", None),
    ("http", "rooms/http.rb", "vectors/rooms_http.json", None),
    ("event-pages", "events/pages.rb", "crates/views/tests/golden/event-pages.json", "event-pages.json"),
    ("event-fragments", "events/fragments.rb", "crates/views/tests/golden/event-fragments.json", "event-fragments.json"),
]
if args.only and set(args.only) - {case[0] for case in cases}:
    parser.error("unknown --only corpus")
checked = 0
for label, probe, expected, output_file in cases:
    if args.only and label not in args.only:
        continue
    source = "/work/reference-tools/" + probe
    command = ["bin/rails", "runner", "--skip-executor", source]
    if output_file:
        # The event recorders write their JSON to a file and progress to stdout.
        # Preserve their output bytes while directing only progress to stderr.
        expression = ("output=$stdout; begin; $stdout=$stderr; load " + repr(source) +
                      "; ensure; $stdout=output; end; puts File.read(" +
                      repr("/rails/storage/db/" + output_file) + ")")
        command[-1] = expression
    clock = "2026-09-22T12:00:00Z" if label == "event-fragments" else "2026-03-02T16:00:00Z"
    result = subprocess.run([
        str(root / "rust/parity/bin/reference"), "exec", "--seed", "first_run" if output_file else "default",
        "--time", clock, "--freeze", "--", *command,
    ], cwd=root, env=env, capture_output=True)
    (scratch / (label + ".stdout")).write_bytes(result.stdout)
    (scratch / (label + ".stderr")).write_bytes(result.stderr)
    result.check_returncode()
    # Collection-digest warnings can precede JSON on the Rails transport. Decode
    # only that envelope; all JSON string/HTML values are compared verbatim.
    payload = result.stdout.decode()
    start = 0 if payload.startswith(("{", "[")) else payload.index("\n{") + 1
    captured_bytes = payload[start:].encode()
    captured = json.loads(captured_bytes)
    for path, digest in (captured.get("sources", {}) if isinstance(captured, dict) else {}).items():
        wanted = subprocess.check_output(["git", "show", reference + ":" + path], cwd=root)
        assert digest == hashlib.sha256(wanted).hexdigest(), path
    fixture = root / "rust" / expected
    if args.write:
        fixture.write_bytes(captured_bytes)
        print(f"WS8br2 room merge oracle {label}: fresh plain pinned JSON written", flush=True)
    else:
        assert captured_bytes == fixture.read_bytes(), label
        print(f"WS8br2 room merge oracle {label}: JSON and rendered bytes match byte for byte", flush=True)
    checked += 1
action = "written" if args.write else "passed"
print(f"WS8br2 room merge goldens: {checked} owner corpora {action}; no masks or normalization", flush=True)
