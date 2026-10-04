#!/usr/bin/env python3
"""Capture the approved #163 layout/assets through the existing Rails exporters."""
import os
from pathlib import Path
import shutil
import sys
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = Path(os.environ.get("PARITY_PIN_LOG_DIR", root / ".scratch"))
import argparse
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--skip-shared-exports", action="store_true")
options = parser.parse_args()
image = os.environ.get("PARITY_IMAGE", "campfire-reference")
subprocess.run(["python3", "rust/reference-tools/ws17_verify_reference.py", "--status-popup"], cwd=root, check=True)
env = dict(os.environ, PARITY_NAMESPACE="ws17", PARITY_OWNER="ws17", PARITY_IMAGE=image)
subprocess.run([
    "rust/parity/bin/reference", "runner", "--seed", "default",
    "-e", "WS9_FULL_PAGE_GOLDENS=/work/vectors/auth_full_pages.json",
    "--time", "2026-03-02T16:00:00Z", "--freeze",
    "rust/reference-tools/auth/full_pages.rb",
], cwd=root, env=env, check=True)
if options.skip_shared_exports:
    print("Auth full pages regenerated; shared core/assets exports left to their owner")
    sys.exit(0)
# Export every original core case to scratch. Only foundation layouts/public pages and
# the changed asset preload partial enter this integration; other owner UI stays separate.
capture = scratch / "status-layout-core-capture"
env.update(STORE=str(scratch / "status-layout-core-store"), OUT=str(capture))
subprocess.run(["bash", "rust/reference-tools/views/core/run.sh"], cwd=root, env=env, check=True)
target = root / "rust/crates/views/tests/golden/core"
paths = list((capture / "layouts").glob("*.html"))
paths += list((capture / "pages").glob("public_pages_*.html"))
paths += [capture / "partials/first_paint_controller_preloads.html"]
for source in paths:
    shutil.copyfile(source, target / source.relative_to(capture))
print(f"Approved layout goldens: {len(paths)} complete foundation cases; reference 2e20b24c")
# Export the compiled-byte, importmap, CSS and static-response cases with the original
# Propshaft/Rack exporter. Gem vendor trees are unchanged and remain on the pin.
assets = scratch / "status-layout-assets-capture"
assets.mkdir(exist_ok=True)
subprocess.run([
    "docker", "run", "--rm", "--name", "ws17-approved-assets-export",
    "--user", f"{os.getuid()}:{os.getgid()}",
    "-e", "RAILS_ENV=production", "-e", "SECRET_KEY_BASE_DUMMY=1", "-e", "DISABLE_SSL=1",
    "-e", "ASSETS_DIR=/work/assets", "-e", f"REFERENCE_SHA={(root / 'rust/parity/reference.sha').read_text().strip()}",
    "-v", f"{assets}:/work/assets",
    "-v", f"{root}/rust/crates/assets/script/export_reference.rb:/work/export_reference.rb:ro",
    image, "bin/rails", "runner", "/work/export_reference.rb",
], cwd=root, check=True)
for source in (assets / "tests/reference").iterdir():
    shutil.copyfile(source, root / "rust/crates/assets/tests/reference" / source.name)
