"""Run the workflow path detector against real Git diffs."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from test_workflows import ROOT, yaml_json

DETECTOR = ROOT / ".github/scripts/check-paths.py"


class PathScopeTest(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory()
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)
        self.repo = self.root / "repo"
        self.repo.mkdir()
        self.git("init", "-q")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "user.name", "Fixture")
        self.git("commit", "--allow-empty", "-qm", "base")
        self.base = self.git("rev-parse", "HEAD").strip()
        self.repo_workflow = yaml_json((ROOT / ".github/workflows/repo.yml").read_text())

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.repo, text=True)

    def commit(self, path):
        target = self.repo / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text("changed")
        self.git("add", ".")
        self.git("commit", "-qm", "input changes")
        return self.git("rev-parse", "HEAD").strip()

    def run_scope(self, event, pattern):
        event_path = self.root / "event.json"
        output = self.root / "output"
        event_path.write_text(json.dumps(event))
        output.unlink(missing_ok=True)
        result = subprocess.run(["python3", str(DETECTOR)], cwd=self.repo, text=True, capture_output=True,
            env={**os.environ, "GITHUB_EVENT_PATH": str(event_path), "GITHUB_OUTPUT": str(output), "PATTERN": pattern})
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return output.read_text().strip()

    def test_required_repo_jobs_checkout_before_scope_and_filter_only_their_work(self):
        self.assertEqual({body["name"] for body in self.repo_workflow["jobs"].values()},
                         {"GitHub Actions audit", "Ops scripts", "Huddle authorization gateway", "Dependency audit"})
        for job, body in self.repo_workflow["jobs"].items():
            with self.subTest(job=job):
                steps = body["steps"]
                self.assertTrue(steps[0]["uses"].startswith("actions/checkout@"))
                self.assertNotIn("if", steps[0])
                self.assertEqual(steps[1]["id"], "scope")
                self.assertNotIn("if", steps[1])
                self.assertEqual(sum(step.get("uses", "").startswith("actions/checkout@") for step in steps), 1)
                for step in steps[2:]:
                    self.assertEqual(step["if"], "steps.scope.outputs.run == 'true'")
                if job == "dependency-audit":
                    self.assertNotIn("if", body)
                else:
                    self.assertEqual(body["if"], "github.event_name != 'schedule'")

    def test_each_required_repo_job_selects_its_own_inputs_on_prs_and_pushes(self):
        cases = [
            (".github/actions/custom/action.yml", {"lint-actions"}),
            ("huddle-gateway/src/index.mjs", {"test_gateway"}),
            ("deploy/huddles/runtime/server.mjs", {"test_gateway", "ops-scripts"}),
            ("ops/backup.sh", {"ops-scripts"}),
            ("deploy/gcp/configure.py", {"ops-scripts"}),
            (".github/workflows/frontend.yml", {"lint-actions", "ops-scripts"}),
            ("crates/db/Cargo.toml", {"dependency-audit"}),
            ("Cargo.lock", {"dependency-audit"}),
            ("frontend/pnpm-lock.yaml", {"dependency-audit"}),
            ("huddle-gateway/package-lock.json", {"dependency-audit", "test_gateway"}),
            ("docs/usage.md", set()),
        ]
        for path, expected in cases:
            head = self.commit(path)
            events = [{"pull_request": {"base": {"sha": self.base}, "head": {"sha": head}}},
                      {"before": self.base, "after": head}]
            for job, body in self.repo_workflow["jobs"].items():
                scope = next(step for step in body["steps"] if step.get("id") == "scope")
                for event in events:
                    with self.subTest(path=path, job=job, event=event):
                        self.assertEqual(self.run_scope(event, scope["env"]["PATTERN"]),
                                         "run=true" if job in expected else "run=false")
            self.base = head

    def test_frontend_mock_server_needs_no_rust_crates(self):
        frontend = yaml_json((ROOT / ".github/workflows/frontend.yml").read_text())
        scope = next(step for step in frontend["jobs"]["frontend"]["steps"] if step.get("id") == "scope")
        for path, expected in [("frontend/mock/vite-plugin.ts", "run=true"),
                               ("crates/spa/build.rs", "run=true"),
                               ("crates/web/src/lib.rs", "run=false"),
                               ("deploy/README.md", "run=false")]:
            head = self.commit(path)
            self.assertEqual(self.run_scope({"before": self.base, "after": head}, scope["env"]["PATTERN"]), expected)
            self.base = head

    def test_deletions_and_renames_include_the_previous_path(self):
        self.base = self.commit("frontend/old.ts")
        self.git("mv", "frontend/old.ts", "unrelated.txt")
        self.git("commit", "-qm", "move out of frontend")
        head = self.git("rev-parse", "HEAD").strip()
        self.assertEqual(self.run_scope({"before": self.base, "after": head}, "^frontend/"), "run=true")

    def test_scheduled_events_and_unavailable_history_run_checks(self):
        self.assertEqual(self.run_scope({}, "^Cargo.lock$"), "run=true")
        self.assertEqual(self.run_scope({"before": "a" * 40, "after": self.base}, "^Cargo.lock$"), "run=true")
