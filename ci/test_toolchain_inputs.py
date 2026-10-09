import importlib.util
from pathlib import Path
import re
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("toolchain_inputs", Path(__file__).with_name("toolchain-inputs.py"))
inputs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(inputs)


class ToolchainInputsTest(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        for name in ["Dockerfile", "rust-toolchain.toml"]:
            (self.root / name).write_bytes((ROOT / name).read_bytes())
        self.original = inputs.toolchain_inputs(self.root)[0]

    def change(self, old, new):
        path = self.root / "Dockerfile"
        text = path.read_text()
        self.assertIn(old, text)
        path.write_text(text.replace(old, new))

    def test_registry_default_and_app_stages_do_not_change_the_hash(self):
        self.change("ARG BASE_REGISTRY=mirror.gcr.io", "ARG BASE_REGISTRY=us-central1-docker.pkg.dev/smart-data-campfire/docker-hub")
        self.change("ARG CARGO_BUILD_JOBS=4", "ARG CARGO_BUILD_JOBS=3")
        self.change("ARG APP_VERSION", "ARG APP_VERSION=another-release")
        self.assertEqual(inputs.toolchain_inputs(self.root)[0], self.original)

    def test_media_recipe_and_global_defaults_change_the_hash(self):
        self.change("-Djpeg=enabled", "-Djpeg=disabled")
        self.assertNotEqual(inputs.toolchain_inputs(self.root)[0], self.original)
        changed = inputs.toolchain_inputs(self.root)[0]
        argument = re.search(r"^ARG RUST_VERSION=.*$", (self.root / "Dockerfile").read_text(), re.M)[0]
        self.change(argument, argument + "-changed")
        self.assertNotEqual(inputs.toolchain_inputs(self.root)[0], changed)

    def test_nightly_contents_change_the_hash(self):
        with (self.root / "rust-toolchain.toml").open("a") as file:
            file.write("\n# changed nightly input\n")
        self.assertNotEqual(inputs.toolchain_inputs(self.root)[0], self.original)

    def test_transitive_copied_patches_change_the_hash(self):
        patch = self.root / "patches" / "vips.patch"
        patch.parent.mkdir()
        patch.write_text("original patch")
        self.change("FROM media-base AS vips", "FROM media-base AS vips\nCOPY patches /patches")
        digest, files = inputs.toolchain_inputs(self.root)
        self.assertEqual(files, ["patches/vips.patch", "rust-toolchain.toml"])
        self.assertNotEqual(digest, self.original)
        patch.write_text("changed patch")
        self.assertNotEqual(inputs.toolchain_inputs(self.root)[0], digest)
        digest = inputs.toolchain_inputs(self.root)[0]
        patch.chmod(0o755)
        self.assertNotEqual(inputs.toolchain_inputs(self.root)[0], digest)

    def test_new_unrelated_stages_do_not_change_the_hash(self):
        with (self.root / "Dockerfile").open("a") as file:
            file.write("\nFROM scratch AS unrelated\nCOPY absent /unused\n")
        self.assertEqual(inputs.toolchain_inputs(self.root)[0], self.original)

    def test_unhandled_context_dependencies_fail_instead_of_reusing_a_tag(self):
        self.change("FROM media-base AS toolchain", "FROM media-base AS toolchain\nRUN --mount=type=bind,source=patches,target=/patches true")
        with self.assertRaisesRegex(ValueError, "RUN mounts"):
            inputs.toolchain_inputs(self.root)

    def test_producer_push_paths_include_every_current_copy_input(self):
        workflow = (ROOT / ".github/workflows/toolchain-image.yml").read_text()
        paths = re.findall(r"^      - (.+)$", workflow, re.M)
        _, files = inputs.toolchain_inputs(ROOT)
        self.assertTrue(set(["Dockerfile", *files]).issubset(paths), "Add new toolchain COPY inputs to the producer's push paths")


if __name__ == "__main__":
    unittest.main()
