"""A cold generated host must supply the tracked outer test inputs."""
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from browser_host import prepare_source, build_host, include_inputs


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
                    b'''async fn ws8bm_browser_host_without_jobs() {}
                    include_str!("../../../../../../public/500.html");
                    include_str!("../../../../../reference-tools/messaging/older_provider_callbacks.rb");
                    include_str!("../../../../../reference-tools/users/new_harness.mjs");''',
                "rust/reference-tools/messaging/older_provider_callbacks.rb": b"callback",
                "rust/reference-tools/messaging/browser-drive-client.rs": b"external Drive client",
                "rust/reference-tools/users/new_harness.mjs": b"new included harness",
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
            tracked = "\0".join(inputs) + "\0"
            with patch("browser_host.subprocess.check_output", return_value=tracked):
                generated = prepare_source(root)
                self.assertEqual((generated.parent / "public/500.html").read_bytes(), inputs["public/500.html"])
                callback = generated / "reference-tools/messaging/older_provider_callbacks.rb"
                self.assertEqual(callback.read_bytes(), b"callback")
                self.assertEqual((generated / "reference-tools/users/new_harness.mjs").read_bytes(), b"new included harness")
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

    def test_reports_the_source_and_missing_tracked_include(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = Path("rust/crates/app/src/main.rs")
            contents = {source: b'include_bytes!("../../../../public/missing.html");'}
            for tracked in [set(), {Path("public/missing.html")}]:
                with self.assertRaisesRegex(RuntimeError, "main.rs: tracked compile-time include missing: public/missing.html"):
                    include_inputs(root, contents, tracked)

    def test_resolves_manifest_concat_raw_paths_and_ignores_rust_literals(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "rust/crates/app").mkdir(parents=True)
            (root / "rust/crates/app/Cargo.toml").write_text("")
            target = Path("rust/reference-tools/new.bin")
            (root / target).parent.mkdir(parents=True)
            (root / target).write_bytes(b"included")
            contents = {Path("rust/crates/app/src/main.rs"): b'''
                // include_str!("missing comment");
                /* nested /* include_bytes!("missing") */ comment */
                let generated = r#"include_bytes!(\"missing generated code\")"#;
                include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../reference-tools/new.bin",),);
                include_str!(r#"../../../reference-tools/new.bin"#);
            '''}
            self.assertEqual(include_inputs(root, contents, {target}), {target})

    def test_rejects_unknown_include_expressions_and_outside_paths(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = Path("rust/crates/app/src/main.rs")
            for content, message in [
                (b'include_str!(env!("NEW_INPUT"));', "unsupported compile-time include path"),
                (b'include_str!("../../../../../outside");', "compile-time include escapes the repository"),
            ]:
                with self.assertRaisesRegex(RuntimeError, message):
                    include_inputs(root, {source: content}, set())


if __name__ == "__main__":
    unittest.main()
