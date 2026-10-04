"""Readback dispatch must survive adding/reordering browser declarations."""
import ast
from pathlib import Path
import unittest

SOURCE = Path(__file__).with_name('behavior-check.py')


class NamedReadbacks(unittest.TestCase):
    def test_native_case_gets_database_metadata_inside_a_shared_batch(self):
        tree = ast.parse(SOURCE.read_text())
        branch = next(node for node in ast.walk(tree) if isinstance(node, ast.If)
                      and any(isinstance(child, ast.Constant)
                              and child.value == 'WS8BM_WORK_DATABASES'
                              for statement in node.body for child in ast.walk(statement)))
        condition = compile(ast.Expression(branch.test), str(SOURCE), 'eval')
        native = 'a release click landing on the just-opened menu does not activate it'
        other = 'opens message actions from context menu and keyboard, and cancels a moving long press'
        for batch in ([other, native], [native, other], [native]):
            with self.subTest(batch=batch):
                self.assertTrue(eval(condition, {'file': 'message_interactions',
                                                 'case': batch[0], 'batch': batch}))
        self.assertFalse(eval(condition, {'file': 'message_interactions',
                                          'case': other, 'batch': [other]}))

    def test_original_named_cases_keep_their_persisted_row_checks(self):
        tree = ast.parse(SOURCE.read_text())
        cases = next(ast.literal_eval(node.value) for node in tree.body
                     if isinstance(node, ast.Assign)
                     and any(isinstance(target, ast.Name) and target.id == 'CASES'
                             for target in node.targets))
        branch = next(node for node in ast.walk(tree) if isinstance(node, ast.If)
                      and ast.unparse(node.test) == "case == 'workspace follows the system theme and mobile navigation remains reachable'")
        originals = [
            ('workspace_markdown', 'Markdown messages reach other users and editing preserves the original source', '## Review complete%'),
            ('workspace_markdown', 'desktop keyboard composition keeps line breaks and sends once after composition ends', 'First line\nSecond line'),
            ('workspace_markdown', 'untrusted markup stays inert in the delivered message', "payload = <<~'MARKDOWN'\n"),
            ('threads', 'creates a thread from a channel message and keeps the channel draft separate', "name='Design review thread'"),
            ('threads', 'the thread root counts its replies live and hides the count when none remain', "name='Indicator thread'"),
            ('threads', 'a stray create re-entry does not wipe the half-filled thread name', "name='Survives a stray reset'"),
        ]
        for file, case, witness in originals:
            with self.subTest(case=case):
                selected = branch
                while not eval(compile(ast.Expression(selected.test), str(SOURCE), 'eval'),
                               {'CASES': cases, 'file': file, 'case': case}):
                    self.assertTrue(selected.orelse, f'No readback for {case}')
                    selected = selected.orelse[0]
                    self.assertIsInstance(selected, ast.If, f'No named readback for {case}')
                strings = [node.value for statement in selected.body
                           for node in ast.walk(statement)
                           if isinstance(node, ast.Constant) and isinstance(node.value, str)]
                self.assertTrue(any(witness in value for value in strings),
                                f'{case} selected another declaration\'s readback')


if __name__ == '__main__':
    unittest.main()
