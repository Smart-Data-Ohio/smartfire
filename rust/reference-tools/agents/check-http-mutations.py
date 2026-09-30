#!/usr/bin/env python3
"""Prove HTTP parity checks reject broken guards; always restore each source file."""
import os
from pathlib import Path
import re
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root.parent / ".scratch" / "http-mutations"
scratch.mkdir(parents=True, exist_ok=True)
assert (root / "parity/.seed/default/db/production.sqlite3").is_file(), "build the default seed first"
environment = dict(os.environ, CI="1", TMPDIR=str(scratch))
mutations = [
    ("human_token", "crates/campfire/src/concerns.rs", "if authenticated_by(c) == AuthenticatedBy::AgentToken {",
     "if false && authenticated_by(c) == AuthenticatedBy::AgentToken {", "agent_token_cannot_access_human_endpoint"),
    ("mirror_headers", "crates/campfire/src/controllers/agents/mcp.rs", "if modern && let Some(message) = header_mismatch(c, method, &params) {",
     "if false && let Some(message) = header_mismatch(c, method, &params) {", "mcp_version_and_header_security"),
    ("revoked_credential", "crates/db/src/models/agent_credential.rs", "token_digest=? AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at>?)", "token_digest=? AND (expires_at IS NULL OR expires_at>?)", "revoked_and_expired_credentials"),
    ("expired_credential", "crates/db/src/models/agent_credential.rs", "(expires_at IS NULL OR expires_at>?) LIMIT 1", "(expires_at IS NULL OR expires_at<=?) LIMIT 1", "revoked_and_expired_credentials"),
    ("missing_grant", "crates/campfire/src/controllers/agents.rs", "if !allowed {", "if !allowed && false {", "grant_denial_is_403_and_nonmembership_is_404"),
    ("nonmember_hidden", "crates/db/src/models/room.rs", 'let sql = format!(r#"{SELECT_FOR_USER} AND "rooms"."id" = ? LIMIT 1"#);', 'let sql = String::from("SELECT * FROM rooms WHERE id=?2 AND ?1 IS NOT NULL AND deleted_at IS NULL LIMIT 1");', "bot_http_nonmember_is_hidden"),
    ("reply_create_only", "crates/campfire/src/controllers/messages/by_bots.rs", "if concerns::authenticated_by(c) == concerns::AuthenticatedBy::BotReply {", "if false && concerns::authenticated_by(c) == concerns::AuthenticatedBy::BotReply {", "bot_http_reply_"),
    ("rate_overflow", "crates/campfire/src/concerns/agent_api.rs", "(count > limit).then_some", "(count > limit && false).then_some", "credential_rate_overflow"),
    ("budget_overflow", "crates/db/src/models/agent_posting.rs", "if usage < limit {", "if usage < limit || limit >= 0 {", "bot_http_budget_overflow"),
    ("mcp_version", "crates/campfire/src/controllers/agents/mcp.rs", 'if !METADATA["versions"]', 'if false && !METADATA["versions"]', "mcp_version_and_header_security"),
    ("duration_input", "crates/campfire/src/controllers/agents.rs", "let (duration_ms, input_errors) = duration_input(fields.get(\"duration_ms\"));",
     "let (duration_ms, _input_errors) = duration_input(fields.get(\"duration_ms\")); let input_errors = campfire_db::Errors::default();", "invalid_duration_never_persists_and_parent_policy_runs_first"),
]
for name, relative, before, after, test in mutations:
    path = root / relative
    original = path.read_text()
    assert before in original, name
    try:
        path.write_text(original.replace(before, after, 1))
        for attempt in range(3):
            result = subprocess.run(["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j", "4", "-p", "campfire", test, "--", "--nocapture"], cwd=root, env=environment, capture_output=True, text=True)
            output = result.stdout + result.stderr
            if "SIGKILL" not in output or "test result:" in output:
                break
            (scratch / f"{name}-compiler-killed-{attempt}.log").write_text(output)
            print(f"WS11-api mutation {name}: compiler killed; retry {attempt + 1}/3", flush=True)
        (scratch / (name + ".log")).write_text(output)
        summary = re.search(r"^test result: FAILED\..+$", output, re.M)
        assert result.returncode != 0 and summary and "assertion" in output, output[-4000:]
        print(f"WS11-api mutation {name}: {summary.group()}", flush=True)
    finally:
        path.write_text(original)
print(f"WS11-api mutations: {len(mutations)} broken guards rejected; all source files restored")
