"""Synthetic browser subprocess results through the driver's actual retry/count block."""
import ast
import contextlib
import io
import json
from pathlib import Path
from subprocess import CompletedProcess
from types import SimpleNamespace
import unittest


class DiscriminationRetryTest(unittest.TestCase):
    def run_attempts(self, first_app, other_app, *, escape=True):
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
        code = ast.fix_missing_locations(ast.Module(body=[ast.For(
            target=loop.target, iter=loop.iter, body=body, orelse=[])], type_ignores=[]))
        name = 'the picker remembers recent reactions'
        first = []
        if escape:
            first.append('WS8bm escaped discrimination run: ' + json.dumps(
                dict(caseName=name, variant='default', app=first_app, reasons=['mutant escaped'])))
        else:
            first.append(f'WS8bm discrimination: message_toolbar: {name}: default: {first_app} served mutant REJECTED (TimeoutError)')
        first.append('WS8bm invalid discrimination run: ' + json.dumps(
            dict(caseName=name, variant='default', app=other_app, reasons=['startup failed'])))
        second = '\n'.join(f'WS8bm discrimination: message_toolbar: {name}: default: {app} served mutant REJECTED (TimeoutError)'
                           for app in ['Rails', 'Rust'])
        state = dict(args=SimpleNamespace(negative=True, keep_going=True, mutant=None, mutant_set=None),
                     jobs=[([name], 'default')], retry_attempts={}, invalid_attempts=0, escaped_cases=set(),
                     env={}, file='message_toolbar', ROOT=path.parents[3], run_env={},
                     failed_cases=[], passed=0, passed_named=set(), command=[],
                     results=iter([CompletedProcess([], 1, '\n'.join(first)), CompletedProcess([], 0, second)]),
                     subprocess=SimpleNamespace(check_output=lambda *args, **kwargs: json.dumps([['default']])), json=json)
        with contextlib.redirect_stdout(io.StringIO()):
            exec(compile(code, str(path), 'exec'), state)
        self.assertEqual(sum(state['retry_attempts'].values()), 2)
        self.assertEqual(state['invalid_attempts'], 1)
        return state

    def test_rails_escape_survives_rust_invalid_then_both_reject(self):
        state = self.run_attempts('Rails', 'Rust')
        self.assertEqual(state['passed'], 0)
        self.assertEqual(state['passed_named'], set())
        self.assertEqual(state['failed_cases'], ['message_toolbar: the picker remembers recent reactions'])

    def test_rust_escape_survives_rails_invalid_then_both_reject(self):
        state = self.run_attempts('Rust', 'Rails')
        self.assertEqual(state['passed'], 0)
        self.assertEqual(state['passed_named'], set())
        self.assertEqual(state['failed_cases'], ['message_toolbar: the picker remembers recent reactions'])

    def test_rails_rejection_and_rust_invalid_can_retry_without_false_failure(self):
        state = self.run_attempts('Rails', 'Rust', escape=False)
        self.assertEqual(state['passed'], 1)
        self.assertEqual(state['failed_cases'], [])

    def test_rust_rejection_and_rails_invalid_can_retry_without_false_failure(self):
        state = self.run_attempts('Rust', 'Rails', escape=False)
        self.assertEqual(state['passed'], 1)
        self.assertEqual(state['failed_cases'], [])


if __name__ == '__main__':
    unittest.main()
