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


class DeferredSystemInventoryTest(unittest.TestCase):
    def fixture(self, case):
        cases = [case] + [
            {'name': f'proved case {i}', 'status': 'passed', 'evidence': 'paired proof'}
            for i in range(134)
        ]
        source = ''.join(f'  test "{item["name"]}" do\n  end\n' for item in cases)
        inventory = {'files': [{
            'file': 'test/system/inventory_fixture_test.rb',
            'source_sha256': hashlib.sha256(source.encode()).hexdigest(),
            'cases': cases,
        }]}
        return inventory, source

    def verify(self, case):
        inventory, source = self.fixture(copy.deepcopy(case))
        output = io.StringIO()
        with patch.object(Path, 'read_text', return_value=json.dumps(inventory)), \
                patch('subprocess.check_output', return_value=source) as read_pin, \
                contextlib.redirect_stdout(output):
            runpy.run_path(str(SCRIPT), run_name='__main__')
        self.assertEqual(read_pin.call_args.args[0], [
            'git', 'show', 'd7c7de92:test/system/inventory_fixture_test.rb',
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

    def causal_pass(self):
        return {
            'name': 'a release click landing on the just-opened menu does not activate it',
            'status': 'passed', 'evidence': 'fresh paired controls and intended negatives',
            'closure_note': 'Native pinned Selenium window geometry replaces the viewport translation.',
            'closure_proof': {
                'independent_fixtures': True, 'automatic_retries': 0,
                'positive_runs': {app: {'passed': 10, 'failed': 0} for app in ('Rails', 'Rust')},
                'negative': {app: 'intended rejection' for app in ('Rails', 'Rust')},
            },
        }

    def test_causal_pass_requires_more_than_an_unchanged_path_receipt(self):
        case = self.causal_pass()
        self.assertIn('135 mapped behaviour passes; 0 deferred', self.verify(case))
        for field in ('closure_note', 'closure_proof'):
            wrong = copy.deepcopy(case)
            del wrong[field]
            with self.subTest(field=field), self.assertRaisesRegex(AssertionError, 'causal'):
                self.verify(wrong)

    def test_causal_pass_rejects_failed_retried_or_unpaired_proofs(self):
        for property, value in [('Rust passes', 9), ('Rust failure', 1), ('retry', 1), ('invalid negative', 'invalid')]:
            wrong = self.causal_pass()
            proof = wrong['closure_proof']
            if property == 'Rust passes':
                proof['positive_runs']['Rust']['passed'] = value
            elif property == 'Rust failure':
                proof['positive_runs']['Rust']['failed'] = value
            elif property == 'retry':
                proof['automatic_retries'] = value
            else:
                proof['negative']['Rails'] = value
            with self.subTest(property=property), self.assertRaisesRegex(AssertionError, 'causal'):
                self.verify(wrong)

    def test_disputed_closures_have_causal_notes_and_new_evidence(self):
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
                self.assertEqual(case['status'], 'passed')
                self.assertTrue(case['evidence'])
                self.assertNotIn('previous_evidence', case)
                self.assertNotIn('remaining_reason', case)
                self.assertTrue(case['closure_note'].strip())
                self.assertGreaterEqual(case['closure_proof']['positive_runs']['Rust']['passed'], 10)


if __name__ == '__main__':
    unittest.main()
