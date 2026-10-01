import unittest
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


if __name__ == '__main__':
    unittest.main()
