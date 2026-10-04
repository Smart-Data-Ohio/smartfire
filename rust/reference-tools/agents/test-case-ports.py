#!/usr/bin/env python3
"""Exercise named-case discovery and audited API evidence reconciliation."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest import mock

root = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location('ports', Path(__file__).with_name('check-case-ports.py'))
ports = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ports)
spec = importlib.util.spec_from_file_location('named_api', Path(__file__).with_name('check-named-api-cases.py'))
named_api = importlib.util.module_from_spec(spec)
spec.loader.exec_module(named_api)

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

    def test_ws12_assignment_macro_and_missing_invocations(self):
        source = (root/'rust/crates/db/src/tests/ws12_agent_named_test.rs').read_text()
        import json
        manifest = json.loads(Path(__file__).with_name('case-ports.json').read_text())
        expected = {c['rust'] for g in manifest['files'] if g['rust_file'].endswith('ws12_agent_named_test.rs') for c in g['cases']}
        self.assertEqual(len(expected), 16)
        self.assertTrue(expected <= ports.rust_ports(source))
        self.assertFalse(expected <= ports.rust_ports(source.replace('cases!', 'unsupported_cases!')))

    def test_literal_async_and_peer_names(self):
        source = '\n#[test]\nfn ws12_fixture() {}\n#[tokio::test]\nasync fn ws8_fixture() {}\n'
        self.assertEqual(ports.rust_ports(source), {'ws12_fixture', 'ws8_fixture'})
        self.assertNotIn('missing_test', ports.rust_ports(source))


class CasePortReferences(unittest.TestCase):
    def test_current_pin_overrides_mapping_reference(self):
        with tempfile.TemporaryDirectory() as scratch:
            private_root = Path(scratch)
            checker = private_root / 'rust/reference-tools/agents/check-case-ports.py'
            checker.parent.mkdir(parents=True)
            mapping = json.loads(Path(ports.__file__).with_name('case-ports.json').read_text())
            mapping['reference_pin'] = 'missing-historical-pin'
            mapping['files'] = mapping['files'][:1]
            inputs = {Path('rust/parity/reference.sha')}
            for group in mapping['files']:
                inputs.update((Path(group['rails_file']), Path(group['rust_file'])))
            for name in inputs:
                target = private_root / name
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(root / name, target)
            checker.with_name('case-ports.json').write_text(json.dumps(mapping))
            (private_root / '.git').symlink_to(root / '.git')
            with mock.patch.object(ports, 'ROOT', private_root), \
                    mock.patch.object(ports, '__file__', str(checker)), \
                    contextlib.redirect_stdout(io.StringIO()):
                ports.check()


class NamedApiCredits(unittest.TestCase):
    def setUp(self):
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)
        self.ledger_path = self.root / 'rust/plans/ws11api-named-api-cases.json'
        self.ledger = json.loads((root / self.ledger_path.relative_to(self.root)).read_text())
        inputs = {self.ledger_path.relative_to(self.root), Path(self.ledger['audit_source']),
                  Path('rust/parity/reference.sha'),
                  Path('rust/plans/ws11api-next-5-controls.json')}
        for case in self.ledger['cases']:
            inputs.add(Path(case['rails_file']))
            for field in ('rust_file', 'vector', 'producer_control', 'receipt'):
                if field in case:
                    inputs.add(Path(case[field]))
        for name in inputs:
            target = self.root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(root / name, target)
        # git show reads the real pin; only the checker inputs are private copies.
        (self.root / '.git').symlink_to(root / '.git')
        patch = mock.patch.object(named_api, 'ROOT', self.root)
        patch.start()
        self.addCleanup(patch.stop)
        self.log = '\n'.join(f"test controllers::agent_work_named_tests::{c['rust_test']} ... ok"
                             for c in self.ledger['cases'] if c['status'] == 'passed')

    def case(self, key):
        return next(c for c in self.ledger['cases'] if c.get('key') == key)

    def check(self, log=None):
        self.ledger_path.write_text(json.dumps(self.ledger))
        with contextlib.redirect_stdout(io.StringIO()):
            named_api.check(self.log if log is None else log)

    def test_real_ledger_and_all_passes(self):
        self.check()

    def test_current_pin_overrides_historical_ledger_reference(self):
        self.ledger['reference'] = 'missing-historical-pin'
        self.check()

    def test_redirected_test_with_only_nineteen_passes(self):
        case = self.case('result_clear')
        source = self.root / case['rust_file']
        source.write_text(source.read_text().replace('    ws11_named_result_clear => "result_clear",\n', ''))
        case['rust_test'] = 'ws11_named_result_noop'
        log = '\n'.join(line for line in self.log.splitlines() if '::ws11_named_result_clear ' not in line)
        self.assertEqual(len(log.splitlines()), len(self.log.splitlines()) - 1)
        with self.assertRaises(AssertionError):
            self.check(log)

    def test_deleted_pending_declarations(self):
        if all(c['status'] == 'passed' for c in self.ledger['cases']):
            self.ledger['cases'][0]['status'] = 'pending'
        self.ledger['cases'] = [c for c in self.ledger['cases'] if c['status'] == 'passed']
        with self.assertRaises(AssertionError):
            self.check()

    def test_duplicated_vector_reference(self):
        # A distinct pinned declaration can borrow this vector's same Rails name.
        case = dict(self.case('posts_credentials'),
                    rails_file='test/controllers/agents/messages_controller_test.rb',
                    rails_line=125, rust_test='ws11_duplicate_credentials')
        target = self.root / case['rails_file']
        shutil.copyfile(root / case['rails_file'], target)
        audit_path = self.root / self.ledger['audit_source']
        audit = json.loads(audit_path.read_text())
        audit['cases'].append({'file': case['rails_file'], 'line': case['rails_line'],
                               'test': case['rails_test'], 'owner': 'WS11-API'})
        audit_path.write_text(json.dumps(audit))
        self.ledger['cases'].append(case)
        source = self.root / case['rust_file']
        source.write_text(source.read_text() + '\ncases! { ws11_duplicate_credentials => "posts_credentials" }\n')
        self.log += '\ntest controllers::agent_work_named_tests::ws11_duplicate_credentials ... ok'
        with self.assertRaises(AssertionError):
            self.check()

    def test_missing_failed_or_ignored_pass(self):
        for status in (None, 'FAILED', 'ignored'):
            with self.subTest(status=status):
                line = 'test controllers::agent_work_named_tests::ws11_named_result_clear ... ok'
                replacement = '' if status is None else line.replace('ok', status)
                with self.assertRaises(AssertionError):
                    self.check(self.log.replace(line, replacement))

    def test_pending_requires_reason(self):
        case = next((c for c in self.ledger['cases'] if c['status'] == 'pending'), self.ledger['cases'][0])
        case.update(status='pending', reason=' ')
        with self.assertRaises(AssertionError):
            self.check()

    def test_source_must_run_the_claimed_vector(self):
        source = self.root / self.case('result_clear')['rust_file']
        source.write_text(source.read_text().replace(
            'ws11_named_result_clear => "result_clear"', 'ws11_named_result_clear => "result_noop"'))
        with self.assertRaises(AssertionError):
            self.check()

    def test_control_and_receipt_must_name_the_claimed_test(self):
        for field, rows in (('producer_control', 'declarations'), ('receipt', 'named_api')):
            with self.subTest(field=field):
                path = self.root / self.case('result_clear')[field]
                original = path.read_text()
                data = json.loads(original)
                next(c for c in data[rows] if c['key'] == 'result_clear')['rust_test'] = 'ws11_named_result_noop'
                path.write_text(json.dumps(data))
                with self.assertRaises(AssertionError):
                    self.check()
                path.write_text(original)

    def test_extra_declaration_is_rejected(self):
        self.ledger['cases'].append(dict(self.case('posts_credentials'),
                                        rails_file='test/controllers/agents/messages_controller_test.rb'))
        with self.assertRaises(AssertionError):
            self.check()

    def test_ambiguous_pass_is_rejected(self):
        with self.assertRaises(AssertionError):
            self.check(self.log + '\ntest unrelated::ws11_named_result_clear ... ok')

if __name__ == '__main__':
    unittest.main()
