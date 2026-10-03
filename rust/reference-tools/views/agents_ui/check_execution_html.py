#!/usr/bin/env python3
"""Prove the real HTML-to-external execution replay rejects a JSON-only controller."""
from pathlib import Path
from discrimination import require_baseline, require_rejected, run_tests

root=Path(__file__).resolve().parents[3]
source=root/'crates/campfire/src/controllers/agent_approvals.rs'
original=source.read_text()
needle='ApprovalDecision::Applied => match c.respond_to(&[&format::HTML, &format::JSON])?'
assert original.count(needle)==1
tests = [
    'ws11ui_human_github_decision_executes_real_transport_and_rechecks_later_changes',
    'ws11ui_human_fizzy_decision_executes_real_transport_and_rechecks_later_changes',
]
require_baseline('ws11ui_human_', count=2)
try:
    source.write_text(original.replace(needle,'ApprovalDecision::Applied => match c.respond_to(&[&format::JSON])?'))
    result=run_tests('ws11ui_human_')
    require_rejected(result, {
        test: ('crates/campfire/src/controllers/agent_approvals/execution_tests.rs',
               'execution decision must use the negotiated HTML/JSON format')
        for test in tests
    })
    print('HTML execution discrimination: both real controller/job/transport replays reject a JSON-only controller at the negotiated response assertion; source restored')
finally:
    source.write_text(original)
