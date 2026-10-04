"""A cold generated host must supply the tracked outer test inputs."""
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from browser_host import prepare_source, build_host


class HostSourceTests(unittest.TestCase):
    def test_generated_executable_cannot_replace_the_workspace_suite_binary(self):
        import json
        from types import SimpleNamespace
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            artifact = {'reason': 'compiler-artifact', 'target': {'name': 'campfire'},
                        'profile': {'test': True}, 'executable': str(root / 'target/ws8bm-browser-host/debug/deps/host')}
            result = SimpleNamespace(returncode=0, stdout=json.dumps(artifact))
            with patch('browser_host.prepare_source', return_value=root / 'generated'), \
                    patch('browser_host.subprocess.run', return_value=result) as run:
                self.assertEqual(build_host(root, {'CARGO_TARGET_DIR': str(root / 'target')}), artifact['executable'])
            self.assertEqual(run.call_args.kwargs['env']['CARGO_TARGET_DIR'], str(root / 'target/ws8bm-browser-host'))
            self.assertEqual(run.call_args.kwargs['env']['CAMPFIRE_REFERENCE'], str(root))

    def test_refreshes_outer_inputs_without_copying_targets_or_old_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            inputs = {
                "rust/Cargo.toml": b"workspace",
                "rust/parity/reference.sha": b"pinned",
                "rust/reference-tools/messaging/browser-attachment-jobs.rs": b"explicit attachment job adapter",
                "rust/crates/campfire/src/controllers/presenters/test_support.rs":
                    b"async fn ws8bm_browser_host_without_jobs() {}",
                "rust/reference-tools/messaging/older_provider_callbacks.rb": b"callback",
                "rust/reference-tools/messaging/browser-drive-client.rs": b"external Drive client",
                "public/500.html": b"original error page",
                "rust/reference-tools/views/agents_ui/extreme_cast_inputs.json.gz": b"extreme",
                "rust/reference-tools/views/agents_ui/normalized_cast_inputs.json.gz": b"normalized",
                "rust/reference-tools/views/agents_ui/render_replay_inputs.json.gz": b"replay",
                "rust/reference-tools/views/agents_ui/casting_followups_inputs.json": b"followups",
                "rust/target/debug/stale": b"not an input",
            }
            for relative, content in inputs.items():
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(content)
            tracked = "\n".join(inputs)
            with patch("browser_host.subprocess.check_output", return_value=tracked):
                generated = prepare_source(root)
                self.assertEqual((generated.parent / "public/500.html").read_bytes(), inputs["public/500.html"])
                callback = generated / "reference-tools/messaging/older_provider_callbacks.rb"
                self.assertEqual(callback.read_bytes(), b"callback")
                for name in ("extreme_cast_inputs.json.gz", "normalized_cast_inputs.json.gz", "render_replay_inputs.json.gz", "casting_followups_inputs.json"):
                    relative = "reference-tools/views/agents_ui/" + name
                    self.assertEqual((generated / relative).read_bytes(), inputs["rust/" + relative])
                self.assertEqual((generated / "parity/reference.sha").read_bytes(), b"pinned")
                self.assertFalse((generated / "target").exists())
                (generated / "stale.rs").write_bytes(b"old generated source")
                (root / "public/500.html").write_bytes(b"updated error page")
                (root / "rust/reference-tools/messaging/older_provider_callbacks.rb").write_bytes(b"updated callback")
                prepare_source(root)
                self.assertFalse((generated / "stale.rs").exists())
                self.assertEqual((generated.parent / "public/500.html").read_bytes(), b"updated error page")
                self.assertEqual(callback.read_bytes(), b"updated callback")

    def test_requires_a_tracked_error_page(self):
        with tempfile.TemporaryDirectory() as directory:
            with patch("browser_host.subprocess.check_output", return_value=""):
                with self.assertRaisesRegex(RuntimeError, "tracked public/500.html"):
                    prepare_source(Path(directory))


if __name__ == "__main__":
    unittest.main()
