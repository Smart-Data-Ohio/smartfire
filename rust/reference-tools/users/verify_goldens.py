#!/usr/bin/env python3
"""Rebuild every WS8br2 oracle and the shared layout goldens without output masks."""
from pathlib import Path
import os
import subprocess
import sys

root = Path(__file__).resolve().parents[2]
scratch = root.parent / ".scratch/verified-shared"
scratch.mkdir(parents=True, exist_ok=True)
env = os.environ.copy()
env.pop("LD_LIBRARY_PATH", None)
env.update(PARITY_RUNTIME="docker", PARITY_OWNER="ws8br2", PARITY_NAMESPACE="ws8br2-goldens",
           PARITY_IMAGE="ws8br2-reference:d7c7de92-status-2e20b24c")
reference = str(root / "parity/bin/reference")

def run(arguments):
    subprocess.run(arguments, cwd=root.parent, env=env, check=True)

def runner(seed, clock, probe, label, *arguments):
    output = scratch / (label + ".json")
    with output.open("wb") as stdout, (scratch / (label + ".log")).open("wb") as stderr:
        subprocess.run([reference, "runner", "--seed", seed, "--time", clock, "--freeze",
                        str(root / "reference-tools" / probe), *arguments],
                       cwd=root.parent, env=env, stdout=stdout, stderr=stderr, check=True)
    return output

def compare(actual, expected):
    assert actual.read_bytes() == expected.read_bytes(), (actual, expected, "raw bytes differ")

for validator in ("verify_post_pin.py", "verify_status_image.py", "verify_worker_harness.py"):
    run([sys.executable, str(root / "reference-tools/users" / validator)])
run(["bash", str(root / "reference-tools/users/run_oracles.sh")])
for seed in ("default", "first_run"):
    run([reference, "runner", "--seed", seed, "--time", "2026-03-02T16:00:00Z", "--freeze",
         str(root / "reference-tools/campfire/verify_parity_seed.rb"), seed])

core = runner("first_run", "2026-02-10T12:00:00Z", "users/status_core.rb", "core")
core_dir = scratch / "core"
run([sys.executable, str(root / "reference-tools/views/core/split.py"), str(core), str(core_dir)])
files = sorted(path for path in core_dir.rglob("*") if path.is_file())
assert len(files) == 29, len(files)
for file in files:
    compare(file, root / "crates/views/tests/golden/core" / file.relative_to(core_dir))
print("WS8br2 shared core verification: all 29 generated files match byte for byte", flush=True)

for label, probe, expected, arguments in (
    ("routes", "users/status_routes.rb", "crates/routes/routes.json", ()),
    ("recognition", "users/status_routes.rb", "vectors/campfire_routes.json", ("campfire",)),
    ("sidebar", "rooms/sidebar_page.rb", "crates/views/tests/golden/sidebar/page.json", ()),
):
    output = runner("default", "2026-03-02T16:00:00Z", probe, label, *arguments)
    compare(output, root / expected)
    print(f"WS8br2 shared {label} verification: complete generated JSON matches byte for byte", flush=True)

output = scratch / "auth-pages.json"
expression = 'output=$stdout; begin; $stdout=$stderr; load File.join(ENV.fetch("PARITY_WORK"),"reference-tools/users/status_auth_pages.rb"); ensure; $stdout=output; end; puts File.read("/rails/storage/db/auth_full_pages.json")'
with output.open("wb") as stdout, (scratch / "auth-pages.log").open("wb") as stderr:
    subprocess.run([reference, "exec", "--seed", "default", "--time", "2026-03-02T16:00:00Z", "--freeze",
                    "-e", "WS9_FULL_PAGE_GOLDENS=/rails/storage/db/auth_full_pages.json", "--",
                    "bin/rails", "runner", "--skip-executor", expression],
                   cwd=root.parent, env=env, stdout=stdout, stderr=stderr, check=True)
compare(output, root / "vectors/auth_full_pages.json")
print("WS8br2 shared WS9 verification: all 15 complete auth pages match byte for byte", flush=True)
print("WS8br2 golden verification: sources, both fresh seeds, 32 owned oracles and shared core/routes/sidebar/auth pages passed; no masks or normalization", flush=True)
