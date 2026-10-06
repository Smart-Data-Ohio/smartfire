"""Synthetic browser subprocess results through the driver's actual retry/count block."""
import ast
import contextlib
import io
import json
from pathlib import Path
from subprocess import CompletedProcess
from types import SimpleNamespace
import unittest

NAME = 'the picker remembers recent reactions'
ESCAPED = 'WS8bm escaped discrimination run: ' + json.dumps(dict(caseName=NAME, variant='default', reasons=['mutant escaped']))
INVALID = 'WS8bm invalid discrimination run: ' + json.dumps(dict(caseName=NAME, variant='default', reasons=['startup failed']))
REJECTED = f'WS8bm discrimination: message_toolbar: {NAME}: default: served mutant REJECTED (TimeoutError)'
FAILED = f'WS8bm browser flow FAILED: message_toolbar: {NAME}: synthetic assertion failure'


class DiscriminationRetryTest(unittest.TestCase):
    def run_attempts(self, attempts, *, negative=True, jobs=1):
        path = Path(__file__).with_name('behavior-check.py')
        tree = ast.parse(path.read_text())
        loop = next(node for node in ast.walk(tree) if isinstance(node, ast.For)
                    and ast.unparse(node.target) == '(batch, variant)')
        container = next(node for node in ast.walk(loop) if isinstance(node, ast.Try)
                         and any(isinstance(child, ast.Expr)
                                 and ast.unparse(child).startswith('print(result.stdout,')
                                 for child in node.body))
        start = next(node.lineno for node in container.body if isinstance(node, ast.Expr)
                     and ast.unparse(node).startswith('print(result.stdout,'))
        stop = next(node.lineno for node in container.body if isinstance(node, ast.If)
                    and node.lineno > start and ast.unparse(node.test) == 'args.mutant or args.mutant_set'
                    and 'passed += len(succeeded)' in ast.unparse(node))
        body = loop.body[:2] + ast.parse('result=next(results)').body
        body += [node for node in container.body if start <= node.lineno < stop]
        terminal = next(node for node in tree.body if isinstance(node, ast.If)
                        and ast.unparse(node.test) == 'failed_cases')
        code = ast.fix_missing_locations(ast.Module(body=[ast.For(
            target=loop.target, iter=loop.iter, body=body, orelse=[]), terminal], type_ignores=[]))
        state = dict(args=SimpleNamespace(negative=negative, keep_going=True, mutant=None, mutant_set=None),
                     jobs=[([NAME], 'default')] * jobs, retry_attempts={}, invalid_attempts=0, escaped_cases=set(),
                     env={}, drive_calls=None, file='message_toolbar', ROOT=path.parents[3], run_env={},
                     failed_cases=[], passed=0, passed_named=set(), command=[],
                     results=iter([CompletedProcess([], status, '\n'.join(lines)) for status, lines in attempts]),
                     subprocess=SimpleNamespace(check_output=lambda *args, **kwargs: json.dumps([['default']])), json=json)
        with contextlib.redirect_stdout(io.StringIO()):
            try:
                exec(compile(code, str(path), 'exec'), state)
            except SystemExit as error:
                state['driver_exit'] = error.code
            else:
                state['driver_exit'] = 0
        return state

    def test_an_escape_after_an_invalid_attempt_fails(self):
        state = self.run_attempts([(1, [INVALID]), (1, [ESCAPED])])
        self.assertEqual(sum(state['retry_attempts'].values()), 2)
        self.assertEqual(state['invalid_attempts'], 1)
        self.assertEqual(state['driver_exit'], 1)
        self.assertEqual(state['passed'], 0)
        self.assertEqual(state['passed_named'], set())
        self.assertEqual(state['failed_cases'], [f'message_toolbar: {NAME}'])

    def test_an_escape_is_never_retried_into_a_pass(self):
        state = self.run_attempts([(1, [ESCAPED]), (0, [REJECTED])])
        self.assertEqual(sum(state['retry_attempts'].values()), 1)
        self.assertEqual(state['driver_exit'], 1)
        self.assertEqual(state['failed_cases'], [f'message_toolbar: {NAME}'])

    def test_an_invalid_attempt_can_retry_without_false_failure(self):
        state = self.run_attempts([(1, [INVALID]), (0, [REJECTED])])
        self.assertEqual(sum(state['retry_attempts'].values()), 2)
        self.assertEqual(state['driver_exit'], 0)
        self.assertEqual(state['passed'], 1)
        self.assertEqual(state['failed_cases'], [])

    def test_positive_repetitions_keep_each_failed_attempt(self):
        state = self.run_attempts([(1, [FAILED]), (1, [FAILED])], negative=False, jobs=2)
        self.assertEqual(state['invalid_attempts'], 0)
        self.assertEqual(state['driver_exit'], 1)
        self.assertEqual(len(state['failed_cases']), 2)


if __name__ == '__main__':
    unittest.main()
