#!/usr/bin/env python3
"""Prove WS7's real-socket specs reject broken checks, restoring each file in finally."""
import os
from pathlib import Path
import signal
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
CHANNELS = "crates/campfire/src/channels/"
MATRIX = "authorization_test::authorization_matrix"
MUTATIONS = [
    ("two-factor gate", CHANNELS + "connection.rs", "if requires_two_factor(&user)", "if false && requires_two_factor(&user)", MATRIX),
    ("expired session", CHANNELS + "connection.rs", "if session_expired(&session,", "if false && session_expired(&session,", MATRIX),
    ("non-member", "crates/db/src/models/room.rs", '"memberships"."user_id" = ?"#;', '("memberships"."user_id" = ? OR 1)"#;', MATRIX),
    ("bot", "crates/campfire/src/channels.rs", "self.role == campfire_db::Role::Bot", "false", MATRIX),
    ("banned", "crates/db/src/models/user.rs", "ban", None, MATRIX),
    ("deactivated", "crates/db/src/models/user.rs", "deactivate", None, MATRIX),
    ("inactive human", "crates/campfire/src/channels.rs", "self.status == campfire_db::Status::Active && !self.bot()", "!self.bot()", "reference_test::activity_rejects_inactive_users"),
    ("typing thread parent", CHANNELS + "typing_notifications.rs", "found.then_some(Conversation::Thread(thread_id))", "Some(Conversation::Thread(thread_id)).filter(|_| found || true)", "reference_test::typing_a_thread"),
    ("typing membership recheck", CHANNELS + "typing_notifications.rs", "if Room::find_for_user(conn, user_id, room_id)?.is_none()", "if false && Room::find_for_user(conn, user_id, room_id)?.is_none()", "reference_test::typing_revoked"),
    ("thread suffix guard", CHANNELS + "room_messages.rs", '[STREAM_SUFFIX, "threads"]', '[STREAM_SUFFIX, STREAM_SUFFIX]', "reference_test::thread_messages_the_stock"),
    ("workspace rejection", CHANNELS + "workspace_presence.rs", "        if sub.rejected() {\n            return Ok(false);\n        }\n", "", "reference_test::workspace_presence_heartbeat_rejects"),
    ("workspace idle expiry", CHANNELS + "workspace_presence.rs", "expire_idle_timed_out_session(tx, session_id, timeout)?;", "let _ = (session_id, timeout);", "reference_test::workspace_presence_heartbeat_destroys"),
    ("sign-out disconnect", "crates/campfire/src/concerns.rs", "            user.reset_remote_connections(tx);", "            let _ = user;", "hub_test::signing_out"),
    ("golden frame difference", CHANNELS + "tests/golden.rs", '"status_badge_agent_1", "<span>ready</span>"', '"wrong_target", "<span>ready</span>"', "golden::replays_reference_frames"),
]


def mutate(text, old, new):
    if new is None:
        start = text.index(f"    pub fn {old}(")
        end = text.find("    pub fn ", start + 1)
        end = len(text) if end == -1 else end
        part = text[start:end]
        needle = 'r#"DELETE FROM "sessions" WHERE "sessions"."user_id" = ?"#'
        assert part.count(needle) == 1
        return text[:start] + part.replace(needle, 'r#"DELETE FROM "sessions" WHERE 0 AND "sessions"."user_id" = ?"#') + text[end:]
    if text.count(old) == 1:
        return text.replace(old, new)
    pattern = r"\s+".join(re.escape(part) for part in old.split())
    assert len(re.findall(pattern, text)) == 1, f"mutation target must occur once: {old}"
    return re.sub(pattern, lambda match: new, text)


def main():
    scratch = ROOT.parent / ".scratch/evidence/mutations"
    scratch.mkdir(parents=True, exist_ok=True)
    for name, file, old, new, test in MUTATIONS:
        path = ROOT / file
        original = path.read_text()
        changed = mutate(original, old, new)
        path.write_text(changed)
        try:
            command = ["cargo", "test", "-j", "4", "-p", "campfire", f"channels::tests::{test}"]
            print(f"mutation: {name}\n$ {' '.join(command)}", flush=True)
            run = subprocess.Popen(command, cwd=ROOT, env=os.environ.copy(), stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
            try:
                stdout, stderr = run.communicate(timeout=120)
            except subprocess.TimeoutExpired:
                os.killpg(run.pid, signal.SIGTERM)
                run.communicate()
                raise
            output = stdout + stderr
            (scratch / (name.replace(" ", "-") + ".log")).write_text(output)
            summaries = [line for line in output.splitlines() if line.startswith("test result:")]
            print("\n".join(summaries), flush=True)
            assert run.returncode != 0 and any("FAILED." in line for line in summaries), f"mutation not caught by a failing test: {name}\n{output}"
        finally:
            assert path.read_text() == changed, f"another writer changed {file}; refusing to overwrite it"
            path.write_text(original)
    print(f"mutation checks: {len(MUTATIONS)} caught; 0 survived", flush=True)


if __name__ == "__main__":
    main()
