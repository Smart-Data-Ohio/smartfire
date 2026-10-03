#!/usr/bin/env python3
"""Prove the directory's token and human-only policy is enforced by HTTP tests."""
from pathlib import Path
from discrimination import require_baseline, require_rejected, run_tests

root = Path(__file__).resolve().parents[3]
source = root / "crates/campfire/src/controllers/agents/directory.rs"
original = source.read_text()
assert original.count("Before::default()") == 1
assert original.count("if require_current_user(c)?.is_bot()") == 1
require_baseline('agent_directory_', count=3)
try:
    source.write_text(original.replace("Before::default()", "Before::default().allow_agent_access().allow_bot_access()")
                     .replace("if require_current_user(c)?.is_bot()", "if false && require_current_user(c)?.is_bot()"))
    result = run_tests('agent_directory_')
    require_rejected(result, {
        'agent_directory_rejects_credentials_bots_and_unsigned_visitors':
            ('crates/campfire/src/controllers/presenters/accounts/tests.rs', 'directory credentials must be forbidden'),
        'agent_directory_bot_session_is_forbidden':
            ('crates/campfire/src/controllers/presenters/accounts/tests.rs', 'directory bot sessions must be forbidden'),
    }, passed=1)
    print('Directory access injection: rejected at both credential and bot-session assertions; source restored')
finally:
    source.write_text(original)
