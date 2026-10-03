#!/usr/bin/env python3
"""Exercise named-case discovery, including the committed generated assignment tests."""
import importlib.util
from pathlib import Path
import unittest

root = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location('ports', Path(__file__).with_name('check-case-ports.py'))
ports = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ports)

class Discovery(unittest.TestCase):
    def test_assignment_macro_and_missing_invocation(self):
        source = (root/'rust/crates/db/src/tests/agent_assignment_cases_test.rs').read_text()
        import json
        manifest = json.loads(Path(__file__).with_name('case-ports.json').read_text())
        expected = {c['rust'] for g in manifest['files'] if g['rust_file'].endswith('agent_assignment_cases_test.rs') for c in g['cases']}
        self.assertEqual(len(expected), 9)
        self.assertTrue(expected <= ports.rust_ports(source))
        self.assertFalse(expected <= ports.rust_ports(source.replace('named_cases!', 'unsupported_cases!')))

    def test_slash_macro_and_missing_invocation(self):
        source = (root/'rust/crates/campfire/src/controllers/message_features/slash_named_tests.rs').read_text()
        import json
        manifest = json.loads(Path(__file__).with_name('case-ports.json').read_text())
        expected = {c['rust'] for g in manifest['files'] if g['rust_file'].endswith('slash_named_tests.rs') for c in g['cases']}
        self.assertEqual(len(expected), 34)
        self.assertTrue(expected <= ports.rust_ports(source))
        self.assertFalse(expected <= ports.rust_ports(source.replace('named!', 'unsupported_cases!')))
        self.assertNotIn('unmapped_probe', ports.rust_ports('named!(unmapped_probe, -1);'))

    def test_literal_async_and_peer_names(self):
        source = '\n#[test]\nfn ws12_fixture() {}\n#[tokio::test]\nasync fn ws8_fixture() {}\n'
        self.assertEqual(ports.rust_ports(source), {'ws12_fixture', 'ws8_fixture'})
        self.assertNotIn('missing_test', ports.rust_ports(source))

if __name__ == '__main__':
    unittest.main()
