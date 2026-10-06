"""Generated diagnostic failures must still execute the real driver teardown."""
import ast
import contextlib
import importlib.util
import io
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile
import unittest

TOOLS=Path(__file__).resolve().parent
ROOT=TOOLS.parents[2]
spec=importlib.util.spec_from_file_location('work_producer_cleanup',TOOLS/'work-producer-discrimination.py')
producer=importlib.util.module_from_spec(spec);spec.loader.exec_module(producer)

class Process:
    def __init__(self,calls):self.calls=calls
    def terminate(self):self.calls.append('Rust terminate')
    def wait(self,**kwargs):self.calls.append('Rust wait')
    def kill(self):self.calls.append('Rust kill')

class DriverCleanupTest(unittest.TestCase):
    def setUp(self):
        scratch=ROOT/'.scratch';scratch.mkdir(exist_ok=True)
        self.temp=tempfile.TemporaryDirectory(prefix='ws8bm-cleanup-test-',dir=scratch)
        self.addCleanup(self.temp.cleanup);self.root=Path(self.temp.name)
        source=self.root/'rust/reference-tools/messaging/behavior-check.py'
        source.parent.mkdir(parents=True);shutil.copyfile(TOOLS/'behavior-check.py',source)
        out=self.root/'generated';out.mkdir();producer.driver_source(self.root,out)
        nodes=[node for node in ast.walk(ast.parse((out/'driver-body.py').read_text()))
               if isinstance(node,ast.Try) and node.finalbody and
               any(marker in ast.unparse(ast.Module(body=node.finalbody,type_ignores=[]))
                   for marker in ['process.terminate()', 'stop_behavior_server('])]
        final=min(nodes,key=lambda node:node.lineno).finalbody
        self.cleanup=compile(ast.fix_missing_locations(ast.Module(body=final,type_ignores=[])),'generated-finally','exec')
        self.work=self.root/'work';self.fixture=self.root/'fixture'
        self.metadata={'thread_id':1,'history_message_id':2,'eligible_id':9,'eligible_agent_id':8}
        for directory in [self.fixture/'db',self.work/'db']:
            directory.mkdir(parents=True)
            with contextlib.closing(sqlite3.connect(directory/'production.sqlite3')) as db, db:
                db.executescript('''CREATE TABLE channel_threads(id,work_status,work_owner_id);
                CREATE TABLE work_thread_events(id,channel_thread_id,actor_id,event_type,from_status,to_status,from_owner_id,to_owner_id);
                CREATE TABLE agent_events(agent_id,event_type);
                CREATE TABLE messages(id,thread_id,markdown_source,client_message_id);
                INSERT INTO channel_threads VALUES(1,'planned',712064548);
                INSERT INTO messages VALUES(2,1,'Keep this history','work-history');''')
                if directory!=self.fixture/'db':
                    db.executescript('''INSERT INTO work_thread_events VALUES(1,1,773523953,'work_update',NULL,'planned',NULL,NULL);
                    INSERT INTO work_thread_events VALUES(2,1,773523953,'work_assignment','planned','planned',NULL,712064548);''')
        self.calls=[];self.process=Process(self.calls)

    def execute(self):
        from unittest.mock import patch
        namespace=dict(file='channel_threads_controller',metadata=self.metadata,
            work=self.work,fixture=self.fixture,
            case='converts a thread to work, assigns an eligible owner, and keeps an audit trail',
            env={'WS8BM_WORK_PRODUCER':'deleted-history'},process=self.process,
            ROOT=self.root,run_env={},log=io.StringIO(),
            sqlite3=sqlite3,subprocess=subprocess,json=__import__('json'))
        from behavior_server_cleanup import stop_behavior_server
        namespace['stop_behavior_server']=stop_behavior_server
        output=io.StringIO()
        with contextlib.redirect_stdout(output):
            try:exec(self.cleanup,namespace)
            except Exception as error:self.error=error
        return output.getvalue()

    def test_deleted_history_is_failure_evidence_and_cleans_up(self):
        with contextlib.closing(sqlite3.connect(self.work/'db/production.sqlite3')) as conn, conn:conn.execute('DELETE FROM messages')
        output=self.execute()
        self.assertEqual(self.calls,['Rust terminate','Rust wait'])
        self.assertFalse(hasattr(self,'error'),getattr(self,'error',None))
        records=[__import__('json').loads(line.split(': ',1)[1]) for line in output.splitlines() if line.startswith('WS8bm real producer rows:')]
        self.assertEqual(len(records),1)
        for row in records:
            self.assertIsNone(row['history_client_id'])
            self.assertEqual(row['row_assertion'],'FAIL')
            self.assertIn('work-history identity:',row['row_error'])

    def test_broken_readback_is_invalid_and_still_cleans_up(self):
        with contextlib.closing(sqlite3.connect(self.work/'db/production.sqlite3')) as conn, conn:conn.execute('DROP TABLE messages')
        output=self.execute()
        self.assertEqual(self.calls,['Rust terminate','Rust wait'])
        self.assertIn('"row_assertion": "INVALID"',output)
        self.assertIn('OperationalError',output)

    def test_uncaught_diagnostic_error_cannot_skip_shutdown(self):
        from unittest.mock import patch
        with patch('behavior_work_diagnostics.work_readback',side_effect=RuntimeError('diagnostic failed')):
            self.execute()
        self.assertEqual(self.calls,['Rust terminate','Rust wait'])
        self.assertIsInstance(self.error,RuntimeError)

    def test_terminate_error_still_kills_the_app(self):
        def terminate():self.calls.append('Rust terminate');raise OSError('shutdown failure')
        self.process.terminate=terminate
        self.execute()
        self.assertEqual(self.calls,['Rust terminate','Rust kill','Rust wait'])

if __name__=='__main__':unittest.main()
