#!/usr/bin/env python3
"""Prove the HTTP directory test rejects Rust's contextual final-sigma ordering."""
from pathlib import Path
from discrimination import require_baseline, require_rejected, run_tests

root = Path(__file__).resolve().parents[3]
source = root / "crates/db/src/models/agent.rs"
original = source.read_text()
assert original.count("unicode::downcase(&user.name)") == 1
require_baseline('agent_directory_lists_active_then_inactive_without_private_facts')
try:
    source.write_text(original.replace("unicode::downcase(&user.name)", "user.name.to_lowercase()"))
    result = run_tests('agent_directory_lists_active_then_inactive_without_private_facts')
    require_rejected(result, {'agent_directory_lists_active_then_inactive_without_private_facts': ('crates/campfire/src/controllers/presenters/accounts/tests.rs', "Ruby lowercases Σ without Rust's final-sigma context rule")})
    print('Directory downcase injection: rejected (Greek final-sigma order differed); source restored')
finally:
    source.write_text(original)
