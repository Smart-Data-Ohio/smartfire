import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("gate_needs", Path(__file__).with_name("check_gate_needs.py"))
gate_needs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate_needs)

WORKFLOW = Path(__file__).resolve().parents[1] / ".github/workflows/rust.yml"


class GateNeedsTest(unittest.TestCase):
    def test_rust_workflow_gates_every_job(self):
        self.assertEqual(gate_needs.problems(WORKFLOW.read_text()), [])

    def test_matches_a_yaml_parser_when_one_is_available(self):
        try:
            import yaml
        except ImportError:
            self.skipTest("PyYAML not installed")
        parsed = yaml.safe_load(WORKFLOW.read_text())["jobs"]
        expected = {job: ([body["needs"]] if isinstance(body.get("needs"), str) else body.get("needs", []))
                    for job, body in parsed.items()}
        self.assertEqual(gate_needs.jobs(WORKFLOW.read_text()), expected)

    def test_a_new_job_outside_the_gates_fails(self):
        text = WORKFLOW.read_text() + "\n  extra:\n    runs-on: ubuntu-latest\n    steps:\n      - run: true\n"
        self.assertEqual(gate_needs.problems(text), ["job 'extra' is not in the needs of rust"])

    def test_block_and_scalar_needs_are_read(self):
        text = ("on: push\njobs:\n  a:\n    runs-on: x\n  b:\n    needs: a\n"
                "  rust:\n    needs:\n      - a\n      - b # comment\n"
                "")
        self.assertEqual(gate_needs.jobs(text), {"a": [], "b": ["a"], "rust": ["a", "b"]})
        self.assertEqual(gate_needs.problems(text), [])


if __name__ == "__main__":
    unittest.main()
