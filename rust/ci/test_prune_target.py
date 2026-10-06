import importlib.util
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location("prune_target", Path(__file__).with_name("prune-target.py"))
prune_target = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prune_target)

REGISTRY = {"serde", "libc", "regex_syntax"}
HASH = "0123456789abcdef"


class PruneTargetTest(unittest.TestCase):
    def tree(self, files):
        root = Path(tempfile.mkdtemp())
        for name in files:
            (root / name).parent.mkdir(parents=True, exist_ok=True)
            (root / name).write_text("")
        return root

    def remaining(self, root):
        return sorted(str(path.relative_to(root)) for path in root.rglob("*") if path.is_file())

    def test_stable_layout_keeps_registry_artifacts(self):
        root = self.tree([
            f"debug/deps/libserde-{HASH}.rlib", f"debug/deps/liblibc-{HASH}.rlib", f"debug/deps/libc-{HASH}.d",
            f"debug/.fingerprint/regex-syntax-{HASH}/lib-regex_syntax", f"debug/build/serde-{HASH}/out/x",
            f"debug/deps/campfire-{HASH}", f"debug/.fingerprint/campfire_db-{HASH}/lib",
            "debug/campfire", "debug/incremental/x", "ci-receipts/r.json",
        ])
        prune_target.prune(root / "debug", REGISTRY)
        self.assertEqual(self.remaining(root / "debug"), [
            f".fingerprint/regex-syntax-{HASH}/lib-regex_syntax", f"build/serde-{HASH}/out/x",
            f"deps/libc-{HASH}.d", f"deps/liblibc-{HASH}.rlib", f"deps/libserde-{HASH}.rlib",
        ])

    def test_nightly_layout_keeps_registry_package_directories(self):
        root = self.tree([
            f"debug/build/serde/{HASH}/out/libserde-{HASH}.rlib", f"debug/build/serde/{HASH}/fingerprint/lib-serde",
            f"debug/build/regex-syntax/{HASH}/out/x", f"debug/build/campfire/{HASH}/out/campfire",
            f"debug/build/campfire_db/{HASH}/fingerprint/lib", "debug/.cargo-lock",
        ])
        prune_target.prune(root / "debug", REGISTRY)
        self.assertEqual(self.remaining(root / "debug"), [
            f"build/regex-syntax/{HASH}/out/x", f"build/serde/{HASH}/fingerprint/lib-serde",
            f"build/serde/{HASH}/out/libserde-{HASH}.rlib",
        ])


if __name__ == "__main__":
    unittest.main()
