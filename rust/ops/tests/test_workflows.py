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


def plan(document, runtime=None):
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
        extra = {} if runtime is None else {"INPUT_RUNTIME": runtime}
        env = {**os.environ, "PATH": f"{work}:{os.environ['PATH']}", "GITHUB_OUTPUT": str(output),
            "INPUT_SHA": REVISION, "INPUT_LABEL": "ws18-fixture", "INPUT_DIGEST": "",
            "INPUT_DRY_RUN": "true", "INPUT_SKIP_SNAPSHOT": "false", "INPUT_RESUME": "false",
            "INPUT_ENVIRONMENT": "validation", "RUN_ID": "42", "SIMULATE_FAILURE_VAR": "0", **extra}
        result = subprocess.run(["bash", "-c", step["run"]], cwd=ROOT, env=env, text=True, capture_output=True)
        if result.returncode:
            raise RuntimeError(result.stdout + result.stderr)
        return dict(line.split("=", 1) for line in output.read_text().splitlines())


class WorkflowTest(unittest.TestCase):
    def test_vm_phases_receive_the_same_resolved_digest_reference(self):
        workflow = yaml_json((ROOT / ".github/workflows/deploy-gcp.yml").read_text())
        steps = {step["id"]: step for step in workflow["jobs"]["deploy"]["steps"] if "id" in step}
        self.assertIn('echo "reference=${GCP_IMAGE}@${digest}"', steps["image"]["run"])
        for phase in ["preflight", "freeze", "cutover"]:
            with self.subTest(phase=phase):
                self.assertEqual(steps[phase]["env"]["IMAGE_REF"], "${{ steps.image.outputs.reference }}")
                self.assertIn("IMAGE_REF='${IMAGE_REF}'", steps[phase]["run"])
                self.assertIn("campfire-release.sh " + phase, steps[phase]["run"])
        print("WORKFLOW REFERENCE: preflight/freeze/cutover use the same resolved repository@digest string")

    def test_plan_always_selects_the_rust_sha_tag(self):
        old = subprocess.check_output(["git", "show", f"{REVISION}:.github/workflows/deploy-gcp.yml"], cwd=ROOT, text=True)
        expected = plan(yaml_json(old), "")
        current = yaml_json((ROOT / ".github/workflows/deploy-gcp.yml").read_text())
        self.assertEqual(plan(current), {**expected, "tag": f"rust-git-{REVISION}"})
        on = current.get("on", current.get("true"))
        self.assertNotIn("runtime", on["workflow_dispatch"]["inputs"])
        self.assertNotIn("INPUT_RUNTIME", next(step for step in current["jobs"]["deploy"]["steps"] if step.get("id") == "plan")["env"])
        print("WORKFLOW PLAN: only the Rust full-SHA tag; no runtime input")

    def test_release_is_gated_on_the_rust_checks_alone(self):
        workflow = yaml_json((ROOT / ".github/workflows/deploy-gcp.yml").read_text())
        steps = {step["id"]: step for step in workflow["jobs"]["deploy"]["steps"] if "id" in step}
        gate = steps["ci"]["run"]
        self.assertIn("rust.yml", gate)
        self.assertIn("push", gate)
        self.assertIn("schedule", gate)
        self.assertNotIn("ci.yml", (ROOT / ".github/workflows/deploy-gcp.yml").read_text())
        self.assertIn('select(.name == "Rust port")', gate)
        plan = steps["plan"]["run"]
        self.assertIn('[ "$INPUT_ENVIRONMENT" = production ] && [ "$GITHUB_REF" != refs/heads/main ]', plan)
        self.assertLess(plan.index("refs/heads/main"), plan.index("git fetch"), "check the ref before anything else")
        preflight = steps["preflight"]
        self.assertEqual(preflight["env"]["SHA"], "${{ steps.plan.outputs.sha }}")
        self.assertIn("EXPECTED_GIT_REVISION='${SHA}'", preflight["run"])
        self.assertLess(preflight["run"].index('[[ "$SHA" =~ ^[0-9a-f]{40}$ ]]'), preflight["run"].index("remote="))
        recovery = next(step["run"] for step in workflow["jobs"]["deploy"]["steps"]
                        if "queue_discarded=" in step.get("run", ""))
        self.assertIn('queue_discarded="$(jq -r \'.job_queue_changes_discarded // false\' rollback.json)"', recovery)
        discarded = recovery.index('if [ "$queue_discarded" = "true" ]; then')
        self.assertLess(discarded, recovery.index("job-queue changes were discarded"))
        reverted = recovery.index('if [ "$action" = "migration-reverted" ]; then')
        self.assertNotIn("discarded", recovery[reverted:discarded])
        rust = yaml_json((ROOT / ".github/workflows/rust.yml").read_text())
        port = [job for job in rust["jobs"].values() if job.get("name") == "Rust port"]
        self.assertEqual(len(port), 1, "rust.yml must keep exactly one aggregate job named 'Rust port'")
        self.assertIn("always()", port[0]["if"])
        print("WORKFLOW GATE: rust.yml push/schedule success with its 'Rust port' aggregate is the single gate")

    def test_image_workflow_publishes_from_main_only_with_pinned_actions(self):
        image = yaml_json((ROOT / ".github/workflows/publish-image.yml").read_text())
        on = image.get("on", image.get("true"))
        self.assertEqual(set(on), {"push", "workflow_dispatch"})
        self.assertEqual(on["push"]["branches"], ["main"])
        self.assertIs(on["workflow_dispatch"]["inputs"]["dry_run"]["default"], True)
        # Publishing runs are never cancelled, and a dry run has a group of its own.
        self.assertIs(image["concurrency"]["cancel-in-progress"], False)
        self.assertIn("github.sha", image["concurrency"]["group"])
        self.assertIn("format('dry-run-{0}', github.run_id)", image["concurrency"]["group"])
        # The deployable tag comes from the amd64 job alone: no needs, no GHCR.
        amd64 = image["jobs"]["amd64"]
        self.assertNotIn("needs", amd64)
        self.assertNotIn("packages", amd64["permissions"])
        plan = next(step["run"] for step in amd64["steps"] if step.get("id") == "plan")
        self.assertIn('echo "tag=rust-git-${SHA}"', plan)
        self.assertIn('[ "$PUBLISH" = true ] && [ "$REF" = refs/heads/main ]', plan)
        build = next(step["with"] for step in amd64["steps"]
                     if step.get("uses", "").startswith("docker/build-push-action@"))
        self.assertEqual((build["context"], build["file"], build["platforms"]),
                         ("rust", "rust/Dockerfile", "linux/amd64"))
        self.assertIn("GIT_REVISION=${{ github.sha }}", build["build-args"])
        self.assertFalse(build["provenance"])
        self.assertEqual(sorted(image["jobs"]["ghcr"]["needs"]), ["amd64", "arm64"])
        for workflow in [image, yaml_json((ROOT / ".github/workflows/deploy-gcp.yml").read_text())]:
            self.assertGreater(audit_actions(workflow), 0)
        print("WORKFLOW ACTIONS: all third-party actions pinned; rust-git-<sha> comes from the amd64 job alone, on main")

    def test_action_audit_rejects_unpinned_references(self):
        with self.assertRaises(ValueError):
            audit_actions({"jobs": {"bad": {"steps": [{"uses": "actions/checkout@main"}]}}})


if __name__ == "__main__":
    unittest.main()
