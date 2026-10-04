"""Exercise the inventory gate without running either application's browser suite."""
import contextlib
import copy
import hashlib
import io
import json
from pathlib import Path
import runpy
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).with_name('deferred-system-inventory.py')
PIN = (SCRIPT.resolve().parents[3] / 'rust/parity/reference.sha').read_text().strip()


class DeferredSystemInventoryTest(unittest.TestCase):
    def fixture(self, case):
        cases = [case] + [
            {'name': f'proved case {i}', 'status': 'passed', 'evidence': 'paired proof'}
            for i in range(134)
        ]
        source = ''.join(f'  test "{item["name"]}" do\n  end\n' for item in cases)
        inventory = {'reference': PIN, 'files': [{
            'file': 'test/system/inventory_fixture_test.rb',
            'source_sha256': hashlib.sha256(source.encode()).hexdigest(),
            'cases': cases,
        }]}
        return inventory, source

    def verify(self, case):
        inventory, source = self.fixture(copy.deepcopy(case))
        output = io.StringIO()
        def read_text(path, *args, **kwargs):
            return PIN if path.name == 'reference.sha' else json.dumps(inventory)
        with patch.object(Path, 'read_text', read_text), \
                patch('subprocess.check_output', return_value=source) as read_pin, \
                contextlib.redirect_stdout(output):
            runpy.run_path(str(SCRIPT), run_name='__main__')
        self.assertEqual(read_pin.call_args.args[0], [
            'git', 'show', f'{PIN}:test/system/inventory_fixture_test.rb',
        ])
        return output.getvalue()

    def deferred(self):
        return {
            'name': 'unresolved release-hit setup', 'status': 'deferred',
            'remaining_reason': 'The earlier hit-test failure has no causal resolution.',
        }

    def test_deferral_without_evidence_emits_inventory(self):
        self.assertIn(
            'WS8bm system inventory: 135 named declarations; 134 mapped behaviour passes; '
            '1 deferred; 0 WS12 blocked; no pixel checks',
            self.verify(self.deferred()),
        )

    def test_null_evidence_is_not_a_closure_claim(self):
        self.assertIn('1 deferred; 0 WS12 blocked', self.verify({**self.deferred(), 'evidence': None}))

    def test_deferral_cannot_claim_a_pass(self):
        with self.assertRaisesRegex(AssertionError, 'closure evidence'):
            self.verify({**self.deferred(), 'evidence': 'paired Rails/Rust PASS'})

    def test_deferral_requires_a_concrete_reason(self):
        for reason in (None, '', '  ', []):
            with self.subTest(reason=reason), self.assertRaisesRegex(AssertionError, 'remaining_reason'):
                self.verify({**self.deferred(), 'remaining_reason': reason, 'evidence': None})

    def test_pass_requires_closure_evidence(self):
        for evidence in (None, ''):
            with self.subTest(evidence=evidence), self.assertRaisesRegex(AssertionError, 'closure evidence'):
                self.verify({'name': 'unsupported pass', 'status': 'passed', 'evidence': evidence})

    def test_blocked_case_without_evidence_keeps_its_own_count(self):
        self.assertIn('0 deferred; 1 WS12 blocked', self.verify({
            **self.deferred(), 'status': 'blocked_ws12',
        }))

    def test_disputed_closures_remain_reason_only_deferrals(self):
        root = SCRIPT.resolve().parents[3]
        inventory = json.loads((root / 'rust/plans/ws8bm-system-cases.json').read_text())
        names = {
            'a release click landing on the just-opened menu does not activate it',
            'mobile drawer keeps the room list scroll position across close and reopen',
            'mobile drawer reopens on the current room when it is already in view',
        }
        cases = {case['name']: case for row in inventory['files'] for case in row['cases']}
        for name in names:
            with self.subTest(name=name):
                case = cases[name]
                self.assertEqual(case['status'], 'deferred')
                self.assertNotIn('evidence', case)
                self.assertNotIn('previous_evidence', case)
                self.assertTrue(case['remaining_reason'].strip())


if __name__ == '__main__':
    unittest.main()
