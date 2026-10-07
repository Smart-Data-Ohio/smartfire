import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

from ignored_tests import ROOT, expression, verify_junit
from nextest_archive import run as run_archive


class CorrectnessArchive(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory()
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)
        for directory in ("ci", ".github/workflows", "parity", "bin", "target/debug",
                          "web/bin", "web/.bundle/livekit"):
            (self.root / directory).mkdir(parents=True)
        for name in ("correctness.sh", "ignored_tests.py", "nextest_archive.py"):
            shutil.copyfile(ROOT / "ci" / name, self.root / "ci" / name)
        records = {suite: [{"path": "tests.rs", "package": "campfire", "binary": "campfire",
                            "test": f"{suite}_case"}]
                   for suite in ("browsers", "drive", "livekit")}
        (self.root / "ci/ignored-tests.json").write_text(json.dumps(records))
        (self.root / "tests.rs").write_text("\n".join(
            f'#[test] #[ignore = "browser prerequisite"] fn {suite}_case() {{}}'
            for suite in records))
        (self.root / ".github/workflows/rust.yml").write_text(
            'suite: [browsers, drive, livekit]\nbash ci/correctness.sh "$SUITE"\n')
        for name in ("Dockerfile.playwright", "package.json", "package-lock.json"):
            (self.root / "parity" / name).write_text("pinned input\n")
        (self.root / "web/.bundle/livekit/env").write_text("")
        self.executable("web/bin/livekit-local", '#!/bin/sh\n[ "$1" != start ] || exec sleep 30\n')
        self.executable("bin/curl", "#!/bin/sh\nexit 0\n")
        self.executable("bin/docker", "#!/bin/sh\nexit 0\n")
        self.executable("bin/cargo", '''#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
root = Path.cwd()
args = sys.argv[1:]
with (root / "cargo-calls.jsonl").open("a") as calls:
    calls.write(json.dumps(args) + "\\n")
if args[:2] != ["nextest", "run"]:
    if os.environ.get("CORRECTNESS_ARCHIVE"):
        raise SystemExit("archive consumer attempted to build")
    raise SystemExit(0)
if "--archive-file" in args:
    archive = Path(args[args.index("--archive-file") + 1])
    if archive.read_text() != "valid archive":
        raise SystemExit("invalid archive")
    assert Path(args[args.index("--extract-to") + 1]).is_dir()
    assert args[args.index("--workspace-remap") + 1] == str(root)
    assert not {"--locked", "-p", "--build-jobs"}.intersection(args)
else:
    assert not os.environ.get("CORRECTNESS_ARCHIVE"), "archive consumer fell back to a build"
    assert {"--locked", "-p", "--build-jobs"}.issubset(args)
assert args[args.index("--profile") + 1] == "ci"
assert args[args.index("-j") + 1] == "4"
assert "--no-fail-fast" in args
assert args[args.index("--success-output") + 1] == "final"
assert args[args.index("--run-ignored") + 1] == "only"
assert args[args.index("--no-tests") + 1] == "fail"
name = os.environ["PROBE_SUITE"] + "_case"
assert "test(=" + name + ")" in args[args.index("-E") + 1]
receipt = root / "target/nextest/ci/junit.xml"
receipt.parent.mkdir(parents=True, exist_ok=True)
body = "<skipped/>" if os.environ.get("PROBE_SKIP") else ""
receipt.write_text(f'<testsuites><testsuite><testcase name="{name}">{body}</testcase></testsuite></testsuites>')
''')
        self.archive = self.root / "target/correctness-tests.tar.zst"
        self.archive.write_text("valid archive")
        self.executable("target/debug/campfire", "#!/bin/sh\nexit 0\n")

    def executable(self, name, source):
        path = self.root / name
        path.write_text(source)
        path.chmod(0o755)

    def run_suite(self, suite, archive=True, skip=False):
        calls = self.root / "cargo-calls.jsonl"
        calls.unlink(missing_ok=True)
        env = dict(os.environ, PATH=f"{self.root / 'bin'}:{os.environ['PATH']}",
                   PROBE_SUITE=suite, TMPDIR=str(self.root))
        env.pop("CORRECTNESS_ARCHIVE", None)
        env.pop("CORRECTNESS_SHARD", None)
        env.pop("CI_SOURCE_SHA", None)
        if archive:
            env["CORRECTNESS_ARCHIVE"] = str(self.archive)
        if skip:
            env["PROBE_SKIP"] = "1"
        result = subprocess.run(["bash", "ci/correctness.sh", suite, "--execute"],
                                cwd=self.root, env=env, text=True, capture_output=True)
        commands = [json.loads(line) for line in calls.read_text().splitlines()] if calls.exists() else []
        return result, commands

    def test_archive_consumers_run_selected_tests_without_building(self):
        for suite in ("browsers", "drive", "livekit"):
            with self.subTest(suite=suite):
                result, commands = self.run_suite(suite)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(len(commands), 1)
                self.assertTrue((self.root / f"target/ci-receipts/{suite}-junit.xml").is_file())

    def test_missing_empty_and_invalid_archives_fail_without_build_fallback(self):
        for contents in (None, "", "invalid archive"):
            with self.subTest(contents=contents):
                self.archive.unlink(missing_ok=True)
                if contents is not None:
                    self.archive.write_text(contents)
                result, commands = self.run_suite("drive")
                self.assertNotEqual(result.returncode, 0)
                self.assertTrue(all(command[:2] == ["nextest", "run"] for command in commands))

    def test_browser_archive_requires_the_prebuilt_server(self):
        (self.root / "target/debug/campfire").unlink()
        result, commands = self.run_suite("browsers")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Prebuilt browser server is missing", result.stderr)
        self.assertEqual(commands, [])

    def test_archive_cannot_accept_a_skipped_selected_test(self):
        result, _ = self.run_suite("drive", skip=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("executed []", result.stderr)

    def test_local_suites_still_build(self):
        for suite, build_command in (("browsers", "build"), ("livekit", "test")):
            with self.subTest(suite=suite):
                result, commands = self.run_suite(suite, archive=False)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(commands[0][0], build_command)
                self.assertEqual(commands[1][:2], ["nextest", "run"])


class RealNextestArchive(unittest.TestCase):
    def test_round_trip_runs_relocated_binary_and_reads_compile_time_fixture(self):
        missing = [tool for tool in ("cargo", "cargo-nextest") if shutil.which(tool) is None]
        if missing:
            message = f"Real nextest archive test requires {', '.join(missing)}"
            self.assertNotEqual(os.environ.get("CI_REQUIRE_NEXTEST_ARCHIVE_TEST"), "1", message)
            self.skipTest(message)
        with tempfile.TemporaryDirectory(prefix="nextest-archive-") as scratch:
            scratch = Path(scratch)
            repo = scratch / "workspace"
            for directory in ("src", ".config", "fixtures"):
                (repo / directory).mkdir(parents=True)
            (repo / "Cargo.toml").write_text('''[package]
name = "archive_smoke"
version = "0.1.0"
edition = "2021"
[workspace]
''')
            (repo / ".config/nextest.toml").write_text('''[profile.ci]
fail-fast = false
retries = 0
[profile.ci.junit]
path = "junit.xml"
report-skipped = "ignored"
''')
            fixture = repo / "fixtures/same-checkout.txt"
            fixture.write_text("fixture from the compiled checkout\n")
            (repo / "src/lib.rs").write_text('''#[test]
#[ignore = "utility: nextest archive round trip"]
fn archive_fixture_smoke() {
    let fixture = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/same-checkout.txt"));
    let contents = std::fs::read_to_string(fixture).expect("compile-time fixture path must survive extraction");
    assert_eq!(contents, "fixture from the compiled checkout\\n");
    let executable = std::env::current_exe().unwrap();
    let marker = std::env::var("NEXTEST_ARCHIVE_SMOKE_MARKER").unwrap();
    std::fs::write(marker, format!("{}\\n{}\\n{}", executable.display(), fixture.display(), contents)).unwrap();
}

#[test]
fn ordinary_test_is_not_selected() {
    panic!("archive runner must select only ignored tests");
}

#[test]
#[ignore = "utility: nextest archive filter decoy"]
fn archive_fixture_smoke_decoy() {
    panic!("archive runner must select the exact test name");
}
''')
            original_target = repo / "target"
            marker = scratch / "executed.txt"
            archive = scratch / "tests.tar.zst"
            env = dict(os.environ, CARGO_TARGET_DIR=str(original_target),
                       NEXTEST_ARCHIVE_SMOKE_MARKER=str(marker))
            # CI supplies the producer's stable toolchain; local runs avoid the
            # repository's nightly selection without changing the checkout.
            env.setdefault("RUSTUP_TOOLCHAIN", "stable")
            manifest = str(repo / "Cargo.toml")
            for command in (
                ["cargo", "generate-lockfile", "--offline", "--manifest-path", manifest],
                ["cargo", "nextest", "archive", "--locked", "--profile", "ci",
                 "--build-jobs", "4", "--manifest-path", manifest, "--archive-file", str(archive)],
            ):
                # Cargo discovers config from its working directory, even with
                # --manifest-path. Keep a TMPDIR inside this checkout from
                # inheriting its developer-only nightly/Cranelift settings.
                result = subprocess.run(command, cwd=repo.anchor, env=env, text=True, capture_output=True)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertGreater(archive.stat().st_size, 0)
            self.assertTrue(any((original_target / "debug/deps").glob("archive_smoke-*")))
            shutil.rmtree(original_target)
            self.assertFalse(original_target.exists())

            records = [{"package": "archive_smoke", "binary": "archive_smoke",
                        "test": "archive_fixture_smoke"}]
            run_archive(archive, repo, expression(records), env=env)
            executable_path, fixture_path, contents = marker.read_text().splitlines()
            executable = Path(executable_path)
            self.assertTrue(executable.is_relative_to(repo / "target/correctness-archive"), executable)
            self.assertTrue(executable.is_file())
            self.assertFalse((original_target / "debug").exists(), "archive consumer rebuilt the original target")
            self.assertEqual(fixture_path, str(fixture))
            self.assertEqual(contents, "fixture from the compiled checkout")
            receipt = repo / "target/nextest/ci/junit.xml"
            verify_junit(records, receipt)
            print(f"Real nextest archive round trip: relocated executable={executable}; fixture={fixture}; receipt={receipt}")
