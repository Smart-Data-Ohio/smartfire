"""The audit must not lose newer code, launch the app, or credit skipped assertions."""
import argparse
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import tempfile
import sys
import threading
from concurrent.futures import Future
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("audit", Path(__file__).with_name("ws12_assertion_mutations.py"))
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


class AuditGuards(unittest.TestCase):
    def test_restore_refuses_every_write_if_a_later_file_has_newer_code(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ["first.rs", "last.rs"]:
                (root / name).write_text("fn value() -> i32 { 1 }\n")
            catalog = {"mutations": [{"key": "wrong", "edits": [
                {"file": name, "before": "{ 1 }", "after": "{ 2 }", "occurrences": 1}
                for name in ["first.rs", "last.rs"]]}]}
            scratch = root / "scratch"
            with patch.object(audit, "ROOT", root), contextlib.redirect_stdout(io.StringIO()):
                audit.install(catalog, scratch)
                first = (root / "first.rs").read_bytes()
                (root / "last.rs").write_text("newer main code\n")
                with self.assertRaisesRegex(AssertionError, "refuse overwriting"):
                    audit.restore(scratch)
                self.assertEqual((root / "first.rs").read_bytes(), first)
                self.assertEqual((root / "last.rs").read_text(), "newer main code\n")

    def test_shared_rust_field_recipes_preserve_the_unselected_expression(self):
        before = '    work_label: post.work_status_label(),'
        edits = [{"mode": name, "after": f'    work_label: if ws12_coverage_mutant("{name}") {{ "Wrong".into() }} else {{ post.work_status_label() }},'}
                 for name in ["first", "second"]]
        combined = audit.combine(before, edits)
        self.assertTrue(combined.startswith('    work_label: if '))
        self.assertIn(' else { post.work_status_label() }', combined)
        self.assertTrue(combined.endswith(' } },'))
        self.assertEqual(combined.count('ws12_coverage_mutant('), 2)
        self.assertEqual(combined.count('std::env::var('), 2)

    def campaign(self, summary, returncode=0, selector=None):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / "target"
            for kind, stem in [("bin", "campfire"), ("lib", "campfire_db")]:
                path = target / "debug/deps" / (stem + "-abc")
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("never execute")
                path.chmod(0o755)
                fingerprint = target / "debug/.fingerprint" / path.name
                fingerprint.mkdir(parents=True)
                (fingerprint / f"test-{kind}-{stem}.json").write_text("{}")
            production = target / "debug/deps/campfire-production"
            production.write_text("never launch the real application")
            production.chmod(0o755)
            catalog = {"declarations": [{"id": "c000", "file": "test/example.rb", "line": 1,
                "test": "example", "mutation": "wrong", "tests": [{
                    "rust_file": "rust/crates/campfire/example.rs", "rust_test": "named_assertion"}]}]}
            args = argparse.Namespace(scratch=root / "scratch", target=target, action="baseline",
                                      declaration=selector, workers=1, previous_tests=False)
            invoked = []

            def run(command, **kwargs):
                invoked.append(command)
                self.assertTrue((args.scratch / "tmp").is_dir())
                kwargs["stdout"].write(summary)
                return subprocess.CompletedProcess(command, returncode)

            with patch.object(audit, "ROOT", root), \
                 patch.object(audit.subprocess, "check_output", return_value="tests::named_assertion: test\n") as listed, \
                 patch.object(audit.subprocess, "run", side_effect=run), \
                 contextlib.redirect_stdout(io.StringIO()):
                audit.run(catalog, args)
            self.assertEqual(len(invoked), 1)
            self.assertTrue(all("production" not in str(call) for call in listed.call_args_list))
            receipt = json.loads((args.scratch / "baseline-results.json").read_text())
            self.assertEqual(receipt[0]["groups"][0]["exit"], 0)

    def test_duplicate_baselines_receive_the_owners_timeout_exception(self):
        # Run in a bounded child because the old unresolved Future deadlocks shutdown.
        child = subprocess.run([sys.executable, str(Path(__file__).resolve()), "--future-child"],
                               capture_output=True, text=True, timeout=5)
        self.assertEqual(child.returncode, 0, child.stdout + child.stderr)
        self.assertIn("WS12_BASELINE_EXCEPTION propagated to duplicate; no hang", child.stdout)

    def test_baseline_creates_tmp_and_never_discovers_the_production_binary(self):
        self.campaign("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured;\n")

    def test_a_baseline_failure_is_not_a_successful_campaign(self):
        with self.assertRaisesRegex(SystemExit, "baseline failed"):
            self.campaign("test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured;\n", 101)

    def test_ignored_assertions_do_not_earn_credit(self):
        with self.assertRaisesRegex(AssertionError, "all selected assertions must run"):
            self.campaign("test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured;\n")

    def test_a_misspelled_declaration_does_not_run_zero_tests_successfully(self):
        with self.assertRaisesRegex(AssertionError, "unknown/empty declaration"):
            self.campaign("", selector=["c999"])


def future_child():
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        target = root / "target"
        for stem in ["campfire", "campfire_db"]:
            binary = target / "debug/deps" / (stem + "-fixture")
            binary.parent.mkdir(parents=True, exist_ok=True)
            binary.write_text("fake executable")
            binary.chmod(0o700)
            fingerprint = target / "debug/.fingerprint" / binary.name
            fingerprint.mkdir(parents=True)
            (fingerprint / "test-fixture.json").write_text("{}")
        waiting = threading.Event()
        class ObservedFuture(Future):
            def result(self, *args, **kwargs):
                waiting.set()
                return super().result(*args, **kwargs)
        def timeout(*args, **kwargs):
            assert waiting.wait(2), "duplicate never waited"
            raise subprocess.TimeoutExpired(["fake-test"], 600)
        rows = [{"id": str(i), "file": "fixture.rb", "line": 1, "test": "duplicate",
                 "mutation": "fake", "tests": [{"rust_file": "rust/crates/db/fixture.rs",
                                                   "rust_test": "selected"}]} for i in range(2)]
        args = argparse.Namespace(scratch=root / "scratch", target=target, action="baseline",
                                  declaration=None, workers=2, previous_tests=False)
        with patch.object(audit, "ROOT", root), patch.object(audit, "Future", ObservedFuture), \
             patch.object(audit.subprocess, "check_output", return_value="fixture::selected: test\n"), \
             patch.object(audit.subprocess, "run", side_effect=timeout):
            try:
                audit.run({"declarations": rows}, args)
            except subprocess.TimeoutExpired:
                print("WS12_BASELINE_EXCEPTION propagated to duplicate; no hang")
            else:
                raise AssertionError("timeout must propagate")

if __name__ == "__main__":
    if "--future-child" in sys.argv:
        future_child()
    else:
        unittest.main()
