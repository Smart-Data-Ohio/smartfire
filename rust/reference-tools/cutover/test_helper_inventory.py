#!/usr/bin/env python3
"""Focused source-inventory and real assertion-map checker regressions."""
import copy
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest

from helper_inventory import HelperInventory, completeness_errors, from_pin

ROOT = pathlib.Path(__file__).resolve().parents[3]
CHECKER = pathlib.Path(__file__).with_name('assertion-maps.py')
MAP = ROOT / 'rust/plans/ledger-ws14-ws15-d-assertions.json'


class SourceInventoryTests(unittest.TestCase):
    def test_direct_root_and_inherited_callback_assertions_are_required(self):
        source = '''class ParentTest < ActiveSupport::TestCase
  setup do
    assert parent_ready?
  end
  teardown do
    refute parent_leaked?
  end
end
class ExampleTest < ParentTest
  setup do
    assert_equal 1, ready_count
  end
  teardown do
    assert_empty pending
  end
  def setup
    assert method_ready?
  end
  def teardown
    refute method_leaked?
  end
  test "callback bodies" do
    assert true
  end
end
'''
        inventory = HelperInventory({'test/example_test.rb': source})
        required = inventory.required('test/example_test.rb', 22)
        self.assertEqual([(r['line'], r['helper']) for r in required],
                         [(3, 'setup'), (6, 'teardown'), (11, 'setup'),
                          (14, 'teardown'), (17, 'setup'), (20, 'teardown')])
        self.assertEqual(inventory.declarations['test/example_test.rb:22']['assertions'], [23])
        record = {'id': 'callbacks', 'helper_assertions': [{'rails': r} for r in required]}
        self.assertEqual(completeness_errors(record, required), [])
        for omitted in (3, 11):
            with self.subTest(omitted=omitted):
                incomplete = copy.deepcopy(record)
                incomplete['helper_assertions'] = [a for a in incomplete['helper_assertions']
                                                    if a['rails']['line'] != omitted]
                self.assertEqual(completeness_errors(incomplete, required),
                                 [f'callbacks: missing helper assertions: test/example_test.rb:{omitted}'])

    def test_indirect_helper_call_and_setup_callback_are_required(self):
        source = '''module SharedHelpers
  def authenticate
    issue_session
  end
  def issue_session
    assert cookie.present?
    authenticate # recursive calls must terminate
  end
end
class ExampleTest < ActiveSupport::TestCase
  include SharedHelpers
  setup :authenticate
  test "indirect" do
    assert_equal 1, 1
  end
end
'''
        inventory = HelperInventory({'test/example_test.rb': source})
        required = inventory.required('test/example_test.rb', 13)
        self.assertEqual(required, [{'file': 'test/example_test.rb', 'line': 6,
                                     'text': 'assert cookie.present?', 'helper': 'issue_session'}])
        record = {'id': 'indirect', 'helper_assertions': [{'rails': required[0]}]}
        self.assertEqual(completeness_errors(record, required), [])
        record['helper_assertions'].clear()
        self.assertEqual(completeness_errors(record, required),
                         ['indirect: missing helper assertions: test/example_test.rb:6'])

    def test_dispatch_uses_overrides_and_ignores_strings_comments_and_other_receivers(self):
        source = '''module SessionHelper
  def sign_in
    assert session_cookie
  end
end
module BrowserHelper
  def sign_in
    assert_selector "room"
  end
end
class ActiveSupport::TestCase
  include SessionHelper
end
class ApplicationSystemTestCase < ActionDispatch::SystemTestCase
  include BrowserHelper
end
class BrowserTest < ApplicationSystemTestCase
  setup do
    sign_in
  end
  test "browser" do
    "assert session_cookie; sign_in"
    # sign_in; assert session_cookie
    object.sign_in
    assert true
  end
end
'''
        inventory = HelperInventory({'test/example_test.rb': source})
        self.assertEqual([r['line'] for r in inventory.required('test/example_test.rb', 21)], [8])
        self.assertEqual(inventory.declarations['test/example_test.rb:21']['assertions'], [25])

    def test_local_helpers_and_self_calls_take_precedence(self):
        source = '''module SharedHelper
  def build_approval
    refute invalid?
  end
end
class ExampleTest < ActiveSupport::TestCase
  include SharedHelper
  test "approval" do
    self.build_approval
  end
  def build_approval
    action.expects(:save)
    assert action.valid?
  end
end
'''
        inventory = HelperInventory({'test/example_test.rb': source})
        self.assertEqual([r['line'] for r in inventory.required('test/example_test.rb', 8)], [12, 13])


class PinnedMapTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.mapping = json.loads(MAP.read_text())
        cls.ledger = {r['id']: r for r in json.loads((ROOT / 'rust/plans/ledger-ws14-ws15.json').read_text())['records']}
        cls.pin = (ROOT / 'rust/parity/reference.sha').read_text().strip()
        cls.inventory = from_pin(ROOT, cls.pin, [cls.ledger[r['id']] for r in cls.mapping['records']])

    def check_map(self, mapping):
        with tempfile.TemporaryDirectory() as temp:
            path = pathlib.Path(temp) / 'renamed-map.json'
            path.write_text(json.dumps(mapping))
            return subprocess.run([sys.executable, str(CHECKER), str(path)], cwd=ROOT,
                                  text=True, capture_output=True)

    def test_pinned_inventory_counts_all_declarations_and_assertions(self):
        required = direct = 0
        for record in self.mapping['records']:
            original = self.ledger[record['id']]
            required += len(self.inventory.required(original['rails'], original['rails_line']))
            direct += len(self.inventory.declarations[f"{original['rails']}:{original['rails_line']}"]['assertions'])
        self.assertEqual((len(self.mapping['records']), direct, required), (127, 547, 142))

    def test_completed_current_maps_pass(self):
        result = self.check_map(self.mapping)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('142 explicit nested helper assertion calls; 0 unmapped', result.stdout)

    def test_deleting_session_sudo_or_build_approval_mapping_is_rejected(self):
        cases = [('WS14e-102', 'test/test_helpers/session_test_helper.rb', 9),
                 ('WS14g-023', 'test/test_helpers/session_test_helper.rb', 16),
                 ('WS15g-058', 'test/jobs/github/perform_agent_action_job_test.rb', 515)]
        for rid, file, line in cases:
            with self.subTest(record=rid, helper=f'{file}:{line}'):
                mapping = copy.deepcopy(self.mapping)
                record = next(r for r in mapping['records'] if r['id'] == rid)
                original_count = len(record.get('helper_assertions', []))
                record['helper_assertions'] = [a for a in record.get('helper_assertions', [])
                                               if (a['rails']['file'], a['rails']['line']) != (file, line)]
                self.assertEqual(len(record['helper_assertions']), original_count - 1,
                                 'Run this regression against the completed current maps')
                result = self.check_map(mapping)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(f'{rid}: missing helper assertions: {file}:{line}', result.stderr)
                self.assertIn('142 required; 141 mapped', result.stderr)

    def test_missing_policy_and_changed_pin_are_rejected_for_renamed_maps(self):
        for altered_pin in (False, True):
            with self.subTest(altered_pin=altered_pin):
                mapping = copy.deepcopy(self.mapping)
                mapping.pop('helper_inventory_version', None)
                if altered_pin:
                    mapping['reference'] = 'd7c7de92'
                result = self.check_map(mapping)
                self.assertNotEqual(result.returncode, 0)
                expected = 'unregistered historical' if altered_pin else 'require helper_inventory_version: 1'
                self.assertIn(expected, result.stderr)

    def test_helper_physical_citations_are_checked(self):
        for kind in ('ruby_quote', 'rust_quote', 'rust_range', 'unexpected_helper'):
            with self.subTest(kind=kind):
                mapping = copy.deepcopy(self.mapping)
                record = next(r for r in mapping['records'] if r['id'] == 'WS14e-102')
                entry = record['helper_assertions'][0]
                if kind == 'ruby_quote':
                    entry['rails']['text'] += ' # fabricated citation'
                    expected = 'fabricated citation'
                elif kind == 'rust_quote':
                    entry['rust'][0]['text'] = 'assert!(false); // fabricated citation'
                    expected = 'fabricated citation'
                elif kind == 'rust_range':
                    entry['rust'][0]['end_line'] = entry['rust'][0]['line'] - 1
                    expected = 'invalid physical Rust citation range'
                else:
                    extra = copy.deepcopy(entry)
                    extra['rails']['line'] = 16
                    extra['rails']['text'] = 'assert_response :redirect'
                    record['helper_assertions'].append(extra)
                    expected = 'WS14e-102: unexpected helper assertions: test/test_helpers/session_test_helper.rb:16'
                result = self.check_map(mapping)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(expected, result.stderr)

    def test_historical_b_c_maps_keep_explicit_declaration_only_semantics(self):
        paths = [ROOT / f'rust/plans/ledger-ws14-ws15-{name}-assertions.json' for name in ('b', 'c')]
        result = subprocess.run([sys.executable, str(CHECKER), *map(str, paths)], cwd=ROOT,
                                text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(result.stdout.count('historical declaration-only map; helper completeness is not claimed'), 2)
        historical = json.loads(paths[0].read_text())
        superseded = next(r for r in historical['records'] if r['id'] == 'WS14e-101')
        self.assertEqual(superseded['review_disposition'], 'reopened')
        self.assertEqual(superseded['superseded_by'], 'rust/plans/ledger-ws14-ws15-d-assertions.json')
        self.assertEqual(sum(a['disposition'] == 'unsupported_assertion' for a in superseded['assertions']), 2)


if __name__ == '__main__':
    unittest.main()
