"""Local release decision harness. No real Docker, ONCE, or cloud command runs."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "deploy/gcp/campfire-release.sh"
BASELINE = Path(__file__).with_name("rails-release-baseline.json")
CANDIDATE = "fixture/image@sha256:" + "1" * 64
PREVIOUS = "fixture/image@sha256:" + "2" * 64

FAKE = r'''#!/usr/bin/env python3
import json, os, pathlib, sys
name = pathlib.Path(sys.argv[0]).name
args = sys.argv[1:]
if name == "id":
    print("0")
    sys.exit(0)
with open(os.environ["TRACE"], "a") as f:
    f.write(json.dumps([name, *args]) + "\n")
if name == "gcloud":
    sys.exit("cloud commands are forbidden in this harness")
if name == "docker":
    if args[0] == "ps":
        print("once-app-fixture")
    elif args[0] == "inspect":
        fmt = args[args.index("--format") + 1]
        if ".State.Running" in fmt:
            print("true")
        elif ".Mounts" in fmt:
            print("fixture-volume")
        elif '"once"' in fmt:
            print(json.dumps({"host":"fixture.invalid", "image":os.environ.get("SETTINGS_IMAGE", os.environ["IMAGE_REF"]),
                "env":{"SECRET_KEY_BASE":"fixture"}, "autoUpdate":False}))
        else:
            sys.exit("unhandled inspect " + fmt)
    elif args[:2] == ["image", "inspect"]:
        if "--format" not in args:
            sys.exit(0)
        fmt = args[args.index("--format") + 1]
        if '"net.smartdata.campfire.runtime"' in fmt:
            counter = pathlib.Path(os.environ["STATE_ROOT"]) / "runtime-inspected"
            if os.environ.get("FAIL_FIRST_RUNTIME_INSPECT") and not counter.exists():
                counter.touch()
                sys.exit("injected one-shot image-inspect failure")
            print(os.environ.get("PREVIOUS_RUNTIME", "") if "2" * 64 in args[2] else os.environ.get("RUNTIME", ""))
        elif fmt == "{{.Architecture}}":
            print("amd64")
        elif fmt == "{{.Os}}":
            print("linux")
        elif fmt == "{{.Size}}":
            print("10485760")
        elif ".Config.Env" in fmt:
            print("GIT_REVISION=fixture")
        else:
            sys.exit("unhandled image inspect " + fmt)
    elif args[:2] == ["volume", "inspect"]:
        print(pathlib.Path(os.environ["STATE_ROOT"]) / "volume")
    elif args[0] == "top":
        print("PID COMMAND")
        print("100 " + os.environ.get("PROCESSES", "puma resque-pool"))
    elif args[0] == "run":
        if os.environ.get("ROLLBACK_STATUS") and "2" * 64 in " ".join(args):
            sys.exit(int(os.environ["ROLLBACK_STATUS"]))
        if os.environ.get("RUN_STATUS"):
            sys.exit(int(os.environ["RUN_STATUS"]))
        print("MATCH: 88 preexisting tables preserved")
        print("ADDITIVE: 0 tables, 0 columns")
        root = pathlib.Path(os.environ["STATE_ROOT"]) / "campfire-fixture"
        (root / "rehearsal/rehearsal-after.sqlite3").write_bytes(b"fixture")
    elif args[0] not in ["rm", "exec", "login", "pull"]:
        sys.exit("unhandled docker " + repr(args))
elif name == "curl":
    print("200", end="")
elif name == "df":
    print("Filesystem 1M-blocks Used Available Use% Mounted on")
    print("fixture 100000 100 99900 1% /")
elif name == "du":
    print("1 " + args[-1])
elif name == "pgrep":
    sys.exit(1)
elif name == "systemctl":
    print("disabled" if args[0] == "is-enabled" else "inactive")
    sys.exit(1)
elif name != "once":
    sys.exit("unhandled fake " + name)
'''


def run_decision(source, decision, **overrides):
    scratch = ROOT / ".scratch"
    scratch.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="release-", dir=scratch) as tmp:
        work = Path(tmp)
        state = work / "campfire-fixture"
        state.mkdir()
        volume = work / "volume"
        (volume / "files").mkdir(parents=True)
        (state / "before.sqlite3").write_bytes(b"fixture")
        before = {"app_host": "fixture.invalid", "volume": "fixture-volume",
                  "volume_mountpoint": str(volume), "current_image": PREVIOUS,
                  "envKeys": ["SECRET_KEY_BASE"]}
        preflight = {**before, "target_image": CANDIDATE}
        if not overrides.pop("LEGACY_RECORD", False):
            preflight.update(target_runtime=overrides.pop("RECORDED_RUNTIME", "rails"),
                             current_runtime=overrides.pop("RECORDED_PREVIOUS_RUNTIME", "rails"))
        record = state / "preflight-result.json"
        if decision != "preflight":
            record.write_text(json.dumps(preflight))
        (state / "freeze-result.json").write_text(json.dumps({"previous_image": PREVIOUS}))
        (state / "before-settings.json").write_text(json.dumps(before))
        (state / "attachment-hashes-before.json").write_text("{}")
        source_file = work / "release.sh"
        assert source.rstrip().endswith('main "$@"')
        # Keep the real main invocation and dispatcher. The recorded rehearsal
        # decision isolates the helper from the rest of freeze's host mutations.
        phase = {"rehearse_migration": "freeze", "phase_cutover": "cutover"}.get(decision, decision)
        setup = ''
        if decision == "rehearse_migration":
            setup = 'phase_freeze() { rehearse_migration; }\n'
        elif decision.startswith("dispatcher_"):
            phase = decision.removeprefix("dispatcher_")
            setup = 'run_phase() { printf "PHASE=%s RUNTIME=%s\\n" "$1" "${CANDIDATE_RUNTIME:-rails}"; }\n'
        source_file.write_text(source.replace('\nmain "$@"', '\n' + setup + 'main "$@"'))
        bin_dir = work / "bin"
        bin_dir.mkdir()
        for name in ["docker", "gcloud", "once", "curl", "id", "df", "du", "pgrep", "systemctl"]:
            fake = bin_dir / name
            fake.write_text(FAKE)
            fake.chmod(0o755)
        trace = work / "trace.jsonl"
        env = {**os.environ, "PATH": f"{bin_dir}:{os.environ['PATH']}",
               "IMAGE_REF": CANDIDATE, "RELEASE_LABEL": "fixture", "STATE_ROOT": str(work),
               "LOCK_FILE": str(work / "release.lock"), "ALLOW_BACKUP_WINDOW": "1",
               "TRACE": str(trace), **overrides}
        result = subprocess.run(["bash", str(source_file), phase],
                                env=env, input="fixture\n", text=True, capture_output=True)
        commands = trace.read_text() if trace.exists() else ""
        commands = json.loads("[" + ",".join(commands.splitlines()) + "]")
        commands = json.loads(json.dumps(commands).replace(str(work), "<scratch>"))
        return result, commands, json.loads(record.read_text()) if record.exists() else None


class ReleaseDecisionsTest(unittest.TestCase):
    def test_rails_trace_matches_committed_origin_baseline(self):
        source = SCRIPT.read_text()
        baseline = json.loads(BASELINE.read_text())
        for decision, expected in baseline["decisions"].items():
            with self.subTest(decision=decision):
                result, trace, _ = run_decision(source, decision)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(trace, expected)

    def test_original_rails_script_matches_baseline_with_main_invoked(self):
        baseline = json.loads(BASELINE.read_text())
        source = subprocess.check_output(
            ["git", "show", baseline["source_sha"] + ":deploy/gcp/campfire-release.sh"],
            cwd=ROOT, text=True)
        for decision, expected in baseline["decisions"].items():
            with self.subTest(decision=decision):
                result, trace, _ = run_decision(source, decision)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(trace, expected)

    def test_rehearsal_failure_refuses_candidate(self):
        result, _, _ = run_decision(SCRIPT.read_text(), "rehearse_migration", RUN_STATUS="7")
        self.assertEqual(result.returncode, 1)
        self.assertIn("refusing to cut over", result.stderr)

    def test_missing_process_fails_after_health(self):
        result, _, _ = run_decision(SCRIPT.read_text(), "phase_cutover", PROCESSES="unrelated")
        self.assertEqual(result.returncode, 20)
        self.assertIn("read-only check(s) failed", result.stderr)

    def test_rust_rehearsal_is_readonly_and_checks_previous_rails_boot(self):
        result, trace, _ = run_decision(SCRIPT.read_text(), "rehearse_migration", RECORDED_RUNTIME="rust")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        runs = [args for args in trace if args[:2] == ["docker", "run"]]
        self.assertEqual(len(runs), 2)
        self.assertIn("<scratch>/campfire-fixture/rehearsal:/rails/storage:ro", runs[0])
        self.assertIn("db-check", runs[0])
        for args in runs:
            self.assertIn("768m", args)
            self.assertIn("none", args)
        self.assertIn(PREVIOUS, runs[1])
        self.assertIn("bin/start-app", runs[1][-1])
        self.assertIn("http://127.0.0.1:3000/up", runs[1][-1])
        self.assertNotIn("bin/rails db:migrate", json.dumps(runs))

    def test_rust_cutover_checks_the_single_server_process(self):
        result, trace, _ = run_decision(SCRIPT.read_text(), "phase_cutover", RECORDED_RUNTIME="rust", PROCESSES="/usr/local/bin/campfire server")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("Rust server present", result.stdout)
        self.assertNotIn("resque-pool", result.stdout)
        self.assertIn(["docker", "top", "once-app-fixture", "-eo", "pid,args"], trace)

    def test_unknown_runtime_label_is_refused(self):
        result, _, record = run_decision(SCRIPT.read_text(), "preflight", RUNTIME="unknown")
        self.assertEqual(result.returncode, 1)
        self.assertIsNone(record)

    def test_unlabelled_image_selects_rails(self):
        result, _, record = run_decision(SCRIPT.read_text(), "preflight")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(record["target_runtime"], "rails")

    def test_transient_inspection_failure_cannot_abort_later_dispatch(self):
        # Review reproduction: the first label inspect fails, later ones would
        # succeed. No label inspection belongs to either phase after preflight.
        for decision in ["dispatcher_freeze", "dispatcher_cutover"]:
            for runtime in ["rails", "rust"]:
                with self.subTest(decision=decision, runtime=runtime):
                    result, trace, _ = run_decision(
                        SCRIPT.read_text(), decision, RECORDED_RUNTIME=runtime,
                        FAIL_FIRST_RUNTIME_INSPECT="1")
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    self.assertIn(f"RUNTIME={runtime}", result.stdout)
                    self.assertEqual(trace, [])

    def test_legacy_preflight_record_means_rails_without_inspection(self):
        baseline = json.loads(BASELINE.read_text())
        for decision, expected in baseline["decisions"].items():
            with self.subTest(decision=decision):
                result, trace, record = run_decision(
                    SCRIPT.read_text(), decision, LEGACY_RECORD=True,
                    RUNTIME="rust", FAIL_FIRST_RUNTIME_INSPECT="1")
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertNotIn("target_runtime", record)
                self.assertEqual(trace, expected)

    def test_preflight_records_each_verified_runtime_once(self):
        for label, runtime in [("", "rails"), ("rails", "rails"), ("rust", "rust")]:
            with self.subTest(label=label):
                result, trace, record = run_decision(
                    SCRIPT.read_text(), "preflight", RUNTIME=label, SETTINGS_IMAGE=PREVIOUS)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(record["target_runtime"], runtime)
                self.assertEqual(record["current_runtime"], "rails")
                inspections = [args for args in trace if args[:3] == ["docker", "image", "inspect"]
                               and "net.smartdata.campfire.runtime" in " ".join(args)]
                self.assertEqual([args[3] for args in inspections], [CANDIDATE, PREVIOUS])

    def test_preflight_inspection_failure_writes_no_verified_record(self):
        result, trace, record = run_decision(
            SCRIPT.read_text(), "preflight", FAIL_FIRST_RUNTIME_INSPECT="1")
        self.assertEqual(result.returncode, 1)
        self.assertIn("injected one-shot image-inspect failure", result.stderr)
        self.assertIsNone(record)
        self.assertFalse(any(args[:2] == ["once", "stop"] for args in trace))

    def test_rust_later_phases_never_reinspect_images(self):
        for decision in ["rehearse_migration", "phase_cutover"]:
            result, trace, _ = run_decision(
                SCRIPT.read_text(), decision, RECORDED_RUNTIME="rust",
                FAIL_FIRST_RUNTIME_INSPECT="1", PROCESSES="/usr/local/bin/campfire server")
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertFalse(any(args[:3] == ["docker", "image", "inspect"] for args in trace))

    def test_rust_rehearsal_requires_recorded_previous_rails_runtime(self):
        result, trace, _ = run_decision(
            SCRIPT.read_text(), "rehearse_migration", RECORDED_RUNTIME="rust",
            RECORDED_PREVIOUS_RUNTIME="rust")
        self.assertEqual(result.returncode, 1)
        self.assertIn("requires a previous Rails image", result.stderr)
        self.assertFalse(any(args[:2] == ["docker", "run"] for args in trace))

    def test_invalid_recorded_runtime_is_not_a_legacy_record(self):
        for runtime in [None, "", "unknown"]:
            with self.subTest(runtime=runtime):
                result, trace, _ = run_decision(
                    SCRIPT.read_text(), "dispatcher_cutover", RECORDED_RUNTIME=runtime)
                self.assertEqual(result.returncode, 1)
                self.assertEqual(trace, [])

    def test_rust_schema_failure_and_rails_rollback_failure_refuse_cutover(self):
        for failure in [{"RUN_STATUS": "7"}, {"ROLLBACK_STATUS": "8"}]:
            result, _, _ = run_decision(SCRIPT.read_text(), "rehearse_migration", RECORDED_RUNTIME="rust", **failure)
            self.assertEqual(result.returncode, 1)
            self.assertIn("refusing to cut over", result.stderr)


if __name__ == "__main__":
    import sys
    if len(sys.argv) == 3 and sys.argv[1] == "--record":
        revision = sys.argv[2]
        source = subprocess.check_output(["git", "show", f"{revision}:deploy/gcp/campfire-release.sh"],
                                         cwd=ROOT, text=True)
        decisions = {}
        for decision in ["rehearse_migration", "phase_cutover"]:
            result, trace, _ = run_decision(source, decision)
            if result.returncode:
                raise RuntimeError(result.stdout + result.stderr)
            decisions[decision] = trace
        sha = subprocess.check_output(["git", "rev-parse", revision], cwd=ROOT, text=True).strip()
        BASELINE.write_text(json.dumps({"source_sha": sha, "decisions": decisions}, indent=2) + "\n")
        print(f"Recorded Rails release baseline from {sha}")
    else:
        unittest.main()
