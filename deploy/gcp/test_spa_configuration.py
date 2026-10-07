import copy
import importlib.util
import io
import json
import os
from pathlib import Path
import runpy
import sys
import tempfile
import unittest
from contextlib import nullcontext
from types import SimpleNamespace
from unittest.mock import patch

DIRECTORY = Path(__file__).resolve().parent
sys.path.insert(0, str(DIRECTORY))
import once_configuration

spec = importlib.util.spec_from_file_location("spa_configuration", DIRECTORY / "spa-configuration.py")
producer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(producer)


class SpaConfigurationTest(unittest.TestCase):
    def setUp(self):
        self.image = "example-docker.pkg.dev/example-project/app/web@sha256:" + "a" * 64
        self.environ = {"GCP_APP_HOST": "chat.example.com", "SPA_MODE": "opt-in", "DRY_RUN": "false",
                        "EXPECTED_REVISION": "b" * 40, "EXPECTED_IMAGE": self.image}
        self.existing = {
            "SECRET_KEY_BASE": "PRIVATE-SIGNING-KEY", "VAPID_PRIVATE_KEY": "PRIVATE-VAPID-KEY",
            "VAPID_PUBLIC_KEY": "vapid-public", "LIVEKIT_API_SECRET": "PRIVATE-LIVEKIT-SECRET",
            "GOOGLE_CLIENT_SECRET": "PRIVATE-GOOGLE-SECRET", "EXTRA": "equals=and\nnewlines",
            "WEB_CONCURRENCY": "1", "SPA_ENABLED": "0", "SPA_DEFAULT": "classic",
        }

    def payload(self, **environ):
        return dict(producer.configuration({**self.environ, **environ}),
                    expected_revision="b" * 40, expected_image=self.image)

    def producer_run(self, environ, arguments=()):
        stdout, stderr = io.StringIO(), io.StringIO()
        with patch.dict(os.environ, environ, clear=True), patch("sys.argv", ["spa-configuration.py", *arguments]), \
             patch("sys.stdout", stdout), patch("sys.stderr", stderr):
            code = 0
            try:
                runpy.run_path(str(DIRECTORY / "spa-configuration.py"), run_name="__main__")
            except SystemExit as error:
                code = error.code
        return code, stdout.getvalue(), stderr.getvalue()

    def host_run(self, payload, *, before_change=None, after_change=None, healthy=True, update_code=0,
                 script="configure-spa.py"):
        settings = {"host": "chat.example.com", "image": self.image, "autoUpdate": False,
                    "env": copy.deepcopy(self.existing), "secretKeyBase": "PRIVATE-SIGNING-KEY",
                    "backup": {"path": "/backups", "schedule": "daily"}, "port": 443}
        runtime = {"GIT_REVISION": "b" * 40, "PATH": "/usr/local/bin", **self.existing}
        container = {"Config": {"Labels": {}, "Env": []},
                     "Mounts": [{"Type": "volume", "Source": "/storage", "Destination": "/rails/storage", "RW": True}],
                     "Image": "pinned-image-id"}
        if before_change:
            before_change(settings, runtime, container)

        def serialize():
            container["Config"]["Labels"]["once"] = json.dumps(settings)
            container["Config"]["Env"] = [k + "=" + v for k, v in runtime.items()]

        serialize()
        before = copy.deepcopy(settings)
        commands = []
        scratch = DIRECTORY.parents[1] / ".scratch"
        scratch.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=scratch, prefix="spa-configuration-") as directory:
            temporary = Path(directory)

            def fake_run(args, **kwargs):
                commands.append(args)
                if args[:3] == ["docker", "ps", "-q"]:
                    output = "container-id\n"
                elif args[:2] == ["docker", "inspect"]:
                    output = json.dumps([container])
                elif args[:2] == ["once", "update"]:
                    # The complete old settings must be protected before ONCE can replace them.
                    backups = list(temporary.glob("*/before-settings.json"))
                    self.assertEqual(len(backups), 1)
                    self.assertEqual(json.loads(backups[0].read_text()), before)
                    self.assertTrue((backups[0].parent / "before-inspect.json").exists())
                    self.assertEqual(args[2:6], ["chat.example.com", "--image", self.image, "--auto-update=false"])
                    merged = dict(args[i + 1].split("=", 1) for i, value in enumerate(args) if value == "--env")
                    settings["env"] = merged
                    settings["autoUpdate"] = False
                    runtime.update(merged)
                    if after_change:
                        after_change(settings, runtime, container)
                    serialize()
                    return SimpleNamespace(returncode=update_code, stdout="PRIVATE-CHILD-OUTPUT", stderr="PRIVATE-CHILD-ERROR")
                else:
                    raise AssertionError("Unexpected command")
                return SimpleNamespace(returncode=0, stdout=output, stderr="")

            stdout, stderr = io.StringIO(), io.StringIO()
            with patch("sys.stdin", io.StringIO(json.dumps(payload))), patch("sys.stdout", stdout), patch("sys.stderr", stderr), \
                 patch("builtins.open", return_value=io.StringIO()), patch("fcntl.flock"), \
                 patch.object(once_configuration, "Path", return_value=temporary), \
                 patch("subprocess.run", side_effect=fake_run), \
                 patch("urllib.request.urlopen", return_value=nullcontext(SimpleNamespace(status=200 if healthy else 503))), \
                 patch("time.monotonic", side_effect=[0, 121]):
                code = 0
                try:
                    runpy.run_path(str(DIRECTORY / script), run_name="__main__")
                except SystemExit as error:
                    code = error.code
            files = {str(p.relative_to(temporary)): p.read_text() for p in temporary.glob("*/*")}
            for path in temporary.iterdir():
                self.assertEqual(path.stat().st_mode & 0o777, 0o700)
                for file in path.iterdir():
                    self.assertEqual(file.stat().st_mode & 0o777, 0o600)
        self.assertNotIn("PRIVATE-", stdout.getvalue() + stderr.getvalue())
        return SimpleNamespace(code=code, stdout=stdout.getvalue(), stderr=stderr.getvalue(), commands=commands,
                               settings=settings, runtime=runtime, files=files)

    def test_every_mode_has_explicit_values_and_unchanged_has_no_overrides(self):
        for mode, expected in {
            "unchanged": {}, "off": {"SPA_ENABLED": "0", "SPA_DEFAULT": "classic"},
            "opt-in": {"SPA_ENABLED": "1", "SPA_DEFAULT": "classic"},
            "default-next": {"SPA_ENABLED": "1", "SPA_DEFAULT": "next"},
        }.items():
            with self.subTest(mode=mode):
                payload = self.payload(SPA_MODE=mode)
                self.assertEqual(payload["environment"], expected)
                code, stdout, stderr = self.producer_run({**self.environ, "SPA_MODE": mode}, ["--validate"])
                self.assertEqual(code, 0, stderr)
                self.assertEqual(json.loads(stdout), {"spa_mode": mode, "environment": expected})

    def test_producer_emits_identity_and_plan_mode_and_refuses_bad_inputs(self):
        for dry_run, expected in [("true", "plan"), ("false", "apply")]:
            code, stdout, stderr = self.producer_run({**self.environ, "DRY_RUN": dry_run})
            self.assertEqual(code, 0, stderr)
            self.assertEqual(json.loads(stdout), dict(self.payload(), mode=expected))
        for change in [{"SPA_MODE": "PRIVATE-INVALID-MODE"}, {"SPA_MODE": "unchanged"},
                       {"GCP_APP_HOST": "invalid;host"}, {"DRY_RUN": "yes"}, {"EXPECTED_IMAGE": None}]:
            environ = {**self.environ, **change}
            environ = {k: v for k, v in environ.items() if v is not None}
            code, stdout, stderr = self.producer_run(environ)
            self.assertEqual(code, 1)
            self.assertEqual(stdout, "")
            self.assertNotIn("PRIVATE-", stderr)

    def test_apply_each_mode_preserves_the_complete_map_and_all_other_settings(self):
        for mode, expected in [("off", {"SPA_ENABLED": "0", "SPA_DEFAULT": "classic"}),
                               ("opt-in", {"SPA_ENABLED": "1", "SPA_DEFAULT": "classic"}),
                               ("default-next", {"SPA_ENABLED": "1", "SPA_DEFAULT": "next"})]:
            with self.subTest(mode=mode):
                result = self.host_run(self.payload(SPA_MODE=mode))
                self.assertEqual(result.code, 0, result.stderr)
                self.assertEqual(result.settings["env"], {**self.existing, **expected})
                self.assertEqual(result.runtime, {"GIT_REVISION": "b" * 40, "PATH": "/usr/local/bin", **self.existing, **expected})
                self.assertEqual(result.settings["secretKeyBase"], "PRIVATE-SIGNING-KEY")
                self.assertEqual(result.settings["backup"], {"path": "/backups", "schedule": "daily"})
                receipt = json.loads(result.stdout)
                self.assertEqual(receipt["configured_keys"], ["SPA_DEFAULT", "SPA_ENABLED"])
                for key in ["existing_environment_preserved", "existing_settings_preserved", "image_and_storage_preserved", "healthy"]:
                    self.assertIs(receipt[key], True)
                self.assertEqual(sum(c[:2] == ["once", "update"] for c in result.commands), 1)
                self.assertEqual(len(result.files), 4)

    def test_apply_can_add_both_switches_to_an_environment_without_them(self):
        del self.existing["SPA_ENABLED"]
        del self.existing["SPA_DEFAULT"]
        result = self.host_run(self.payload())
        self.assertEqual(result.code, 0, result.stderr)
        self.assertEqual(result.settings["env"], {**self.existing, "SPA_ENABLED": "1", "SPA_DEFAULT": "classic"})

    def test_only_the_two_spa_keys_and_supported_pairs_are_accepted_in_apply_and_plan(self):
        for mode in ["apply", "plan"]:
            for changes in [{"SPA_ENABLED": "1", "SPA_DEFAULT": "classic", "SECRET_KEY_BASE": "PRIVATE-REPLACEMENT"},
                            {"SPA_ENABLED": "1"}, {}, {"GOOGLE_CLIENT_SECRET": "PRIVATE-REPLACEMENT"},
                            {"SPA_ENABLED": "0", "SPA_DEFAULT": "next"}, {"SPA_ENABLED": True, "SPA_DEFAULT": "classic"},
                            {"SPA_ENABLED": "true", "SPA_DEFAULT": "classic"}]:
                with self.subTest(mode=mode, changes=list(changes)):
                    result = self.host_run(dict(self.payload(), mode=mode, environment=changes))
                    self.assertEqual(result.code, 1)
                    self.assertEqual(result.stdout, "")
                    self.assertFalse(any(c[:2] == ["once", "update"] for c in result.commands))
                    self.assertEqual(result.files, {})

    def test_stale_identity_unpinned_image_or_enabled_updates_refuse_before_backup_or_mutation(self):
        cases = [
            (dict(self.payload(), expected_revision="c" * 40), None),
            (dict(self.payload(), expected_image="wrong"), None),
            (dict(self.payload(), expected_image="mutable:tag"), lambda s, r, c: s.update(image="mutable:tag")),
            (self.payload(), lambda s, r, c: s.update(autoUpdate=True)),
            (dict(self.payload(), host="other.example.com"), None),
            (dict(self.payload(), mode="invalid"), None),
        ]
        for payload, change in cases:
            result = self.host_run(payload, before_change=change)
            self.assertEqual(result.code, 1)
            self.assertEqual(result.stdout, "")
            self.assertFalse(any(c[:2] == ["once", "update"] for c in result.commands))
            self.assertEqual(result.files, {})

    def test_verification_detects_every_preservation_failure_without_success_output(self):
        cases = [
            (lambda s, r, c: r.pop("SECRET_KEY_BASE"), "Runtime environment verification failed"),
            (lambda s, r, c: r.update(EXTRA_RUNTIME="unexpected"), "Runtime environment verification failed"),
            (lambda s, r, c: s["env"].pop("LIVEKIT_API_SECRET"), "Persistent environment verification failed"),
            (lambda s, r, c: s.update(autoUpdate=True), "Automatic image updates must remain disabled"),
            (lambda s, r, c: c.update(Image="changed"), "Image or storage changed unexpectedly"),
            (lambda s, r, c: c["Mounts"][0].update(Source="/other"), "Image or storage changed unexpectedly"),
            (lambda s, r, c: s.update(secretKeyBase="PRIVATE-REPLACEMENT"), "An existing ONCE setting changed unexpectedly"),
            (lambda s, r, c: s.pop("backup"), "An existing ONCE setting changed unexpectedly"),
            (lambda s, r, c: s.update(image="changed"), "An existing ONCE setting changed unexpectedly"),
        ]
        for change, expected in cases:
            with self.subTest(expected=expected):
                result = self.host_run(self.payload(), after_change=change)
                self.assertEqual(result.code, 1)
                self.assertEqual(result.stdout, "")
                self.assertIn(expected, result.stderr)
                self.assertEqual(len(result.files), 3)

    def test_update_failure_and_health_timeout_keep_protected_diagnostics_and_do_not_report_success(self):
        for options, message in [({"update_code": 1}, "ONCE configuration update failed"),
                                 ({"healthy": False}, "Application did not become healthy")]:
            result = self.host_run(self.payload(), **options)
            self.assertEqual(result.code, 1)
            self.assertEqual(result.stdout, "")
            self.assertIn(message, result.stderr)
            self.assertEqual(len(result.files), 3)
            self.assertTrue(any("PRIVATE-CHILD-OUTPUT" in value for value in result.files.values()))

    def test_dry_run_validates_and_prints_only_the_non_secret_values_without_changes(self):
        for mode in ["off", "opt-in", "default-next"]:
            result = self.host_run(self.payload(SPA_MODE=mode, DRY_RUN="true"))
            self.assertEqual(result.code, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout)["environment"], self.payload(SPA_MODE=mode)["environment"])
            self.assertEqual(result.settings["env"], self.existing)
            self.assertEqual(result.files, {})
            self.assertFalse(any(c[:2] == ["once", "update"] for c in result.commands))

    def test_google_plan_keeps_its_original_output_and_does_not_disclose_values(self):
        result = self.host_run({"mode": "plan", "host": "chat.example.com"}, script="configure-google.py")
        self.assertEqual(result.code, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), {
            "host": "chat.example.com", "image": self.image, "revision": "b" * 40,
            "configured_environment_keys": sorted(self.existing),
            "override_keys": sorted(["APP_URL", "GOOGLE_CLIENT_ID", "GOOGLE_CLIENT_SECRET", "GOOGLE_SIGN_IN_DOMAINS",
                                     "GOOGLE_PICKER_API_KEY", "GOOGLE_CLOUD_PROJECT_NUMBER"]),
            "will_preserve_existing_settings": True,
        })
        self.assertEqual(result.files, {})
        self.assertFalse(any(c[:2] == ["once", "update"] for c in result.commands))


if __name__ == "__main__":
    unittest.main()
