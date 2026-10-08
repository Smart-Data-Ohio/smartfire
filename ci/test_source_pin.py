"""Execute the workflow's pin step across a moving branch and full reruns."""
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import unittest

from check_gate_needs import jobs

WORKFLOW = Path(__file__).resolve().parents[1] / '.github/workflows/rust.yml'


class SourcePinTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workflow = WORKFLOW.read_text()
        cls.source = cls.workflow.split('  source:\n', 1)[1].split('\n  changes:\n', 1)[0]
        step = cls.source.split('      - name: Pin the tested commit\n', 1)[1]
        script = step.split("          python3 - <<'PY'\n", 1)[1].split('\n          PY', 1)[0]
        cls.script = '\n'.join(line[10:] for line in script.splitlines())

    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory()
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)
        self.repo = self.root / 'repo'
        self.repo.mkdir()
        self.git('init', '-q')
        self.commit('first')
        self.first = self.git('rev-parse', 'HEAD').strip()
        self.runner = self.root / 'runner'
        self.output = self.root / 'output'
        self.pin = self.runner / 'source/source.json'

    def git(self, *args):
        return subprocess.check_output(['git', '-c', 'core.hooksPath=/dev/null',
            '-c', 'commit.gpgsign=false', '-c', 'user.name=CI test',
            '-c', 'user.email=ci@example.invalid', *args], cwd=self.repo, text=True)

    def commit(self, content):
        (self.repo / 'file.txt').write_text(content)
        self.git('add', 'file.txt')
        self.git('commit', '-qm', content)

    def resolve(self, attempt=1, **changed):
        self.output.unlink(missing_ok=True)
        env = dict(os.environ, RUNNER_TEMP=str(self.runner), GITHUB_OUTPUT=str(self.output),
                   GITHUB_RUN_ID='12345', GITHUB_RUN_ATTEMPT=str(attempt), GITHUB_SHA='a' * 40,
                   GITHUB_WORKFLOW_SHA='b' * 40, REQUESTED_REF='moving-branch')
        env.update(changed)
        return subprocess.run([sys.executable, '-c', self.script], env=env, cwd=self.repo,
                              text=True, capture_output=True)

    def test_reruns_keep_the_first_sha_after_the_requested_branch_moves(self):
        first = self.resolve()
        self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
        saved = self.pin.read_bytes()
        self.assertEqual(json.loads(saved)['head'], self.first)
        self.commit('branch advances')
        self.assertNotEqual(self.git('rev-parse', 'HEAD').strip(), self.first)
        for attempt in (2, 3):
            result = self.resolve(attempt)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(self.output.read_text(), f'sha={self.first}\n')
            self.assertEqual(self.pin.read_bytes(), saved)

    def test_missing_pin_on_rerun_fails_without_resolving_the_branch(self):
        result = self.resolve(2)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('source pin is missing', result.stderr)
        self.assertFalse(self.output.exists())
        self.assertFalse(self.pin.exists())

    def test_pin_from_another_run_or_context_fails(self):
        self.assertEqual(self.resolve().returncode, 0)
        for key, value in (('GITHUB_RUN_ID', '54321'), ('GITHUB_SHA', 'c' * 40),
                           ('GITHUB_WORKFLOW_SHA', 'd' * 40), ('REQUESTED_REF', 'other-branch')):
            with self.subTest(key=key):
                result = self.resolve(2, **{key: value})
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('does not belong', result.stderr)
                self.assertFalse(self.output.exists())

    def test_invalid_or_missing_commit_is_rejected(self):
        self.assertEqual(self.resolve().returncode, 0)
        saved = json.loads(self.pin.read_text())
        for head in ('main', 'a' * 39, 'a' * 40 + '\nextra=value', None):
            with self.subTest(head=head):
                self.pin.write_text(json.dumps(dict(saved, head=head)))
                result = self.resolve(2)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(self.output.exists())

    def test_only_initial_resolution_can_checkout_a_mutable_ref(self):
        refs = re.findall(r'^          ref: (.+)$', self.workflow, re.M)
        self.assertEqual(refs.count('${{ inputs.ref || github.sha }}'), 1)
        self.assertTrue(all(ref == '${{ needs.source.outputs.sha }}' for ref in refs
                            if ref != '${{ inputs.ref || github.sha }}'))
        self.assertIn('if: github.run_attempt == 1\n        uses: actions/checkout@', self.source)
        self.assertIn('if: github.run_attempt > 1\n        uses: actions/download-artifact@', self.source)
        restore = self.source.split('      - name: Restore the first attempt', 1)[1].split(
            '      - name: Resolve the requested ref', 1)[0]
        self.assertIn('github-token: ${{ github.token }}', restore)
        self.assertIn('repository: ${{ github.repository }}', restore)
        self.assertIn('run-id: ${{ github.run_id }}', restore)
        for job, needs in jobs(self.workflow).items():
            if job != 'source':
                self.assertIn('source', needs, job)


if __name__ == '__main__':
    unittest.main()
