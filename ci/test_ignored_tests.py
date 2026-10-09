import json
from pathlib import Path
import tempfile
import unittest

from ignored_tests import ROOT, check, check_compiled, inventory, shard, verify_junit, workflow_suites


class IgnoredTestCoverage(unittest.TestCase):
    def test_ignore_attribute_mutations_fail_closed(self):
        mutations = (
            '#[test] #[ignore]\nfn unowned() {}',
            '#[test] #[ignore = "requires prerequisite"]\nfn unowned() {}',
            '#[test]\n#[ignore] fn unowned() {}',
            '#[test]\n#[cfg_attr(test, ignore)] fn unowned() {}',
            '#[test]\n#[cfg_attr(test, ignore = "requires prerequisite")] fn unowned() {}',
            '#[test]\n#[cfg_attr(all(test, unix), cfg_attr(test, ignore = r#"requires prerequisite"#))] fn unowned() {}',
            '#[test]\n#[cfg_attr(any(), ignore)] fn unowned() {}',
        )
        workflow = 'suite: [server]\nbash ci/correctness.sh "$SUITE"'
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            for source in mutations:
                with self.subTest(source=source):
                    (root / "test.rs").write_text(source)
                    self.assertEqual(set(inventory(root)), {("test.rs", "unowned")})
                    with self.assertRaisesRegex(ValueError, "no CI job"):
                        check(root, {}, workflow)

    def test_comments_and_literals_do_not_create_ignore_attributes(self):
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            (root / "test.rs").write_text('''
                /* nested /* #[ignore] */ fn comment() {} */
                // #[cfg_attr(test, ignore)] fn comment() {}
                const NORMAL: &str = "#[ignore] fn normal() {}";
                const RAW: &str = r###"#[cfg_attr(test, ignore)] fn raw() {}"###;
                #[test] #[cfg_attr(test, ignore = r#"utility: recorder"#)] fn utility() {}
            ''')
            self.assertEqual(inventory(root), {("test.rs", "utility"): "utility: recorder"})

    def test_compiler_inventory_rejects_expansion_and_stale_classification(self):
        def document(names):
            return {"rust-suites": {"probe": {
                "package-name": "probe", "binary-name": "probe", "binary-id": "probe", "status": "listed",
                "testcases": {name: {"ignored": True, "filter-match": {"status": "matches"}} for name in names},
            }}}
        record = {"package": "probe", "binary": "probe", "test": "selected"}
        check_compiled(document(["selected"]), {"server": [record]}, [])
        for names in ([], ["selected", "macro_generated_ignore"]):
            with self.subTest(names=names), self.assertRaisesRegex(ValueError, "inventory mismatch"):
                check_compiled(document(names), {"server": [record]}, [])
        with self.assertRaisesRegex(ValueError, "duplicate"):
            check_compiled(document(["selected"]), {"server": [record]}, [record])

    def test_repository_has_no_unowned_ignored_correctness(self):
        check(ROOT, json.loads((ROOT / "ci/ignored-tests.json").read_text()),
              (ROOT / ".github/workflows/rust.yml").read_text())

    def test_new_ignored_test_and_missing_job_fail_closed(self):
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            (root / "crates").mkdir()
            source = root / "crates/tests.rs"
            source.write_text('// #[ignore]\n// fn prose() {}\n#[test]\n#[ignore = "requires a server"]\nasync fn correctness() {}\n')
            record = {"path": "crates/tests.rs", "test": "correctness"}
            workflow = 'suite: [server]\nbash ci/correctness.sh "$SUITE"'
            self.assertEqual(len(inventory(root)), 1)
            with self.assertRaisesRegex(ValueError, "no CI job"):
                check(root, {}, workflow)
            with self.assertRaisesRegex(ValueError, "missing CI job"):
                check(root, {"absent": [record]}, workflow)
            self.assertEqual(check(root, {"server": [record]}, workflow), (1, 0))
            source.write_text('#[test]\n#[ignore = "utility: recorder"]\nfn correctness() {}\n')
            self.assertEqual(check(root, {}, workflow), (0, 1))
            with self.assertRaisesRegex(ValueError, "stale"):
                check(root, {"server": [record]}, workflow)

    def test_missing_or_skipped_selected_test_cannot_make_a_green_receipt(self):
        with tempfile.TemporaryDirectory() as scratch:
            path = Path(scratch) / "junit.xml"
            records = [{"test": "selected"}]
            for body in ('', '<testcase name="selected"><skipped/></testcase>',
                         '<testcase name="selected"><failure/></testcase>', '<testcase name="unrelated"/>'):
                path.write_text(f"<testsuites><testsuite>{body}</testsuite></testsuites>")
                with self.assertRaises(ValueError):
                    verify_junit(records, path)
            path.write_text('<testsuites><testsuite><testcase name="selected"/></testsuite></testsuites>')
            verify_junit(records, path)

    def test_shards_partition_every_selected_test_exactly_once(self):
        records = [{"package": "p", "binary": "b", "test": f"t{index}", "seconds": index % 5} for index in range(17)]
        for count in (1, 2, 3, 17):
            with self.subTest(count=count):
                parts = [shard(records, (index, count)) for index in range(1, count + 1)]
                names = [record["test"] for part in parts for record in part]
                self.assertEqual(sorted(names), sorted(record["test"] for record in records))
                self.assertTrue(all(parts))
                self.assertEqual(parts, [shard(list(reversed(records)), (index, count)) for index in range(1, count + 1)])
        with self.assertRaisesRegex(ValueError, "empty"):
            shard(records, (1, 18))

    def test_union_of_shard_receipts_must_be_exact(self):
        with tempfile.TemporaryDirectory() as scratch:
            first, second = Path(scratch) / "1.xml", Path(scratch) / "2.xml"
            records = [{"test": "a"}, {"test": "b"}]
            first.write_text('<testsuites><testsuite><testcase name="a"/></testsuite></testsuites>')
            with self.assertRaises(ValueError):
                verify_junit(records, [first])
            second.write_text('<testsuites><testsuite><testcase name="b"><failure/></testcase></testsuite></testsuites>')
            with self.assertRaises(ValueError):
                verify_junit(records, [first, second])
            second.write_text('<testsuites><testsuite><testcase name="a"/></testsuite></testsuites>')
            with self.assertRaises(ValueError):
                verify_junit(records, [first, second])
            second.write_text('<testsuites><testsuite><testcase name="b"/></testsuite></testsuites>')
            verify_junit(records, [first, second])

    def test_workflow_shards_must_cover_one_to_n(self):
        runner = 'bash ci/correctness.sh "$SUITE"\n'
        self.assertEqual(workflow_suites(runner + 'suite: [a, b]\nsuite: [c]\n        shard: ["1/2", "2/2"]'),
                         {"a": [[]], "b": [[]], "c": [["1/2", "2/2"]]})
        for shards in ('["1/2"]', '["1/3", "2/3", "2/3"]', '["0/1"]', '["2/2", "1/2"]'):
            with self.subTest(shards=shards), self.assertRaisesRegex(ValueError, "1/N"):
                workflow_suites(runner + f"suite: [c]\n  shard: {shards}")
        with self.assertRaisesRegex(ValueError, "not invoked"):
            workflow_suites("suite: [a]")

    def test_repository_workflow_shards_are_parsed_for_browsers_and_messaging(self):
        jobs = workflow_suites((ROOT / ".github/workflows/rust.yml").read_text())
        self.assertEqual(jobs["browsers"], [["1/3", "2/3", "3/3"]])
        self.assertEqual(jobs["messaging"], [["1/4", "2/4", "3/4", "4/4"]])
