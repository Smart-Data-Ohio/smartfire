"""Workflow contracts and local execution of the release-plan shell step."""
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
REVISION = "a2fbe296f0675b1a657cf81c1f537b6687451403"


def yaml_json(text):
    return json.loads(subprocess.check_output(["mise", "exec", "ruby@3.4.10", "--", "ruby", "-ryaml", "-rjson",
        "-e", "puts JSON.generate(YAML.safe_load(STDIN.read))"], input=text, text=True))


def audit_actions(document):
    count = 0
    for job in document["jobs"].values():
        for step in job.get("steps", []):
            if "uses" in step:
                if not re.fullmatch(r"[^@]+@[0-9a-f]{40}", step["uses"]):
                    raise ValueError(f"unpinned action: {step['uses']}")
                count += 1
    return count


def plan(document, runtime):
    step = next(step for step in document["jobs"]["deploy"]["steps"] if step.get("id") == "plan")
    with tempfile.TemporaryDirectory(dir=ROOT / ".scratch", prefix="workflow-plan-") as tmp:
        work = Path(tmp)
        fake_git = work / "git"
        fake_git.write_text("#!/bin/sh\ncase \"$1\" in\nfetch) exit 0 ;;\nrev-parse) printf '%s\\n' '" + REVISION + "' ;;\n*) exit 99 ;;\nesac\n")
        fake_git.chmod(0o755)
        for name in ["gcloud", "gh", "docker"]:
            poison = work / name
            poison.write_text("#!/bin/sh\necho forbidden-external-command >&2\nexit 99\n")
            poison.chmod(0o755)
        output = work / "output"
        env = {**os.environ, "PATH": f"{work}:{os.environ['PATH']}", "GITHUB_OUTPUT": str(output),
            "INPUT_SHA": REVISION, "INPUT_LABEL": "ws18-fixture", "INPUT_DIGEST": "", "INPUT_RUNTIME": runtime,
            "INPUT_DRY_RUN": "true", "INPUT_SKIP_SNAPSHOT": "false", "INPUT_RESUME": "false",
            "INPUT_ENVIRONMENT": "validation", "RUN_ID": "42", "SIMULATE_FAILURE_VAR": "0"}
        result = subprocess.run(["bash", "-c", step["run"]], cwd=ROOT, env=env, text=True, capture_output=True)
        if result.returncode:
            raise RuntimeError(result.stdout + result.stderr)
        return dict(line.split("=", 1) for line in output.read_text().splitlines())


class WorkflowTest(unittest.TestCase):
    def test_default_rails_plan_is_identical_and_rust_uses_a_separate_sha_tag(self):
        old = subprocess.check_output(["git", "show", f"{REVISION}:.github/workflows/deploy-gcp.yml"], cwd=ROOT, text=True)
        old = yaml_json(old)
        current = yaml_json((ROOT / ".github/workflows/deploy-gcp.yml").read_text())
        expected = plan(old, "")
        self.assertEqual(plan(current, ""), expected)
        self.assertEqual(plan(current, "rails"), expected)
        self.assertEqual(plan(current, "rust"), {**expected, "tag": f"rust-git-{REVISION}"})
        on = current.get("on", current.get("true"))
        self.assertEqual(on["workflow_dispatch"]["inputs"]["runtime"]["default"], "rails")
        print("WORKFLOW PLAN: Rails default unchanged; Rust full-SHA tag selected")

    def test_image_workflow_has_readonly_pr_builds_and_main_only_publish(self):
        image = yaml_json((ROOT / ".github/workflows/publish-rust-image.yml").read_text())
        pr, publish = image["jobs"]["pull-request"], image["jobs"]["publish"]
        self.assertEqual(pr["permissions"], {"contents": "read"})
        self.assertEqual(pr["if"], "github.event_name == 'pull_request'")
        self.assertEqual(publish["if"].strip(), "github.event_name == 'push' && github.ref == 'refs/heads/main'")
        builds = [step["with"] for step in pr["steps"] if step.get("uses", "").startswith("docker/build-push-action@")]
        self.assertEqual(len(builds), 1)
        self.assertFalse(builds[0]["push"])
        self.assertNotIn("cache-to", builds[0])
        self.assertEqual(builds[0]["build-contexts"], "reference=.")
        for workflow in [image, yaml_json((ROOT / ".github/workflows/deploy-gcp.yml").read_text())]:
            self.assertGreater(audit_actions(workflow), 0)
        print("WORKFLOW ACTIONS: all third-party actions pinned; PRs have no publish credentials/cache writes")

    def test_action_audit_rejects_unpinned_references(self):
        with self.assertRaises(ValueError):
            audit_actions({"jobs": {"bad": {"steps": [{"uses": "actions/checkout@main"}]}}})


if __name__ == "__main__":
    unittest.main()
