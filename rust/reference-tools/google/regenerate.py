#!/usr/bin/env python3
"""Regenerate explicitly owned Google goldens from independent copies of the current Rails seed."""
import argparse
import json
import os
import re
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]

OUTPUTS = {'google_admin_links.json': ('admin-links.rb', []), 'google_agent_delivery.json': ('agent-delivery.rb', []), 'google_api.json': ('api.rb', []), 'google_attendance_params.json': ('attendance-params.rb', []), 'google_auth_security.json': ('auth-security.rb', []), 'google_calendar_consumers.json': ('consumers.rb', []), 'google_calendar_retries.json': ('retries.rb', []), 'google_cleanup_report.json': ('cleanup-report.rb', []), 'google_connection_cases.json': ('connection-cases.rb', []), 'google_controller_cases.json': ('controller-cases.rb', []), 'google_drive_agent_auth.json': ('drive-agent-auth.rb', []), 'google_drive_link.json': ('drive-link.rb', []), 'google_drive_null.json': ('drive-null.rb', []), 'google_drive_policy.json': ('drive-policy.rb', []), 'google_endpoint_cases.json': ('endpoint-cases.rb', []), 'google_entry.json': ('entry.rb', []), 'google_full_pages.json': ('full-pages.rb', []), 'google_inbound_preload.json': ('inbound-preload.rb', []), 'google_lifecycle.json': ('lifecycle.rb', []), 'google_meeting_intervals.json': ('meeting-intervals.rb', []), 'google_message_drive.json': ('message-drive.rb', []), 'google_periodic.json': ('periodic.rb', []), 'google_picker.json': ('picker.rb', []), 'google_profile_html.json': ('profile-html.rb', []), 'google_recipient_cases.json': ('recipient-cases.rb', []), 'google_refresh_races.json': ('refresh-race.rb', []), 'google_sign_in.json': ('sign_in.rb', []), 'google_sign_in_html.json': ('html.rb', []), 'google_sync_entry_cases.json': ('sync-entry-cases.rb', []), 'google_time_boundaries.json': ('time-boundaries.rb', []), 'google_webhook.json': ('webhook.rb', [])}
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("output", type=Path)
parser.add_argument("--write", action="store_true")
parser.add_argument("--names", default="")
options = parser.parse_args()
options.output = options.output.resolve()
options.output.mkdir(parents=True, exist_ok=True)
pin = (ROOT / "rust/parity/reference.sha").read_text().strip()
helper = ROOT / "rust/reference-tools/google/google_calendar_test_helper.rb"
assert helper.read_bytes() == subprocess.check_output(["git", "show", f"{pin}:test/test_helpers/google_calendar_test_helper.rb"], cwd=ROOT), "Google helper source drift"
outputs = OUTPUTS
if options.names:
    names = set(options.names.split(","))
    assert names <= outputs.keys(), names - outputs.keys()
    outputs = {name: probe for name, probe in outputs.items() if name in names}
if "google_lifecycle.json" in outputs:
    inputs = json.loads((ROOT / "rust/reference-tools/google/lifecycle-inputs.json").read_text())
    original = json.loads(subprocess.check_output(
        ["git", "cat-file", "blob", inputs["source_git_blob"]], cwd=ROOT,
    ))
    note = original["cases"][0]["frames"][1]["payload"]
    assert inputs["uuid"] == re.search(r'id="message_([0-9a-f-]+)"', note)[1]
    assert inputs["timestamp_milliseconds"] == [
        int(re.search(r'data-message-timestamp="(\d+)"', note)[1]),
        int(re.search(r'data-message-updated-at="(\d+)"', note)[1]),
    ], "historical lifecycle clock inputs differ from their source artifact"
env = dict(os.environ, PARITY_IMAGE=os.environ.get("PARITY_IMAGE", "campfire-reference"), PARITY_NAMESPACE="pin-refresh-google", PARITY_OWNER="pin-refresh", PARITY_CPUS=os.environ.get("PARITY_CPUS", "1"))
for name, (script, extra) in outputs.items():
    args = [str(ROOT / "rust/parity/bin/reference"), "runner", "--seed", "default", "--time", "2026-03-02T16:00:00Z", "--freeze", "-e", "RAILS_LOG_LEVEL=fatal"]
    if extra and extra[0] == "-e":
        args.extend(extra)
        extra = []
    args += [str(ROOT / "rust/reference-tools/google" / script), *extra]
    output = options.output / name
    with output.open("wb") as stdout, (options.output / (name + ".log")).open("wb") as stderr:
        subprocess.run(args, cwd=ROOT, env=env, stdout=stdout, stderr=stderr, check=True)
    value = json.loads(output.read_bytes())
    target = ROOT / "rust/vectors" / name
    if options.write:
        target.write_bytes(output.read_bytes())
    assert output.read_bytes() == target.read_bytes(), name
    print(f"Rails Google: {name}; complete capture byte-identical", flush=True)
print(f"Rails Google: {len(outputs)} corpora replayed", flush=True)
