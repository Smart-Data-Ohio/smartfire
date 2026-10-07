"""A cold generated host must supply the tracked outer test inputs."""
from pathlib import Path
import re
import tempfile
import unittest
from unittest.mock import patch

from browser_host import prepare_source, build_host, include_inputs, audit_build


class HostSourceTests(unittest.TestCase):
    def test_follows_tracked_literal_include_inputs_outside_the_copy_prefixes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            inputs = {
                "public/500.html": b"original error page",
                "crates/campfire/src/receipt.rs":
                    b'include_str!(\n "../../../plans/receipt.json"\n);\n'
                    b'include_bytes!("../../../reference-tools/original.rb");',
                "plans/receipt.json": b'{"original_assertions":3}',
                "reference-tools/original.rb": b"assert rendered_body",
            }
            for relative, content in inputs.items():
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(content)
            with patch("browser_host.subprocess.check_output", return_value="\0".join(inputs)):
                generated = prepare_source(root)
                self.assertEqual((generated / "plans/receipt.json").read_bytes(), inputs["plans/receipt.json"])
                self.assertEqual((generated / "reference-tools/original.rb").read_bytes(), inputs["reference-tools/original.rb"])
            with patch("browser_host.subprocess.check_output", return_value="\0".join(k for k in inputs if k != "plans/receipt.json")):
                with self.assertRaisesRegex(RuntimeError, "receipt.rs: tracked compile-time include missing: plans/receipt.json"):
                    prepare_source(root)

    def test_generated_pwa_module_keeps_its_real_relative_compile_inputs(self):
        project = Path(__file__).resolve().parents[2]
        module = Path("crates/campfire/src/controllers/pwa/tests.rs")
        content = (project / module).read_bytes()
        includes = re.findall(r'\binclude_(?:str|bytes)!\s*\(\s*"([^"]+)"', content.decode())
        self.assertTrue(includes, "the real PWA module must exercise compile-time inputs")
        inputs = {module: content, Path("web/public/500.html"): (project / "web/public/500.html").read_bytes()}
        for include in includes:
            path = (project / module.parent / include).resolve().relative_to(project)
            inputs[path] = (project / path).read_bytes()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative, original in inputs.items():
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(original)
            with patch("browser_host.subprocess.check_output", return_value="\0".join(map(str, inputs))):
                generated = prepare_source(root)
                copied_module = generated / module
                self.assertEqual(copied_module.read_bytes(), content)
                for include in includes:
                    with self.subTest(include=include):
                        original = (root / module.parent / include).resolve()
                        copied = (copied_module.parent / include).resolve()
                        self.assertEqual(copied.read_bytes(), original.read_bytes())
                        updated = original.read_bytes() + b"\n// refreshed compile-time input\n"
                        original.write_bytes(updated)
                        copied.write_bytes(b"stale generated input")
                        prepare_source(root)
                        self.assertEqual(copied.read_bytes(), updated)

    def test_generated_executable_cannot_replace_the_workspace_suite_binary(self):
        import json
        from types import SimpleNamespace
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            artifact = {'reason': 'compiler-artifact', 'target': {'name': 'campfire'},
                        'profile': {'test': True}, 'executable': str(root / 'target/ws8bm-browser-host/debug/deps/host')}
            result = SimpleNamespace(returncode=0, stdout=json.dumps(artifact))
            with patch('browser_host.prepare_source', return_value=root / 'generated'), \
                    patch('browser_host.audit_build') as audit, \
                    patch('browser_host.subprocess.run', return_value=result) as run:
                self.assertEqual(build_host(root, {'CARGO_TARGET_DIR': str(root / 'target')}), artifact['executable'])
            audit.assert_called_once()
            self.assertEqual(run.call_args.kwargs['env']['CARGO_TARGET_DIR'], str(root / 'target/ws8bm-browser-host'))

    def test_refreshes_outer_inputs_without_copying_targets_or_old_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            inputs = {
                "Cargo.toml": b"workspace",
                "parity/reference.sha": b"pinned",
                "reference-tools/messaging/browser-attachment-jobs.rs": b"explicit attachment job adapter",
                "crates/campfire/src/controllers/presenters/test_support.rs":
                    b'''async fn ws8bm_browser_host_without_jobs() {}
                    include_str!("../../../../../public/500.html");
                    include_str!("../../../../../reference-tools/messaging/older_provider_callbacks.rb");
                    include_str!("../../../../../reference-tools/users/new_harness.mjs");''',
                "reference-tools/messaging/older_provider_callbacks.rb": b"callback",
                "reference-tools/messaging/browser-drive-client.rs": b"external Drive client",
                "reference-tools/users/new_harness.mjs": b"new included harness",
                "public/500.html": b"original error page",
                "test-support/agents_ui/extreme_cast_inputs.json.gz": b"extreme",
                "test-support/agents_ui/normalized_cast_inputs.json.gz": b"normalized",
                "test-support/agents_ui/render_replay_inputs.json.gz": b"replay",
                "test-support/agents_ui/casting_followups_inputs.json": b"followups",
                "frontend/src/auth/auth.css": b"auth styles",
                "frontend/src/styles/index.css": b"shared styles",
                "frontend/src/styles/fonts/example.woff2": b"font",
                "frontend/src/motion/index.css": b"motion styles",
                "frontend/src/ui/button.css": b"button styles",
                "frontend/src/ui/text-field.css": b"field styles",
                "frontend/src/ui/checkbox.css": b"checkbox styles",
                "frontend/src/rooms/index.tsx": b"not an input",
                "frontend/dist/index.html": b"not an input",
                "target/debug/stale": b"not an input",
            }
            for relative, content in inputs.items():
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(content)
            tracked = "\0".join(inputs) + "\0"
            with patch("browser_host.subprocess.check_output", return_value=tracked):
                generated = prepare_source(root)
                self.assertEqual((generated / "public/500.html").read_bytes(), inputs["public/500.html"])
                callback = generated / "reference-tools/messaging/older_provider_callbacks.rb"
                self.assertEqual(callback.read_bytes(), b"callback")
                self.assertEqual((generated / "reference-tools/users/new_harness.mjs").read_bytes(), b"new included harness")
                for name in ("extreme_cast_inputs.json.gz", "normalized_cast_inputs.json.gz", "render_replay_inputs.json.gz", "casting_followups_inputs.json"):
                    relative = "test-support/agents_ui/" + name
                    self.assertEqual((generated / relative).read_bytes(), inputs[relative])
                self.assertEqual((generated / "parity/reference.sha").read_bytes(), b"pinned")
                self.assertFalse((generated / "target").exists())
                for relative in inputs:
                    if relative.startswith("frontend/src/") and "/rooms/" not in relative:
                        self.assertEqual((generated / relative).read_bytes(), inputs[relative])
                self.assertFalse((generated / "frontend/src/rooms").exists())
                self.assertFalse((generated / "frontend/dist").exists())
                (generated / "stale.rs").write_bytes(b"old generated source")
                (root / "public/500.html").write_bytes(b"updated error page")
                (root / "reference-tools/messaging/older_provider_callbacks.rb").write_bytes(b"updated callback")
                prepare_source(root)
                self.assertFalse((generated / "stale.rs").exists())
                self.assertEqual((generated / "public/500.html").read_bytes(), b"updated error page")
                self.assertEqual(callback.read_bytes(), b"updated callback")

    def test_reports_the_source_and_missing_tracked_include(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = Path("crates/app/src/main.rs")
            contents = {source: b'include_bytes!("../../../public/missing.html");'}
            for tracked in [set(), {Path("public/missing.html")}]:
                with self.assertRaisesRegex(RuntimeError, "main.rs: tracked compile-time include missing: public/missing.html"):
                    include_inputs(root, contents, tracked)

    def test_resolves_manifest_concat_raw_paths_and_ignores_rust_literals(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "crates/app").mkdir(parents=True)
            (root / "crates/app/Cargo.toml").write_text("")
            target = Path("reference-tools/new.bin")
            (root / target).parent.mkdir(parents=True)
            (root / target).write_bytes(b"included")
            contents = {Path("crates/app/src/main.rs"): b'''
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
            source = Path("crates/app/src/main.rs")
            for content, message in [
                (b'include_str!(env!("NEW_INPUT"));', "unsupported compile-time include path"),
                (b'include_str!("../../../../outside");', "compile-time include escapes the repository"),
            ]:
                with self.assertRaisesRegex(RuntimeError, message):
                    include_inputs(root, {source: content}, set())

    def test_all_include_macros_accept_all_delimiters(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = Path('input.rs')
            (root / target).write_text('const INCLUDED: u8 = 1;')
            for macro in ('include', 'include_str', 'include_bytes'):
                for opening, closing in (('(', ')'), ('[', ']'), ('{', '}')):
                    with self.subTest(macro=macro, delimiter=opening):
                        source = f'{macro}!{opening}"input.rs",{closing};'.encode()
                        self.assertEqual(include_inputs(root, {Path('main.rs'): source}, {target}), {target})

    def test_all_include_forms_reject_unresolved_and_missing_paths(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for macro in ('include', 'include_str', 'include_bytes'):
                for opening, closing in (('(', ')'), ('[', ']'), ('{', '}')):
                    for argument, error in [('concat!(env!("NEW_INPUT"), "/missing")', 'unsupported'),
                                            ('$path', 'unsupported'), ('"missing"', 'missing')]:
                        with self.subTest(macro=macro, delimiter=opening, argument=argument):
                            content = f'{macro}!{opening}{argument}{closing};'.encode()
                            with self.assertRaisesRegex(RuntimeError, f'main.rs: .*{error}'):
                                include_inputs(root, {Path('main.rs'): content}, set())

    def test_raw_include_identifiers_accept_all_delimiters(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = Path('input.rs')
            (root / target).write_text('const INCLUDED: u8 = 1;')
            for macro in ('include', 'include_str', 'include_bytes'):
                for opening, closing in (('(', ')'), ('[', ']'), ('{', '}')):
                    with self.subTest(macro=macro, delimiter=opening):
                        source = f'r#{macro}!{opening}"input.rs"{closing};'.encode()
                        self.assertEqual(include_inputs(root, {Path('build.rs'): source}, {target}), {target})

    def test_raw_include_identifiers_reject_unknown_and_missing_paths(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for macro in ('include', 'include_str', 'include_bytes'):
                for opening, closing in (('(', ')'), ('[', ']'), ('{', '}')):
                    for argument, error in [('env!("UNKNOWN")', 'unsupported'), ('$path', 'unsupported'), ('"missing"', 'missing')]:
                        with self.subTest(macro=macro, delimiter=opening, argument=argument):
                            source = f'r#{macro}!{opening}{argument}{closing};'.encode()
                            with self.assertRaisesRegex(RuntimeError, f'build.rs: .*{error}'):
                                include_inputs(root, {Path('build.rs'): source}, set())

    def test_qualified_include_identifiers_and_comments_accept_all_delimiters(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = Path('input.rs')
            (root / target).write_text('const INCLUDED: u8 = 1;')
            for prefix in ('::core::', 'std::', '$crate::', '::r#core::r#', 'r#std::r#'):
                for macro in ('include', 'include_str', 'include_bytes'):
                    for opening, closing in (('(', ')'), ('[', ']'), ('{', '}')):
                        with self.subTest(prefix=prefix, macro=macro, delimiter=opening):
                            source = f'{prefix}{macro} /* before bang */ ! // before delimiter\n{opening}"input.rs"{closing};'.encode()
                            self.assertEqual(include_inputs(root, {Path('main.rs'): source}, {target}), {target})

    def test_qualified_include_identifiers_reject_unknown_and_missing_paths(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for prefix in ('::core::', 'std::', '$crate::', '::r#core::r#', 'r#std::r#'):
                for macro in ('include', 'include_str', 'include_bytes'):
                    for opening, closing in (('(', ')'), ('[', ']'), ('{', '}')):
                        for argument, error in [('env!("UNKNOWN")', 'unsupported'), ('$path', 'unsupported'), ('"missing"', 'missing')]:
                            with self.subTest(prefix=prefix, macro=macro, delimiter=opening, argument=argument):
                                source = f'{prefix}{macro}!{opening}{argument}{closing};'.encode()
                                with self.assertRaisesRegex(RuntimeError, f'main.rs: .*{error}'):
                                    include_inputs(root, {Path('main.rs'): source}, set())

    def test_raw_and_qualified_include_syntax_errors_fail_loudly(self):
        for name in ('r#include', '::core::include_str', 'std::r#include_bytes'):
            for body, error in [('"input"', 'invalid .* delimiter'), ('["input"}', 'unbalanced'), ('("input"', 'unterminated')]:
                with self.subTest(name=name, body=body), tempfile.TemporaryDirectory() as directory:
                    with self.assertRaisesRegex(RuntimeError, f'main.rs: .*{error}'):
                        include_inputs(Path(directory), {Path('main.rs'): f'{name}!{body}'.encode()}, set())

    def test_raw_include_recursively_copies_non_rs_sources_and_rejects_unknown_paths(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            inputs = {
                'Cargo.toml': '',
                'crates/app/build.rs': 'r#include!["../../reference-tools/source.inc"];',
                'reference-tools/source.inc': '::core::r#include_str!{"leaf.txt"};',
                'reference-tools/leaf.txt': 'included',
            }
            for relative, content in inputs.items():
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content)
            with patch('browser_host.subprocess.check_output', return_value='\0'.join(inputs)):
                generated = prepare_source(root)
                for relative in ('reference-tools/source.inc', 'reference-tools/leaf.txt'):
                    self.assertEqual((generated / relative).read_text(), inputs[relative])
                (root / 'reference-tools/source.inc').write_text('r#include_bytes![env!("UNKNOWN")];')
                with self.assertRaisesRegex(RuntimeError, 'source.inc: unsupported'):
                    prepare_source(root)

    def test_concat_and_manifest_env_accept_all_delimiters_and_rust_escapes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'Cargo.toml').write_text('')
            target = Path('a snow ☃.txt')
            (root / target).write_bytes(b'input')
            for opening, closing in (('(', ')'), ('[', ']'), ('{', '}')):
                with self.subTest(delimiter=opening):
                    content = f'include_str!{{concat!{opening}env!{opening}"CARGO_MANIFEST_DIR"{closing}, r"/a", "\\x20snow \\u{{2603}}.txt",{closing}}};'.encode()
                    self.assertEqual(include_inputs(root, {Path('build.rs'): content}, {target}), {target})

    def test_include_follows_non_rs_sources_with_the_owning_manifest_and_cycles(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'Cargo.toml').write_text('')
            (root / 'other').mkdir()
            (root / 'other/Cargo.toml').write_text('')
            fragment = Path('other/source.inc')
            leaf = Path('leaf.txt')
            (root / leaf).write_bytes(b'included')
            (root / fragment).write_text('include_str![concat!(env!("CARGO_MANIFEST_DIR"), "/leaf.txt")]; include!{"source.inc"};')
            self.assertEqual(include_inputs(root, {Path('build.rs'): b'include!("other/source.inc");'}, {fragment, leaf}), {fragment, leaf})

    def test_build_script_includes_are_copied_and_unresolved_paths_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            paths = ['Cargo.toml', 'crates/app/Cargo.toml', 'crates/app/build.rs', 'reference-tools/input.txt']
            for relative in paths:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('')
            build = root / paths[2]
            build.write_text('include_str!{"../../reference-tools/input.txt"};')
            with patch('browser_host.subprocess.check_output', return_value='\0'.join(paths)):
                generated = prepare_source(root)
                self.assertTrue((generated / 'reference-tools/input.txt').is_file())
                build.write_text('include![env!("UNKNOWN")];')
                with self.assertRaisesRegex(RuntimeError, 'build.rs: unsupported'):
                    prepare_source(root)

    def test_out_dir_can_only_be_deferred_explicitly_and_unknown_parts_still_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'Cargo.toml').write_text('')
            (root / 'build.rs').write_text('')
            content = b'include!(concat!(env!("OUT_DIR"), "/generated.rs"));'
            with self.assertRaisesRegex(RuntimeError, 'main.rs: unsupported'):
                include_inputs(root, {Path('main.rs'): content}, set())
            pending = []
            self.assertEqual(include_inputs(root, {Path('main.rs'): content}, set(), pending=pending), set())
            self.assertEqual(len(pending), 1)
            with self.assertRaisesRegex(RuntimeError, 'main.rs: unsupported'):
                include_inputs(root, {Path('main.rs'): b'include!(concat!(env!("OUT_DIR"), env!("UNKNOWN")));'}, set(), pending=[])

    def generated_fixture(self, root):
        generated = root / '.scratch/host/workspace'
        crate = generated / 'crates/app'
        crate.mkdir(parents=True)
        tracked = [Path('crates/app') / name for name in ('Cargo.toml', 'build.rs', 'lib.rs', 'static.txt')]
        for path in tracked:
            (root / path).parent.mkdir(parents=True, exist_ok=True)
            (root / path).write_text('')
            (generated / path).write_text('')
        (crate / 'lib.rs').write_text('include!{concat![env!("OUT_DIR"), "/generated.rs"]};')
        target = root / 'target'
        output = target / 'debug/build/app/out'
        output.mkdir(parents=True)
        (output / 'generated.rs').write_text('include_bytes!["payload.bin"]; include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/static.txt"));')
        (output / 'payload.bin').write_bytes(b'generated payload')
        depfile = target / 'debug/deps/app-hash.d'
        depfile.parent.mkdir(parents=True)
        dependencies = [crate / 'lib.rs', output / 'generated.rs', output / 'payload.bin', crate / 'static.txt']
        depfile.write_text('artifact: ' + ' '.join(str(path).replace(' ', '\\ ') for path in dependencies) + '\n')
        entries = [{'reason': 'compiler-artifact', 'manifest_path': str(crate / 'Cargo.toml'), 'package_id': 'app',
                    'filenames': [str(depfile.with_name('libapp-hash.rlib'))], 'target': {'src_path': str(crate / 'lib.rs'), 'kind': ['lib']}},
                   {'reason': 'build-script-executed', 'package_id': 'app', 'out_dir': str(output)}]
        return generated, target, entries, '\0'.join(map(str, tracked)), depfile, output

    def test_cargo_output_resolves_and_audits_generated_nested_includes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            generated, target, entries, tracked, _, _ = self.generated_fixture(root)
            with patch('browser_host.subprocess.check_output', return_value=tracked):
                report = audit_build(root, generated, entries, target)
            self.assertEqual(len(report['generated_include_edges']), 3)
            self.assertEqual(len(report['compiler_dependencies']), 4)
            self.assertEqual(report['pending'], [])

    def test_raw_generated_non_rs_include_cannot_bypass_the_final_audit(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            generated, target, entries, tracked, depfile, output = self.generated_fixture(root)
            (generated / 'crates/app/lib.rs').write_text('r#include!{concat![env!{"OUT_DIR"}, "/generated.inc"]};')
            (output / 'generated.rs').rename(output / 'generated.inc')
            depfile.write_text(depfile.read_text().replace('generated.rs', 'generated.inc'))
            with patch('browser_host.subprocess.check_output', return_value=tracked):
                report = audit_build(root, generated, entries, target)
                self.assertEqual(len(report['generated_include_edges']), 3)
                (output / 'generated.inc').write_text('::core::r#include_str!(env!("REVIEW_UNKNOWN"));')
                with self.assertRaisesRegex(RuntimeError, 'generated.inc: unsupported'):
                    audit_build(root, generated, entries, target)

    def test_compiler_dependencies_support_escaped_spaces(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'space root'
            root.mkdir()
            generated, target, entries, tracked, _, _ = self.generated_fixture(root)
            with patch('browser_host.subprocess.check_output', return_value=tracked):
                self.assertEqual(len(audit_build(root, generated, entries, target)['compiler_dependencies']), 4)

    def test_generated_rust_modules_in_compiler_dependencies_are_also_scanned(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            generated, target, entries, tracked, depfile, output = self.generated_fixture(root)
            helper = output / 'helper.rs'
            helper.write_text('include_str!{env!("UNKNOWN")};')
            depfile.write_text(depfile.read_text().rstrip() + ' ' + str(helper) + '\n')
            with patch('browser_host.subprocess.check_output', return_value=tracked):
                with self.assertRaisesRegex(RuntimeError, 'helper.rs: unsupported'):
                    audit_build(root, generated, entries, target)

    def test_host_executable_is_not_returned_when_the_compiler_audit_fails(self):
        import json
        from types import SimpleNamespace
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            artifact = {'reason': 'compiler-artifact', 'target': {'name': 'campfire'},
                        'profile': {'test': True}, 'executable': str(root / 'host')}
            with patch('browser_host.prepare_source', return_value=root / 'generated'), \
                    patch('browser_host.subprocess.run', return_value=SimpleNamespace(returncode=0, stdout=json.dumps(artifact))), \
                    patch('browser_host.audit_build', side_effect=RuntimeError('include audit failed')):
                with self.assertRaisesRegex(RuntimeError, 'include audit failed'):
                    build_host(root, {})

    def test_generated_audit_rejects_missing_receipts_dependencies_and_unknown_code(self):
        for defect in ('receipt', 'depfile', 'edge', 'untracked', 'generated_code'):
            with self.subTest(defect=defect), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                generated, target, entries, tracked, depfile, output = self.generated_fixture(root)
                if defect == 'receipt':
                    entries.pop()
                elif defect == 'depfile':
                    depfile.unlink()
                elif defect == 'edge':
                    depfile.write_text('artifact: ' + str(output / 'generated.rs') + '\n')
                elif defect == 'untracked':
                    (root / 'untracked.txt').write_text('')
                    depfile.write_text(depfile.read_text().rstrip() + ' ' + str(root / 'untracked.txt') + '\n')
                else:
                    (output / 'generated.rs').write_text('include_bytes!{env!("UNKNOWN")};')
                with patch('browser_host.subprocess.check_output', return_value=tracked):
                    with self.assertRaises(RuntimeError):
                        audit_build(root, generated, entries, target)


if __name__ == "__main__":
    unittest.main()
