"""Exercise the workflow's actual required-check script for full and scoped runs."""
import json
import os
from pathlib import Path
import subprocess
import sys
import unittest

from check_gate_needs import jobs

WORKFLOW = Path(__file__).resolve().parents[1] / '.github/workflows/rust.yml'


class CorrectnessJobGateTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        workflow = WORKFLOW.read_text()
        cls.required = jobs(workflow)['correctness-gate']
        step = workflow.split('      - name: Gate on every correctness job\n', 1)[1]
        cls.script = step.split("          python3 - <<'PY'\n", 1)[1].split('\n          PY', 1)[0]
        cls.script = '\n'.join(line[10:] for line in cls.script.splitlines())

    def gate(self, *, rust=True, pages=True, spa=False, partial=False, changed=None):
        statuses = dict.fromkeys(self.required, 'success' if pages else 'skipped')
        statuses['source'] = 'success'
        statuses['changes'] = 'success'
        statuses['seeds'] = 'success' if rust else 'skipped'
        if spa:
            statuses['correctness'] = 'success'
            statuses['correctness-image'] = 'success'
        if partial:
            statuses['correctness'] = 'skipped'
            statuses['correctness-livekit'] = 'skipped'
        statuses.update(changed or {})
        env = dict(os.environ, RUST=str(rust).lower(), PAGES=str(pages).lower(),
                   SPA=str(spa).lower(),
                   PARTIAL=str(partial).lower(),
                   NEEDS=json.dumps({job: {'result': status} for job, status in statuses.items()}))
        return subprocess.run([sys.executable, '-c', self.script], env=env, capture_output=True, text=True)

    def test_full_and_partial_runs_pass_with_every_selected_job_successful(self):
        for partial in (False, True):
            result = self.gate(partial=partial)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_every_selected_failure_cancellation_or_skip_fails_the_gate(self):
        for partial in (False, True):
            for job in self.required:
                if partial and job in ('correctness', 'correctness-livekit'):
                    continue
                for status in ('failure', 'cancelled', 'skipped'):
                    with self.subTest(partial=partial, job=job, status=status):
                        result = self.gate(partial=partial, changed={job: status})
                        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)

    def test_no_page_inputs_requires_every_page_job_to_be_skipped(self):
        for rust in (False, True):
            result = self.gate(rust=rust, pages=False)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            for job in set(self.required) - {'source', 'changes', 'seeds'}:
                for status in ('success', 'failure', 'cancelled'):
                    with self.subTest(rust=rust, job=job, status=status):
                        result = self.gate(rust=rust, pages=False, changed={job: status})
                        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)

    def test_partial_scope_requires_full_only_jobs_to_be_skipped(self):
        for job in ('correctness', 'correctness-livekit'):
            for status in ('success', 'failure', 'cancelled'):
                with self.subTest(job=job, status=status):
                    result = self.gate(partial=True, changed={job: status})
                    self.assertEqual(result.returncode, 1, result.stdout + result.stderr)

    def test_spa_only_runs_require_pwa_and_its_image_and_skip_other_page_jobs(self):
        for rust in (False, True):
            result = self.gate(rust=rust, pages=False, spa=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            for job in set(self.required) - {'source', 'changes', 'seeds'}:
                expected = 'success' if job in ('correctness', 'correctness-image') else 'skipped'
                for status in {'success', 'failure', 'cancelled', 'skipped'} - {expected}:
                    with self.subTest(rust=rust, job=job, status=status):
                        result = self.gate(rust=rust, pages=False, spa=True, changed={job: status})
                        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)

    def test_invalid_spa_output_fails_the_gate(self):
        for spa in (None, '', 'invalid'):
            with self.subTest(spa=spa):
                result = self.gate(spa=spa)
                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)


if __name__ == '__main__':
    unittest.main()
