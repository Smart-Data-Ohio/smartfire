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
    ("duration_input", "crates/campfire/src/controllers/agents.rs", "let (duration_ms, input_errors) = duration_input(fields.get(\"duration_ms\"));",
     "let (duration_ms, _input_errors) = duration_input(fields.get(\"duration_ms\")); let input_errors = campfire_db::Errors::default();", "invalid_duration_never_persists_and_parent_policy_runs_first"),
]
for name, relative, before, after, test in mutations:
    path = root / relative
    original = path.read_text()
    assert before in original, name
    try:
        path.write_text(original.replace(before, after, 1))
        result = subprocess.run(["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j", "4", "-p", "campfire", test, "--", "--nocapture"], cwd=root, env=environment, capture_output=True, text=True)
        output = result.stdout + result.stderr
        (scratch / (name + ".log")).write_text(output)
        summary = re.search(r"^test result: FAILED\..+$", output, re.M)
        assert result.returncode != 0 and summary and "assertion" in output, output[-4000:]
        print(f"WS11-api mutation {name}: {summary.group()}", flush=True)
    finally:
        path.write_text(original)
print("WS11-api mutations: 3 broken guards rejected; 3 source files restored")
