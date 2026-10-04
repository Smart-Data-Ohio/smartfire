#!/usr/bin/env python3
"""Regenerate explicitly owned domain goldens from independent copies of the current Rails seed."""
import argparse
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]

OUTPUTS = {'agents_account_removal_contract.json': ('account_removal_contract.rb', []), 'agents_agent_record_contract.json': ('agent_record_contract.rb', []), 'agents_approval_contract.json': ('approval_contract.rb', []), 'agents_approval_neighbour_contract.json': ('approval_neighbour_contract.rb', []), 'agents_approvals_service_contract.json': ('approvals_service_contract.rb', []), 'agents_assignment_named.json': ('assignment_named_cases.rb', []), 'agents_bot_contract.json': ('bot_contract.rb', []), 'agents_bot_gaps_contract.json': ('bot_gaps_contract.rb', []), 'agents_bot_plaintext_named_cases.json': ('bot_plaintext_named_cases.rb', []), 'agents_cleanup_contract.json': ('cleanup_contract.rb', []), 'agents_context_contract.json': ('context_contract.rb', []), 'agents_deletion_callback_contract.json': ('deletion_callback_contract.rb', []), 'agents_deletion_indicator_contract.json': ('deletion_indicator_contract.rb', []), 'agents_deletion_publication_contract.json': ('deletion_publication_contract.rb', []), 'agents_delivery_contract.json': ('delivery_contract.rb', []), 'agents_direct_messages_contract.json': ('direct_messages_contract.rb', []), 'agents_event_access_contract.json': ('event_access_contract.rb', []), 'agents_event_callback_failure_contract.json': ('event_callback_failure_contract.rb', []), 'agents_event_payload_contract.json': ('event_payload_contract.rb', []), 'agents_event_polling_contract.json': ('event_polling_contract.rb', []), 'agents_finalization_failure_contract.json': ('finalization_failure_contract.rb', []), 'agents_github_approval_inbox_http_contract.json': ('github_approval_inbox_http_contract.rb', []), 'agents_github_stream_peer_contract.json': ('github_stream_peer_contract.rb', []), 'agents_grant_credential_contract.json': ('grant_credential_contract.rb', []), 'agents_lifecycle_contract.json': ('lifecycle_contract.rb', []), 'agents_message_presenter_contract.json': ('message_presenter_contract.rb', []), 'agents_next_named.json': ('next_named_cases.rb', []), 'agents_posting_budget_contract.json': ('posting_budget_contract.rb', []), 'agents_presence_slash_named_cases.json': ('presence_slash_named_cases.rb', []), 'agents_private_guard_cases.json': ('private_guard_cases.rb', []), 'agents_repository_adapter_contract.json': ('repository_adapter_contract.rb', []), 'agents_review_fixes_contract.json': ('review_fixes_contract.rb', []), 'agents_security_lifecycle_cases.json': ('security_lifecycle_cases.rb', []), 'agents_slash_command_contract.json': ('slash_command_contract.rb', []), 'agents_step_contract.json': ('step_contract.rb', []), 'agents_stream_frames_contract.json': ('stream_frames_contract.rb', []), 'agents_stream_resume_contract.json': ('stream_resume_contract.rb', []), 'agents_stream_trailing_contract.json': ('stream_trailing_contract.rb', []), 'agents_streaming_contract.json': ('streaming_contract.rb', []), 'agents_ui_owner_inputs_contract.json': ('ui_owner_inputs_contract.rb', []), 'agents_user_removal_contract.json': ('user_removal_contract.rb', []), 'agents_webhook_contract.json': ('webhook_contract.rb', []), 'agents_webhook_paths_contract.json': ('webhook_paths_contract.rb', []), 'agents_work_events_contract.json': ('work_events_contract.rb', []), 'agents_work_payload_named_cases.json': ('work_payload_named_cases.rb', []), 'agents_work_services_contract.json': ('work_services_contract.rb', []), 'agents_working_presence_contract.json': ('working_presence_contract.rb', []), 'agents_stream_configured_frames_contract.json': ('stream_frames_contract.rb', ['-e', 'APP_URL=https://campfire.example.test:8443']), 'agent_next6_named.json': ('next6_named.rb', []), 'agent_next6_queries.json': ('next6_named.rb', ['next6-query-inputs.json']), 'agent_recorder_cost.json': ('next5_recorder.rb', []), 'agent_work_named_http.json': ('next5_work_named.rb', []), 'agent_pr227_deleted_work.json': ('pr227_deleted_work.rb', [])}

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("output", type=Path)
parser.add_argument("--write", action="store_true")
parser.add_argument("--names", default="")
options = parser.parse_args()
options.output = options.output.resolve()
options.output.mkdir(parents=True, exist_ok=True)
subprocess.run(["python3", str(ROOT / "rust/reference-tools/agents/refresh-input-pins.py")], cwd=ROOT, check=True)
outputs = OUTPUTS
if options.names:
    names = set(options.names.split(","))
    assert names <= outputs.keys(), names - outputs.keys()
    outputs = {name: probe for name, probe in outputs.items() if name in names}
env = dict(os.environ, PARITY_IMAGE=os.environ.get("PARITY_IMAGE", "campfire-reference"), PARITY_NAMESPACE="pin-refresh-agents", PARITY_OWNER="pin-refresh", PARITY_CPUS=os.environ.get("PARITY_CPUS", "1"))
for name, (script, extra) in outputs.items():
    args = [str(ROOT / "rust/parity/bin/reference"), "runner", "--seed", "first_run" if script == "event_callback_failure_contract.rb" else "default", "-e", "RAILS_LOG_LEVEL=fatal"]
    if extra and extra[0] == "-e":
        args.extend(extra)
        extra = []
    args += [str(ROOT / "rust/reference-tools/agents" / script), *extra]
    output = options.output / name
    with output.open("wb") as stdout, (options.output / (name + ".log")).open("wb") as stderr:
        subprocess.run(args, cwd=ROOT, env=env, stdout=stdout, stderr=stderr, check=True)
    value = json.loads(output.read_bytes())
    target = ROOT / "rust/vectors" / name
    if options.write:
        target.write_bytes(output.read_bytes())
    assert output.read_bytes() == target.read_bytes(), name
    print(f"Rails agent domain: {name}; complete capture byte-identical", flush=True)
print(f"Rails agent domain: {len(outputs)} corpora replayed", flush=True)
