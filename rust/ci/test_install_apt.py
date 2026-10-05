import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from install_apt import install, validate_dependency_graph, validate_lock, verify_deb


LOCK = json.loads((Path(__file__).parent / "apt-amd64.lock.json").read_text())


class AptPrerequisitePins(unittest.TestCase):
    def test_runtime_dependencies_require_locked_providers(self):
        packages = [{"package": "root"}, {"package": "provider"}]
        metadata = {"root": ("virtual-library:any (>= 1) | fallback", "provider", ""),
                    "provider": ("", "", "virtual-library (= 1)")}
        validate_dependency_graph(packages, metadata)
        metadata["root"] = ("unlocked-library (>= 1)", "", "")
        with self.assertRaisesRegex(ValueError, "unlocked runtime dependency"):
            validate_dependency_graph(packages, metadata)

    def test_lock_covers_prerequisites_and_rejects_missing_pins(self):
        validate_lock(LOCK)
        for mutation in ("checksum", "root", "duplicate"):
            lock = copy.deepcopy(LOCK)
            if mutation == "checksum":
                lock["packages"][0]["sha256"] = ""
            elif mutation == "root":
                lock["requested"].append("unlocked-dependency")
            else:
                lock["packages"].append(lock["packages"][0])
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                validate_lock(lock)

    def test_corrupted_archive_cannot_execute_package_tools(self):
        with tempfile.TemporaryDirectory() as scratch:
            archive = Path(scratch) / "test.deb"
            archive.write_bytes(b"corrupt")
            with patch("install_apt.subprocess.check_output") as command:
                with self.assertRaisesRegex(ValueError, "checksum mismatch"):
                    verify_deb(LOCK["packages"][0], archive)
                command.assert_not_called()

    def test_all_archives_are_verified_before_installation(self):
        lock = copy.deepcopy(LOCK)
        lock["packages"] = lock["packages"][:2]
        lock["requested"] = [package["package"] for package in lock["packages"]]
        lock["packages"][0]["sha256"] = hashlib.sha256(b"archive").hexdigest()

        def download(command, **kwargs):
            self.assertEqual(command[0], "curl", "package installation began before all checksums passed")
            Path(command[-1]).write_bytes(b"archive")

        with patch("install_apt.subprocess.run", side_effect=download), \
             patch("install_apt.subprocess.check_output", side_effect=["amd64\n", "\t".join(lock["packages"][0][key] for key in ("package", "version", "architecture"))]):
            with self.assertRaisesRegex(ValueError, "checksum mismatch"):
                install(lock)
