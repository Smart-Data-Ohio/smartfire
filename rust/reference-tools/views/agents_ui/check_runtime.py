#!/usr/bin/env python3
"""Compile real regressions; require HTTP/socket tests to reject each one."""
import argparse
from pathlib import Path
from discrimination import require_baseline, require_rejected, run_tests

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
     "Ok(Some(views::uncached_message(ctx, &view)))",
     "Ok(Some(views::message(ctx, &view)))",
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
parser.add_argument("--only", action="append", choices=[case[0] for case in cases])
args = parser.parse_args()
selected = [case for case in cases if args.only is None or case[0] in args.only]
assertions = {
    "activity": ("agent_broadcasts.rs", "committed approval must broadcast its activity item"),
    "status": ("agent_broadcasts.rs", "committed agent status must broadcast both status fragments"),
    "steps": ("agent_broadcasts.rs", "committed thread step must broadcast ordered steps"),
    "message-steps": ("agent_broadcasts.rs", "committed message step must broadcast the current message"),
    "message-cache": ("agent_broadcasts.rs", "message step callback must render current uncached steps"),
    "presence-broadcast": ("agent_broadcasts.rs", "working presence incorrectly broadcast a status callback"),
    "presence-status-note": ("agent_broadcasts.rs", "committed status note must broadcast both status fragments"),
    "ledger": ("agent_histories.rs", "ledger content requires the agent's read_messages grant"),
    # Without the administrator guard, the real GitHub identity validation returns 422.
    "external-owner": ("approval_decisions.rs", "external write approvals require an administrator", ("422", "403")),
}
originals = {path: path.read_text() for _, path, *_ in selected}
for name, path, needle, *_ in selected:
    assert originals[path].count(needle) == 1, f"{name}: source changed; review mutation"
for test in dict.fromkeys(case[-1] for case in selected):
    require_baseline(test)
try:
    for name, path, needle, replacement, test in selected:
        original = originals[path]
        path.write_text(original.replace(needle, replacement))
        result = run_tests(test)
        filename, message, *values = assertions[name]
        require_rejected(result, {
            test: ("crates/campfire/src/controllers/presenters/accounts/tests/" + filename, message, *values)
        })
        print(f"{name}: rejected at {message}", flush=True)
        path.write_text(original)
    print(f"Agent runtime discrimination: {len(selected)} regressions detected; sources restored")
finally:
    for path, original in originals.items():
        path.write_text(original)
