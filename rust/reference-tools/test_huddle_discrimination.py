import contextlib
import io
import tempfile
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest import mock
import huddle_discrimination as runner


class MutationCatalogueTest(unittest.TestCase):
    def test_every_catalogue_mutation_has_a_live_unique_anchor(self):
        self.assertEqual(len(runner.mutations), 102)
        self.assertEqual(len({entry[0] for entry in runner.mutations}), 102)
        self.assertEqual(runner.preflight(runner.mutations), [])

    def test_preflight_reports_all_missing_anchors_without_editing_sources(self):
        path = runner.H
        original = path.read_text()
        missing = lambda source: runner.replace_once(source, 'missing-ws13-anchor', '')
        entries = [('first', path, missing, 'campfire', ''), ('second', path, missing, 'campfire', '')]
        errors = runner.preflight(entries)
        self.assertEqual(len(errors), 2)
        self.assertTrue(errors[0].startswith('first:'))
        self.assertTrue(errors[1].startswith('second:'))
        self.assertEqual(path.read_text(), original)

    def test_failing_or_empty_baselines_never_edit_sources(self):
        path = runner.H
        original = path.read_text()
        entry = ('baseline-guard', path, lambda source: source + '\n// mutation\n', 'campfire', 'baseline-test')
        for result in [
            SimpleNamespace(returncode=101, stdout='panicked at baseline\ntest result: FAILED. 0 passed; 1 failed;\n', stderr=''),
            SimpleNamespace(returncode=0, stdout='test result: ok. 0 passed; 0 failed;\n', stderr=''),
        ]:
            with self.subTest(result=result.returncode), tempfile.TemporaryDirectory(dir=runner.SCRATCH) as temporary:
                observed = []
                def run(*args, **kwargs):
                    observed.append(path.read_text())
                    return result
                with mock.patch.object(runner, 'mutations', [entry]), mock.patch.object(runner.sys, 'argv', ['huddle_discrimination.py']), mock.patch.object(runner, 'SCRATCH', Path(temporary)), mock.patch.object(runner.subprocess, 'run', side_effect=run), contextlib.redirect_stdout(io.StringIO()):
                    with self.assertRaises(SystemExit):
                        runner.main()
                self.assertEqual(observed, [original])
                self.assertEqual(path.read_text(), original)


if __name__ == '__main__':
    unittest.main()
