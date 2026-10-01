#!/usr/bin/env python3
"""Re-execute the room/message owners' Rails recorders after the WS8br2 merge.

Compare every recorded value, including complete HTML strings, without altering
the owners' fixtures. Use this worker's private containers and declared images.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("--only", nargs="+")
args = parser.parse_args()

root = Path(__file__).resolve().parents[3]
scratch = root / ".scratch/verified-room-merge"
scratch.mkdir(parents=True, exist_ok=True)
env = os.environ.copy()
env.pop("LD_LIBRARY_PATH", None)
env.update(PARITY_RUNTIME="docker", PARITY_OWNER="ws8br2",
           PARITY_NAMESPACE="ws8br2-room-merge")
pin_image = "ws8br2-reference:d7c7de92"
status_image = "ws8br2-reference:d7c7de92-status-2e20b24c"

# These inputs compose the conflicted room template and its owner panels. The
# status-image validator checks the complete pin/approved-overlay provenance.
sources = {
    "app/views/rooms/show.html.erb": "d7c7de92",
    "app/views/rooms/show/_nav.html.erb": "d7c7de92",
    "app/views/messages/_message.html.erb": "d7c7de92",
    "app/views/layouts/application.html.erb": "2e20b24c",
    "app/views/rooms/show/_thread_panel.html.erb": "d7c7de92",
    "app/views/work_threads/_guide.html.erb": "d7c7de92",
    "app/views/polls/_builder.html.erb": "d7c7de92",
    "app/views/rooms/pins/_panel.html.erb": "d7c7de92",
    "app/views/rooms/pins/_count.html.erb": "d7c7de92",
    "app/models/rooms/direct.rb": "d7c7de92",
}
actual = subprocess.check_output([
    "docker", "run", "--rm", "--network", "none", "--name", "ws8br2-merge-source",
    "--entrypoint", "sha256sum", status_image,
    *["/rails/" + path for path in sources],
], env=env, text=True)
for line, (path, revision) in zip(actual.splitlines(), sources.items(), strict=True):
    wanted = subprocess.check_output(["git", "show", revision + ":" + path], cwd=root)
    assert line.split()[0] == hashlib.sha256(wanted).hexdigest(), path
print("WS8br2 room merge sources: 10 pinned/approved files verified", flush=True)

cases = [
    ("full-pages", "rooms/full_pages.rb", "crates/campfire/src/controllers/rooms/full_pages.json", status_image, None),
    ("empty-shell", "rooms/empty_shell_http.rb", "crates/campfire/src/controllers/rooms/empty_shell_http.json", status_image, None),
    ("panels", "rooms/owner_panels.rb", "crates/campfire/src/controllers/rooms/owner_panels.json", pin_image, None),
    ("pins", "rooms/owner_pins.rb", "crates/campfire/src/controllers/rooms/owner_pins.json", pin_image, None),
    ("pr-thread", "rooms/thread_review_http.rb", "vectors/messaging/pr-thread-http.json", pin_image, None),
    ("picker", "rooms/picker_components.rb", "crates/campfire/src/controllers/rooms/picker_config.json", pin_image, None),
    ("direct-forms", "rooms/direct_forms.rb", "vectors/direct_forms.json", pin_image, None),
    ("native-components", "rooms/native_components.rb", "crates/views/tests/golden/rooms/native_components.json", pin_image, None),
    ("event-pages", "events/pages.rb", "crates/views/tests/golden/event-pages.json", status_image, "event-pages.json"),
    ("event-fragments", "events/fragments.rb", "crates/views/tests/golden/event-fragments.json", pin_image, "event-fragments.json"),
]
for label, probe, expected, image, output_file in cases:
    if args.only and label not in args.only:
        continue
    env["PARITY_IMAGE"] = image
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
    captured = json.loads(payload[start:])
    assert captured == json.loads((root / "rust" / expected).read_bytes()), label
    for path, digest in (captured.get("sources", {}) if isinstance(captured, dict) else {}).items():
        revision = "2e20b24c" if path == "app/views/layouts/application.html.erb" else "d7c7de92"
        wanted = subprocess.check_output(["git", "show", revision + ":" + path], cwd=root)
        assert digest == hashlib.sha256(wanted).hexdigest(), path
    print(f"WS8br2 room merge oracle {label}: all recorded values and rendered bytes unchanged", flush=True)
print(f"WS8br2 room merge goldens: {len(args.only or cases)} owner corpora passed; rendered bytes identical; no masks or normalization", flush=True)
