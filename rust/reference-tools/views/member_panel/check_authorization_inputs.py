#!/usr/bin/env python3
"""Keep recorded/test credentials separate from Authorization header syntax."""
import json
from pathlib import Path
import re
import sys
import unittest

ROOT = Path(__file__).resolve().parents[3]
SOURCES = [
    "crates/campfire/src/controllers/presenters/accounts/tests/approval_decisions.rs",
    "crates/campfire/src/controllers/presenters/accounts/tests/agent_histories.rs",
    "crates/campfire/src/controllers/presenters/accounts/tests/github_connections.rs",
    "crates/campfire/src/controllers/agent_approvals/execution_tests.rs",
    "crates/campfire/src/integrations/github/tests.rs",
]
CORPORA = ["reference-tools/views/member_panel/polling_cases.json", "vectors/member-polling.json"]
LITERAL = re.compile(r'["\']Bearer [A-Za-z0-9][A-Za-z0-9_-]+')


def literal_lines(source):
    return [i for i, line in enumerate(source.splitlines(), 1) if LITERAL.search(line)]


def recorded_headers(corpus):
    return [case["name"] for case in corpus["cases"]
            if any(key.lower() == "authorization" for key in case["headers"])]


class Discrimination(unittest.TestCase):
    def test_literal_header_is_rejected(self):
        self.assertEqual(literal_lines('Some("' + 'Bearer ' + 'fixture-token")'), [1])

    def test_composed_header_is_allowed(self):
        self.assertEqual(literal_lines('format!("Bearer {token}")'), [])

    def test_recorded_header_is_rejected_case_insensitively(self):
        for key in ["Authorization", "authorization"]:
            self.assertEqual(recorded_headers({"cases": [{"name": "token", "headers": {key: "value"}}]}), ["token"])

    def test_recorded_token_is_allowed(self):
        self.assertEqual(recorded_headers({"cases": [{"name": "token", "headers": {}, "token": "fixture"}]}), [])


if __name__ == "__main__":
    if "--self-test" in sys.argv:
        unittest.main(argv=[sys.argv[0]])
    violations = []
    for name in SOURCES:
        violations.extend(f"{name}:{line}: literal Authorization header" for line in literal_lines((ROOT / name).read_text()))
    for name in CORPORA:
        violations.extend(f"{name}: {case}: recorded Authorization header" for case in recorded_headers(json.loads((ROOT / name).read_text())))
    if violations:
        print("\n".join(violations), file=sys.stderr)
        sys.exit(1)
    print("Authorization inputs: 5 Rust test files and 2 corpora; no literal headers")
