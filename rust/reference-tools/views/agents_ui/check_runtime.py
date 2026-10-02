#!/usr/bin/env python3
"""Compile real regressions; require HTTP/socket tests to reject each one."""
import argparse
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[3]
sink = root / "crates/campfire/src/channels/sink.rs"
history = root / "crates/campfire/src/controllers/presenters/agents/history.rs"
decisions = root / "crates/campfire/src/controllers/agent_approvals.rs"
rendered = root / "crates/campfire/src/controllers/messages/rendered.rs"
agent = root / "crates/db/src/models/agent.rs"
cases = [
    ("activity", sink, "cable.broadcast(&stream, &payload);",
     'if !stream.ends_with("_activity") { cable.broadcast(&stream, &payload); }',
     "approval_activity_ids_follow_committed_http_decisions_without_cross_user_leaks"),
    ("status", sink,
     "decode(request).and_then(|broadcast| agent_status(app, &broadcast))",
     "decode::<campfire_db::models::agent::AgentStatusChange>(request).map(|_| ())",
     "status_callback_replaces_badge_then_directory_over_live_socket_after_commit"),
    ("steps", sink,
     "decode(request).and_then(|broadcast| agent_steps(app, &broadcast))",
     "decode::<campfire_db::models::agent_step::StepParentChange>(request).map(|_| ())",
     "thread_step_callback_renders_ordered_steps_and_updates_over_live_socket"),
    ("message-steps", sink,
     "if let Some(id) = change.message_id {",
     "if change.message_id.is_some() { return Ok(()); }\n    if let Some(id) = change.message_id {",
     "message_step_callbacks_replace_current_message_in_room_and_thread_without_cached_tokens"),
    ("message-cache", rendered,
     "views::uncached_message(ctx, &view)",
     "views::message(ctx, &view)",
     "message_step_callbacks_replace_current_message_in_room_and_thread_without_cached_tokens"),
    ("presence-broadcast", agent,
     "if self.status != before.status || self.status_note != before.status_note {",
     "if true || self.status != before.status || self.status_note != before.status_note {",
     "working_presence_is_polled_and_does_not_emit_status_callbacks"),
    ("presence-status-note", sink,
     "decode(request).and_then(|broadcast| agent_status(app, &broadcast))",
     "decode::<campfire_db::models::agent::AgentStatusChange>(request).map(|_| ())",
     "working_presence_is_polled_and_does_not_emit_status_callbacks"),
    ("ledger", history,
     '&& agent.can(conn, "read_messages", Some(message.room_id))?',
     "&& true",
     "ledger_redacts_content_when_viewer_agent_membership_or_read_grant_is_missing"),
    ("external-owner", decisions, "&& !actor.is_administrator()",
     "&& false && !actor.is_administrator()",
     "owner_cannot_approve_github_or_fizzy_but_can_deny_each"),
]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--only", choices=[case[0] for case in cases])
args = parser.parse_args()
selected = [case for case in cases if args.only is None or case[0] == args.only]
originals = {path: path.read_text() for _, path, *_ in selected}
try:
    for name, path, needle, replacement, test in selected:
        original = originals[path]
        assert original.count(needle) == 1, f"{name}: source changed; review mutation"
        path.write_text(original.replace(needle, replacement))
        result = subprocess.run(
            ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked",
             "-p", "campfire", test, "--", "--nocapture", "--test-threads=8"],
            cwd=root, env={**os.environ, "CI": "1", "CARGO_BUILD_JOBS": "2"}, capture_output=True, text=True)
        output = result.stdout + result.stderr
        assert result.returncode != 0 and "test result: FAILED." in output and "panicked at" in output, f"{name}: mutation escaped test\n{output}"
        print(name + ": " + next(line for line in output.splitlines() if line.startswith("test result:")), flush=True)
        path.write_text(original)
    print(f"Agent runtime discrimination: {len(selected)} regressions detected; sources restored")
finally:
    for path, original in originals.items():
        path.write_text(original)
