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
            print(json.dumps({"host":"fixture.invalid", "image":os.environ["IMAGE_REF"],
                "env":{"SECRET_KEY_BASE":"fixture"}, "autoUpdate":False}))
        else:
            sys.exit("unhandled inspect " + fmt)
    elif args[:2] == ["image", "inspect"]:
        if "--format" not in args:
            sys.exit(0)
        fmt = args[args.index("--format") + 1]
        if '"net.smartdata.campfire.runtime"' in fmt:
            print(os.environ.get("RUNTIME", ""))
        else:
            sys.exit("unhandled image inspect " + fmt)
    elif args[0] == "top":
        print("PID COMMAND")
        print("100 " + os.environ.get("PROCESSES", "puma resque-pool"))
    elif args[0] == "run":
        if os.environ.get("RUN_STATUS"):
            sys.exit(int(os.environ["RUN_STATUS"]))
        print("MATCH: 88 preexisting tables preserved")
        print("ADDITIVE: 0 tables, 0 columns")
        root = pathlib.Path(os.environ["STATE_ROOT"]) / "campfire-fixture"
        (root / "rehearsal/rehearsal-after.sqlite3").write_bytes(b"fixture")
    elif args[0] not in ["rm", "exec"]:
        sys.exit("unhandled docker " + repr(args))
elif name == "curl":
    print("200", end="")
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
        (state / "preflight-result.json").write_text(json.dumps(before))
        (state / "freeze-result.json").write_text(json.dumps({"previous_image": PREVIOUS}))
        (state / "before-settings.json").write_text(json.dumps(before))
        (state / "attachment-hashes-before.json").write_text("{}")
        source_file = work / "release.sh"
        assert source.rstrip().endswith('main "$@"')
        source_file.write_text(source.rsplit('main "$@"', 1)[0])
        bin_dir = work / "bin"
        bin_dir.mkdir()
        for name in ["docker", "gcloud", "once", "curl"]:
            fake = bin_dir / name
            fake.write_text(FAKE)
            fake.chmod(0o755)
        trace = work / "trace.jsonl"
        env = {**os.environ, "PATH": f"{bin_dir}:{os.environ['PATH']}",
               "IMAGE_REF": CANDIDATE, "RELEASE_LABEL": "fixture", "STATE_ROOT": str(work),
               "TRACE": str(trace), **overrides}
        shell = 'source "$1"; require_label; "$2"'
        result = subprocess.run(["bash", "-c", shell, "harness", str(source_file), decision],
                                env=env, text=True, capture_output=True)
        commands = trace.read_text() if trace.exists() else ""
        commands = json.loads("[" + ",".join(commands.splitlines()) + "]")
        commands = json.loads(json.dumps(commands).replace(str(work), "<scratch>"))
        return result, commands


class ReleaseDecisionsTest(unittest.TestCase):
    def test_rails_trace_matches_committed_origin_baseline(self):
        source = SCRIPT.read_text()
        baseline = json.loads(BASELINE.read_text())
        for decision, expected in baseline["decisions"].items():
            with self.subTest(decision=decision):
                result, trace = run_decision(source, decision)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(trace, expected)

    def test_rehearsal_failure_refuses_candidate(self):
        result, _ = run_decision(SCRIPT.read_text(), "rehearse_migration", RUN_STATUS="7")
        self.assertEqual(result.returncode, 1)
        self.assertIn("refusing to cut over", result.stderr)

    def test_missing_process_fails_after_health(self):
        result, _ = run_decision(SCRIPT.read_text(), "phase_cutover", PROCESSES="unrelated")
        self.assertEqual(result.returncode, 20)
        self.assertIn("read-only check(s) failed", result.stderr)


if __name__ == "__main__":
    import sys
    if len(sys.argv) == 3 and sys.argv[1] == "--record":
        revision = sys.argv[2]
        source = subprocess.check_output(["git", "show", f"{revision}:deploy/gcp/campfire-release.sh"],
                                         cwd=ROOT, text=True)
        decisions = {}
        for decision in ["rehearse_migration", "phase_cutover"]:
            result, trace = run_decision(source, decision)
            if result.returncode:
                raise RuntimeError(result.stdout + result.stderr)
            decisions[decision] = trace
        sha = subprocess.check_output(["git", "rev-parse", revision], cwd=ROOT, text=True).strip()
        BASELINE.write_text(json.dumps({"source_sha": sha, "decisions": decisions}, indent=2) + "\n")
        print(f"Recorded Rails release baseline from {sha}")
    else:
        unittest.main()
