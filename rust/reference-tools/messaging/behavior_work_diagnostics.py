"""Read-only producer evidence; missing rows fail assertions, query errors are invalid."""
from contextlib import closing
import json
import sqlite3
from behavior_work_rows import assert_work_rows, work_counts


def work_readback(work, fixture, ports, metadata, case, env):
    for app, db in [('Rails', work / f'.instances/{ports[0]}/db/production.sqlite3'),
                    ('Rust', work / 'db/production.sqlite3')]:
        state={'app':app,'producer':env['WS8BM_WORK_PRODUCER']}
        try:
            with closing(sqlite3.connect(f'file:{db}?mode=ro',uri=True)) as candidate, closing(sqlite3.connect(
                    f"file:{fixture / 'db/production.sqlite3'}?mode=ro",uri=True)) as original:
                thread=metadata['thread_id'];message=metadata['history_message_id']
                state['thread']=candidate.execute('SELECT work_status,work_owner_id FROM channel_threads WHERE id=?',(thread,)).fetchone()
                state['events']=candidate.execute('SELECT actor_id,event_type,from_status,to_status,from_owner_id,to_owner_id FROM work_thread_events WHERE channel_thread_id=? ORDER BY id',(thread,)).fetchall()
                before=work_counts(original,metadata['eligible_agent_id']);after=work_counts(candidate,metadata['eligible_agent_id'])
                state['agent_events']=after['agent_events']
                history=candidate.execute('SELECT client_message_id FROM messages WHERE id=?',(message,)).fetchone()
                state['history_client_id']=history[0] if history is not None else None
                state['original_history_id']=message
                state['history_lookup']=candidate.execute("SELECT id,markdown_source FROM messages WHERE thread_id=? AND client_message_id='work-history'",(thread,)).fetchone()
                state['global_event_delta']=after['work_events']-before['work_events']
                state['target_event_delta']=len(state['events'])-original.execute('SELECT COUNT(*) FROM work_thread_events WHERE channel_thread_id=?',(thread,)).fetchone()[0]
                try:
                    assert_work_rows(candidate,original,case,metadata)
                    state['row_assertion']='PASS'
                except AssertionError as error:
                    state['row_assertion']='FAIL';state['row_error']=str(error)
        except Exception as error:
            # Operational/setup failures cannot earn rejection credit. Continue
            # with the peer; the driver's outer finally still owns teardown.
            state.update(row_assertion='INVALID',diagnostic_error=f'{type(error).__name__}: {error}')
        print('WS8bm real producer rows: '+json.dumps(state),flush=True)
