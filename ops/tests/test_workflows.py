"""Workflow contracts and local execution of the release-plan shell step."""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[2]
REVISION = "a2fbe296f0675b1a657cf81c1f537b6687451403"


def yaml_json(text):
    """The workflow as JSON-shaped data, from PyYAML when it's installed and otherwise from Ruby's
    YAML (the runner's ruby, or mise's). Both read YAML 1.1, so `on:` is the key "true"."""
    try:
        import yaml
    except ImportError:
        ruby = ["ruby"] if shutil.which("ruby") else ["mise", "exec", "ruby@3.4.10", "--", "ruby"]
        return json.loads(subprocess.check_output([*ruby, "-ryaml", "-rjson",
            "-e", "puts JSON.generate(YAML.safe_load(STDIN.read))"], input=text, text=True))
    return json.loads(json.dumps(yaml.safe_load(text)))


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
    step = next(step for step in document["jobs"].get("prepare", document["jobs"]["deploy"])["steps"] if step.get("id") == "plan")
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
        self.assertNotIn("INPUT_RUNTIME", next(step for step in current["jobs"]["prepare"]["steps"] if step.get("id") == "plan")["env"])
        print("WORKFLOW PLAN: only the Rust full-SHA tag; no runtime input")

    def test_release_is_gated_on_the_rust_checks_alone(self):
        workflow = yaml_json((ROOT / ".github/workflows/deploy-gcp.yml").read_text())
        steps = {step["id"]: step for step in workflow["jobs"]["deploy"]["steps"] if "id" in step}
        preparation = {step["id"]: step for step in workflow["jobs"]["prepare"]["steps"] if "id" in step}
        gate = preparation["ci"]["run"]
        self.assertIn("rust.yml", gate)
        self.assertIn("push", gate)
        self.assertIn("schedule", gate)
        self.assertNotIn("ci.yml", (ROOT / ".github/workflows/deploy-gcp.yml").read_text())
        self.assertIn('select(.name == "Rust port")', gate)
        plan = preparation["plan"]["run"]
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

    def test_spa_mode_uses_the_verified_cutover_or_a_read_only_dry_run(self):
        workflow = yaml_json((ROOT / ".github/workflows/deploy-gcp.yml").read_text())
        inputs = workflow.get("on", workflow.get("true"))["workflow_dispatch"]["inputs"]
        self.assertEqual(inputs["spa_mode"]["type"], "choice")
        self.assertEqual(inputs["spa_mode"]["default"], "unchanged")
        self.assertEqual(inputs["spa_mode"]["options"], ["unchanged", "off", "opt-in", "default-next"])
        steps = workflow["jobs"]["deploy"]["steps"]
        by_id = {step["id"]: step for step in steps if "id" in step}
        spa = by_id["spa_configuration"]
        self.assertEqual(spa["if"], "inputs.spa_mode != 'unchanged' && (steps.plan.outputs.dry_run == 'true' || steps.cutover.conclusion == 'success')")
        self.assertEqual(spa["env"], {
            "SPA_MODE": "${{ inputs.spa_mode }}", "DRY_RUN": "${{ steps.plan.outputs.dry_run }}",
            "EXPECTED_REVISION": "${{ steps.plan.outputs.sha }}", "EXPECTED_IMAGE": "${{ steps.image.outputs.reference }}",
        })
        self.assertIn("set -euo pipefail", spa["run"])
        self.assertIn("python3 deploy/gcp/spa-configuration.py", spa["run"])
        self.assertIn("sudo python3 /opt/campfire-deploy/configure-spa.py", spa["run"])
        self.assertLess(steps.index(by_id["cutover"]), steps.index(by_id["google_configuration"]))
        self.assertLess(steps.index(by_id["google_configuration"]), steps.index(spa))
        self.assertLess(steps.index(spa), steps.index(by_id["finish"]))
        # Everyone gets the SPA whatever spa_mode wrote: its shell is checked after every applied
        # release, whichever mode ran (unchanged and off included).
        check = by_id["spa_check"]
        self.assertEqual(check["if"], "steps.plan.outputs.dry_run != 'true' && steps.cutover.conclusion == 'success'")
        self.assertNotIn("spa_mode", check["if"])
        self.assertIn('python3 deploy/gcp/check-frontend.py "https://${GCP_APP_HOST}" --spa', check["run"])
        self.assertLess(steps.index(spa), steps.index(check))
        self.assertLess(steps.index(check), steps.index(by_id["finish"]))
        validation = next(step for step in steps if step.get("run") == "python3 deploy/gcp/spa-configuration.py --validate")
        self.assertNotIn("if", validation)
        self.assertLess(steps.index(validation), steps.index(by_id["preflight"]))
        for name in ["configure-google.py", "configure-spa.py", "once_configuration.py", "check-frontend.py"]:
            self.assertIn(f"/opt/campfire-deploy/{name}", by_id["copy"]["run"])
        # Current-image settings runs retain the ordinary release and recovery conditions.
        self.assertEqual(by_id["freeze"]["if"], "steps.plan.outputs.dry_run != 'true'")
        self.assertEqual(by_id["cutover"]["if"], "steps.plan.outputs.dry_run != 'true'")
        self.assertEqual(by_id["finish"]["if"], "success() && steps.plan.outputs.dry_run != 'true'")
        self.assertIn("steps.cutover.outputs.rc != '20'", by_id["rollback"]["if"])

    def test_image_workflow_publishes_from_main_only_with_pinned_actions(self):
        image = yaml_json((ROOT / ".github/workflows/publish-image.yml").read_text())
        on = image.get("on", image.get("true"))
        self.assertEqual(set(on), {"workflow_call", "workflow_dispatch"})
        self.assertTrue(on["workflow_call"]["inputs"]["git_sha"]["required"])
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
        self.assertIn("github.ref == 'refs/heads/main'", image["env"]["PUBLISH"])
        build = next(step["with"] for step in amd64["steps"]
                     if step.get("uses", "").startswith("docker/build-push-action@"))
        self.assertEqual((build["context"], build["file"], build["platforms"]),
                         (".", "Dockerfile", "linux/amd64"))
        self.assertIn("GIT_REVISION=${{ steps.plan.outputs.sha }}", build["build-args"])
        self.assertFalse(build["provenance"])
        self.assertEqual(set(image["jobs"]), {"amd64"})
        checkout = next(step for step in amd64["steps"] if step.get("uses", "").startswith("actions/checkout@"))
        self.assertEqual(checkout["with"]["ref"], "${{ inputs.git_sha || github.sha }}")
        deploy = yaml_json((ROOT / ".github/workflows/deploy-gcp.yml").read_text())["jobs"]
        self.assertEqual(deploy["publish"]["needs"], "prepare")
        self.assertIn("outputs.exists == 'false'", deploy["publish"]["if"])
        self.assertIn("outputs.dry_run != 'true'", deploy["publish"]["if"])
        self.assertEqual(deploy["publish"]["with"]["git_sha"], "${{ needs.prepare.outputs.sha }}")
        self.assertEqual(deploy["deploy"]["needs"], ["prepare", "publish"])
        self.assertIn("needs.publish.result == 'skipped'", deploy["deploy"]["if"])
        for workflow in [image, yaml_json((ROOT / ".github/workflows/deploy-gcp.yml").read_text())]:
            self.assertGreater(audit_actions(workflow), 0)
        print("WORKFLOW ACTIONS: all third-party actions pinned; rust-git-<sha> comes from the amd64 job alone, on main")

    def test_frontend_check_reports_on_every_pull_request(self):
        frontend = yaml_json((ROOT / ".github/workflows/frontend.yml").read_text())
        on = frontend.get("on", frontend.get("true"))
        self.assertEqual(set(on), {"push", "pull_request"})
        for event, trigger in on.items():
            self.assertFalse({"paths", "paths-ignore"} & set(trigger), f"frontend.yml filters {event} by path")
        # The required "Frontend" job always runs: the scope step, not a job condition, skips the
        # work when the PR leaves frontend/ alone, so its result never goes missing. The mock
        # Playwright shards beside it are advisory: named apart from it, and nothing needs them.
        self.assertEqual(set(frontend["jobs"]), {"frontend", "e2e"})
        job = frontend["jobs"]["frontend"]
        self.assertEqual(job["name"], "Frontend")
        self.assertNotIn("if", job)
        self.assertNotIn("needs", job)
        e2e = frontend["jobs"]["e2e"]
        self.assertTrue(e2e["name"].startswith("Frontend e2e ("), e2e["name"])
        self.assertEqual(e2e["if"], "needs.frontend.outputs.run == 'true'")
        self.assertEqual(e2e["strategy"]["matrix"]["shard"], [1, 2, 3])
        self.assertEqual(frontend["permissions"], {})
        self.assertGreater(audit_actions(frontend), 0)
        # The only CI build of crates/spa against a real dist: the job builds one and embeds it.
        steps = [step.get("run", "") for step in job["steps"]]
        build = next(i for i, run in enumerate(steps) if run.strip() == "pnpm build")
        embed = next(i for i, step in enumerate(job["steps"]) if "-p campfire_spa" in step.get("run", ""))
        self.assertGreater(embed, build)
        self.assertEqual(job["steps"][embed]["env"]["SPA_DIST"], "frontend/dist")
        print("WORKFLOW FRONTEND: an always-run 'Frontend' job gating nothing on e2e, no path filter, pinned actions, real-dist crates/spa tests")

    def test_rust_gates_skip_frontend_only_changes(self):
        rust = yaml_json((ROOT / ".github/workflows/rust.yml").read_text())
        script = next(step["run"] for step in rust["jobs"]["changes"]["steps"] if step.get("id") == "scope")
        source = script[script.index("RUST_DIRS ="):script.index("rust = True")]
        scope = {}
        exec(textwrap.dedent(source), scope)
        rust_input = scope["rust_input"]
        for path in [b"frontend/src/main.tsx", b"frontend/pnpm-lock.yaml", b"docs/development.md",
                     b".github/workflows/frontend.yml", b"ops/README.md", b"huddle-gateway/index.mjs",
                     b"web/app/javascript/application.js"]:
            self.assertFalse(rust_input(path), path)
        # frontend/src/gen is generated from crates/api_types; the clippy job checks it.
        for path in [b"crates/kit/src/lib.rs", b"Cargo.lock", b"crates/static_assets/auth/auth.js",
                     b".github/workflows/rust.yml", b"crates/api_types/src/lib.rs",
                     b"frontend/src/gen/MessageDTO.ts"]:
            self.assertTrue(rust_input(path), path)
        print("WORKFLOW RUST SCOPE: frontend/-only pull requests skip the Rust gates")

    def test_action_audit_rejects_unpinned_references(self):
        with self.assertRaises(ValueError):
            audit_actions({"jobs": {"bad": {"steps": [{"uses": "actions/checkout@main"}]}}})


if __name__ == "__main__":
    unittest.main()
