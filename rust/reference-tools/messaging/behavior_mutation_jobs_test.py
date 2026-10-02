import unittest
from collections import Counter
from behavior_mutation_jobs import probe_jobs


class ProbeJobsTest(unittest.TestCase):
    def test_named_only_case_never_schedules_nonexistent_default(self):
        self.assertEqual(probe_jobs([["sending"]], ["sending"], negative=True,
                                    mutation_variants={"sending": ["hidden-body"]}),
                         [(["sending"], "hidden-body")])

    def test_mixed_readonly_batch_covers_each_registered_pair_once(self):
        variants = {"menu": ["default", "opacity"], "sending": ["body"],
                    "toolbar": ["default"], "thread": ["default", "title"]}
        jobs = probe_jobs([["menu", "sending", "toolbar"], ["thread"]],
                          list(variants), negative=True, mutation_variants=variants)
        actual = Counter((case, variant) for batch, variant in jobs for case in batch)
        self.assertEqual(actual, Counter((case, variant) for case, names in variants.items()
                                         for variant in names))
        self.assertIn((["menu", "toolbar"], "default"), jobs)
        self.assertIn((["sending"], "body"), jobs)
        self.assertIn((["thread"], "title"), jobs)

    def test_selected_mutant_only_schedules_registered_cases(self):
        self.assertEqual(probe_jobs([["menu", "sending"]], ["menu", "sending"],
                                    negative=True, mutant="opacity",
                                    mutation_variants={"menu": ["opacity"], "sending": []}),
                         [(["menu"], "opacity")])

    def test_unregistered_explicit_variant_cannot_report_an_empty_green_run(self):
        with self.assertRaisesRegex(ValueError, "no registered served variant"):
            probe_jobs([["sending"]], ["sending"], negative=True, mutant="default",
                       mutation_variants={"sending": ["hidden-body"]})

    def test_positive_case_and_allowed_hidden_probe_keep_their_scopes(self):
        self.assertEqual(probe_jobs([["menu", "sending"]], ["menu", "sending"]),
                         [(["menu", "sending"], "default")])
        self.assertEqual(probe_jobs([["menu"]], ["menu"],
                                    diagnostic_variants={"menu": ["allowed-hidden"]}),
                         [(["menu"], "allowed-hidden")])

    def test_full_scope_diagnostic_negative_uses_all_registered_variants(self):
        self.assertEqual(probe_jobs([["menu"]], ["menu"], negative=True,
                                    mutation_variants={"menu": ["default", "opacity"]},
                                    diagnostic_variants={"menu": ["opacity"]}),
                         [(["menu"], "default"), (["menu"], "opacity")])


if __name__ == "__main__":
    unittest.main()
