#!/usr/bin/env python3
"""Independently replay each owned Rails oracle from this checkout's rebuilt seed."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[3]
SCRATCH = Path(os.environ.get("WS8BM2_ORACLE_SCRATCH", ROOT / ".scratch/continuation-oracles"))
SCRATCH.mkdir(parents=True, exist_ok=True)
ENV = dict(os.environ, PARITY_NAMESPACE="ws8bm2", PARITY_OWNER="ws8bm2",
           PARITY_RUNTIME="docker", PARITY_IMAGE="ws8bm2-reference:d7c7de92")
NAMES = ["features", "saved", "scheduled", "search", "preloads", "slash", "links_files",
         "reminder_push", "quote_integration", "root_cache", "panels", "date_inputs", "review_saved_race", "review_dates", "date_compact_widths", "providers", "provider_edits", "event_cards", "date_coercions", "composer", "composer_sti", "twitter_preloads", "twitter_cards", "twitter_text", "provider_callbacks", "agent_command", "user_coercions", "date_years",
         "ws12_consumers", "provider_batch", "private_provider_pages", "search_headers",
         "older_provider_callbacks", "bounded_provider_callbacks", "mapped_provider_callbacks",
         "older_owner_callbacks", "older_calendar_callbacks", "older_calendar_jobs", "older_embed_jobs", "pin_poll_scaling", "exceptional_inputs", "slash_named", "older_calendar_execution", "older_embed_children", "older_embed_failures", "final_state_siblings",
         "calendar_retry_consumers", "relative_split_inputs", "container_inputs", "relative_consumers", "adapter_rejections"]
# Main's #213 adds a stored non-HTTP quote case. Its golden uses the exact
# reviewed Rails template revision, while every other corpus keeps the shared pin.
TWITTER_ENV = dict(ENV, PARITY_IMAGE="ws8bm2-reference:twitter-955af4c37",
                   PARITY_REFERENCE_REVISION="955af4c3781bef07b97b7aefce12c376110a812c")
if subprocess.run(["docker", "image", "inspect", TWITTER_ENV["PARITY_IMAGE"]],
                  stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode:
    subprocess.run([str(ROOT / "rust/reference-tools/messaging/build-twitter-reference.sh")],
                   cwd=ROOT, env=ENV, check=True)
subprocess.run(["python3", str(ROOT / "rust/reference-tools/messaging/features-reference-check.py")],
               cwd=ROOT, env=TWITTER_ENV, check=True)
for name in NAMES:
    storage = Path(tempfile.mkdtemp(prefix=f"{name}-", dir=SCRATCH))
    shutil.copytree(ROOT / "rust/parity/.seed/default", storage, dirs_exist_ok=True)
    command = [str(ROOT / "rust/parity/bin/reference"), "runner", "--storage", str(storage),
               "--time", "2026-03-02T16:00:00Z", "--freeze",
               str(ROOT / f"rust/reference-tools/{'embeds' if name in ('twitter_cards','twitter_text') else 'messaging'}/{name}.rb"),
               f"/rails/storage/db/{name}.json"]
    run = subprocess.run(command, cwd=ROOT, env=TWITTER_ENV if name == "twitter_cards" else ENV, capture_output=True, text=True)
    (storage / "runner.log").write_text(run.stdout + run.stderr)
    assert run.returncode == 0, f"{name}: see {storage}/runner.log"
    actual = (storage / f"db/{name}.json").read_bytes()
    expected = (ROOT / (f"rust/vectors/ws15e_{name}.json" if name in ("twitter_cards", "twitter_text") else f"rust/vectors/messaging/{name}.json")).read_bytes()
    assert actual == expected, f"{name}: committed bytes differ from independently replayed Rails oracle"
    for line in run.stdout.splitlines():
        if line.startswith("WS8bm2"):
            print(line, flush=True)
    print(f"WS8bm2 oracle replay: {name}.json byte-identical", flush=True)
print(f"WS8bm2 oracle replay: {len(NAMES)}/{len(NAMES)} independently replayed fixtures byte-identical", flush=True)
