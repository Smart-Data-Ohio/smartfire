#!/usr/bin/env python3
"""Exercise the ignore guard against real compiler/nextest attribute expansion."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

from ignored_tests import check_compiled


class CompilerIgnoreMutations(unittest.TestCase):
    def test_each_unclassified_compiled_ignore_is_rejected(self):
        forms = {
            "same_line_bare": '#[test] #[ignore] fn same_line_bare() {}',
            "same_line_reason": '#[test] #[ignore = "requires prerequisite"]\nfn same_line_reason() {}',
            "same_line_function": '#[test]\n#[ignore] fn same_line_function() {}',
            "conditional_bare": '#[test]\n#[cfg_attr(test, ignore)] fn conditional_bare() {}',
            "conditional_reason": '#[test]\n#[cfg_attr(test, ignore = "requires prerequisite")] fn conditional_reason() {}',
            "conditional_nested": '#[test]\n#[cfg_attr(all(test, unix), cfg_attr(test, ignore = r#"requires prerequisite"#))] fn conditional_nested() {}',
            "macro_generated": 'macro_rules! hidden { () => { #[test] #[ignore] fn macro_generated() {} }; } hidden!();',
            "included": 'include!("included.rs");',
        }
        with tempfile.TemporaryDirectory(prefix="ignore-mutations-") as scratch:
            root = Path(scratch)
            (root / "src").mkdir()
            (root / "Cargo.toml").write_text('[package]\nname="ignore_mutations"\nversion="0.0.0"\nedition="2024"\n[workspace]\n')
            (root / "src/lib.rs").write_text("\n".join(forms.values()))
            (root / "src/included.rs").write_text('#[test] #[cfg_attr(test, ignore)] fn included() {}')
            document = json.loads(subprocess.check_output([
                "cargo", "nextest", "list", "--manifest-path", str(root / "Cargo.toml"),
                "--build-jobs", "4", "--run-ignored", "only", "--ignore-default-filter", "--message-format", "json",
            ], text=True))
            records = [{"package": "ignore_mutations", "binary": "ignore_mutations", "test": name} for name in forms]
            check_compiled(document, {"probe": records}, [])
            for record in records:
                with self.subTest(mutation=record["test"]):
                    with self.assertRaisesRegex(ValueError, record["test"]):
                        check_compiled(document, {"probe": [other for other in records if other != record]}, [])


if __name__ == "__main__":
    unittest.main(verbosity=2)
