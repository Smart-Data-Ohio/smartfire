#!/usr/bin/env python3
"""Prove the new HTTP checks reject compiled regressions; always restore the source."""
from pathlib import Path
import os
import subprocess
import sys

root = Path(__file__).resolve().parents[2]
scratch = root.parent / ".scratch" / "users-discrimination"
scratch.mkdir(parents=True, exist_ok=True)
env = os.environ.copy()
env.update(CI="1", TMPDIR=str(root.parent / ".scratch" / "tmp"))
mutations = [
    ("public-cookie", "crates/campfire/src/controllers/public_pages.rs", "let formats = c.formats()?;", "let _ = c.form_authenticity_token();\n    let formats = c.formats()?;", "public_pages_bypass_authentication_browser_and_private_state"),
    ("public-escaping", "crates/views/templates/public_pages/about.html", "{{ operator_name }}", "{{ operator_name|safe }}", "public_page_bodies_match_rails_with_operator_escaping_and_email_uri_encoding"),
    ("public-bytes", "crates/views/templates/public_pages/about.html", "<h1>About Smartfire</h1>", "<h1>About  Smartfire</h1>", "public_page_bodies_match_rails_with_operator_escaping_and_email_uri_encoding"),
    ("avatar-boundary", "crates/db/src/models/user.rs", r"\p{Mark}", "", "default_initials_svg_matches_rails_bytes_and_cache_validation"),
    ("qr-capacity", "crates/campfire/src/controllers/qr_code.rs", 'ok_or_else(|| Error::internal(anyhow::anyhow!("Data length exceed maximum capacity of version 40")))?', "ok_or(Error::Status(StatusCode::UNPROCESSABLE_ENTITY))?", "qr_http_matches_rails_bytes_cache_and_capacity_errors"),
    ("pwa-bytes", "crates/views/templates/pwa/service_worker.js", 'const STATIC_CACHE = "smartfire-static-v1"', 'const STATIC_CACHE = "smartfire-static-v2"', "pwa_http_bodies_match_rails_before_and_after_first_run"),
]
selected = set(sys.argv[1:])
count = 0
for name, relative, before, after, test in mutations:
    if selected and name not in selected:
        continue
    path = root / relative
    original = path.read_bytes()
    source = original.decode()
    assert source.count(before) == 1, (name, "mutation anchor", source.count(before))
    try:
        path.write_text(source.replace(before, after))
        run = subprocess.run(["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j", "4", "-p", "campfire", test, "--", "--nocapture"], cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        (scratch / f"{name}.log").write_text(run.stdout)
        summaries = [line for line in run.stdout.splitlines() if line.startswith("test result:")]
        assert run.returncode == 101 and summaries and "FAILED" in summaries[-1] and f"::{test} ... FAILED" in run.stdout, (name, run.stdout[-4000:])
        print(f"{name}: {summaries[-1]}", flush=True)
        count += 1
    finally:
        path.write_bytes(original)
assert count, "no mutations selected"
print(f"WS8br2 discrimination: {count} compiled regressions detected; sources restored")
