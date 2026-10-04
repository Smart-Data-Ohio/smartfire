"""Native producer provenance follows the built app, not a moving source checkout."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

REFERENCE = Path(__file__).resolve().parent / 'bin/reference'
PIN = (REFERENCE.parent.parent / 'reference.sha').read_text().strip()


class NativeReferenceTests(unittest.TestCase):
    def test_build_records_actual_source_revision(self):
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            source, native = root / 'source', root / 'native'
            source.mkdir()
            (source / '.ruby-version').write_text('3.4.10\n')
            (source / 'bin').mkdir()
            rails = source / 'bin/rails'
            rails.write_text('#!/bin/sh\nexit 0\n')
            rails.chmod(0o755)
            subprocess.run(['git', 'init', '-q', str(source)], check=True)
            subprocess.run(['git', '-C', str(source), 'add', '.'], check=True)
            subprocess.run(['git', '-C', str(source), '-c', 'user.name=Parity test',
                            '-c', 'user.email=parity@example.test', '-c', 'core.hooksPath=/dev/null',
                            '-c', 'commit.gpgsign=false', 'commit', '-qm', 'Reference fixture'], check=True)
            revision = subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip()
            (native / 'bin').mkdir(parents=True)
            redis = native / 'bin/redis-server'
            redis.touch()
            redis.chmod(0o755)
            (native / 'lib').mkdir()
            (native / 'lib/libfaketime.so.1').touch()
            env = dict(os.environ, PARITY_RUNTIME='native', REFERENCE_SCRIPT=str(REFERENCE),
                       NATIVE_TEST=str(native), SOURCE_TEST=str(source))
            run = subprocess.run(['bash', '-ec', '''
                source "$REFERENCE_SCRIPT" runtime >/dev/null
                NATIVE=$NATIVE_TEST
                REFERENCE_ROOT=$SOURCE_TEST
                ruby() { return 0; }
                bundle() { return 0; }
                native_bundle_bin() { echo /usr/bin; }
                native_build
            '''], env=env, capture_output=True, text=True)
            self.assertEqual(run.returncode, 0, run.stderr)
            self.assertEqual((native / 'app/.parity-reference.sha').read_text().strip(), revision)

    def probe(self, revision):
        with tempfile.TemporaryDirectory() as scratch:
            app = Path(scratch) / 'app'
            app.mkdir()
            if revision is not None:
                (app / '.parity-reference.sha').write_text(revision + '\n')
            env = dict(os.environ, PARITY_RUNTIME='native', REFERENCE_SCRIPT=str(REFERENCE),
                       NATIVE_TEST=scratch, PARITY_REFERENCE_SHA='stale-caller-value')
            return subprocess.run(['bash', '-ec', '''
                source "$REFERENCE_SCRIPT" runtime >/dev/null
                NATIVE=$NATIVE_TEST
                native_bundle_bin() { echo /usr/bin; }
                cd "$NATIVE/app"
                native_env exec
                ruby -e 'puts ENV.fetch("PARITY_REFERENCE_SHA"); puts ENV.fetch("GIT_REVISION")'
            '''], env=env, capture_output=True, text=True)

    def test_exports_built_revision_and_stable_http_revision(self):
        run = self.probe(PIN)
        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertEqual(run.stdout.splitlines(), [PIN, 'parity'])

    def test_keeps_older_built_revision_for_controls(self):
        revision = 'a' * 40
        run = self.probe(revision)
        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertEqual(run.stdout.splitlines(), [revision, 'parity'])

    def test_unrecorded_native_app_requires_rebuild(self):
        run = self.probe(None)
        self.assertNotEqual(run.returncode, 0)
        self.assertIn('native app has no revision: rebuild', run.stderr)


if __name__ == '__main__':
    unittest.main()
