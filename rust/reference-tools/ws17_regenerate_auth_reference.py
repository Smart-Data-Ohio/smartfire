#!/usr/bin/env python3
"""Capture the approved #163 layout/assets through the existing Rails exporters."""
import os
from pathlib import Path
import shutil
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root / ".scratch"
context = scratch / "status-layout-reference"
context.mkdir(parents=True, exist_ok=True)
status_files = [
    "app/assets/stylesheets/people.css", "app/controllers/users/statuses_controller.rb",
    "app/javascript/controllers/profile_card_controller.js", "app/views/layouts/application.html.erb",
    "app/views/users/cards/show.html.erb", "app/views/users/profiles/_status.html.erb",
    "app/views/users/sidebars/show.html.erb", "app/views/users/statuses/_fields.html.erb",
    "app/views/users/statuses/edit.html.erb", "config/routes.rb",
]
pins = {path: "2e20b24c" for path in status_files}
pins["app/models/board_automations/nudge_pusher.rb"] = "a6f10a25"
for path, pin in pins.items():
    target = context / "overlay" / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(subprocess.check_output(["git", "show", f"{pin}:{path}"], cwd=root))
(context / "Dockerfile").write_text(
    "FROM triage-reference-d7c7de92:latest\n"
    "COPY overlay/ /rails/\n"
    "RUN rm -rf public/assets && SECRET_KEY_BASE_DUMMY=1 bin/rails assets:precompile\n"
)
image = "ws17-reference-status-2e20b24c:latest"
with (scratch / "status-layout-image.log").open("w") as out:
    subprocess.run(["docker", "build", "--quiet", "-t", image, str(context)],
                   cwd=root, stdout=out, stderr=subprocess.STDOUT, check=True)
subprocess.run(["python3", "rust/reference-tools/ws17_verify_reference.py", "--status-popup"],
               cwd=root, check=True)
env = dict(os.environ, PARITY_NAMESPACE="ws17", PARITY_OWNER="ws17", PARITY_IMAGE=image)
subprocess.run([
    "rust/parity/bin/reference", "runner", "--seed", "default",
    "-e", "WS9_FULL_PAGE_GOLDENS=/work/vectors/auth_full_pages.json",
    "--time", "2026-03-02T16:00:00Z", "--freeze",
    "rust/reference-tools/auth/full_pages.rb",
], cwd=root, env=env, check=True)
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
    "-e", "ASSETS_DIR=/work/assets", "-e", "REFERENCE_SHA=2e20b24c",
    "-v", f"{assets}:/work/assets",
    "-v", f"{root}/rust/crates/assets/script/export_reference.rb:/work/export_reference.rb:ro",
    image, "bin/rails", "runner", "/work/export_reference.rb",
], cwd=root, check=True)
for source in (assets / "tests/reference").iterdir():
    shutil.copyfile(source, root / "rust/crates/assets/tests/reference" / source.name)
