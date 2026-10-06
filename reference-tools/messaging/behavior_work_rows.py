"""Model/event assertions from pinned controller declarations :304-505."""
import json
import sqlite3
import sys


def work_counts(conn, agent_id):
    return {
        'work_events': conn.execute('SELECT COUNT(*) FROM work_thread_events').fetchone()[0],
        'agent_events': conn.execute("SELECT COUNT(*) FROM agent_events WHERE agent_id=? AND event_type='work_assigned'", (agent_id,)).fetchone()[0],
    }


def assert_work_rows(conn,seed,case,fixture):
    id=fixture['thread_id']
    row=conn.execute('SELECT work_status,work_owner_id FROM channel_threads WHERE id=?',(id,)).fetchone()
    events=lambda db:db.execute('SELECT actor_id,event_type,from_status,to_status,from_owner_id,to_owner_id FROM work_thread_events WHERE channel_thread_id=? ORDER BY id',(id,)).fetchall()
    before=events(seed);after=events(conn)
    # Rails :308,315,336,360,379 count all events, not just this thread.
    expected_delta=(2 if case.startswith(('converts a thread','work owner must','a manager can assign'))
                    else 1 if case.startswith(('assigned owner','only a thread manager')) else 0)
    global_delta=work_counts(conn,fixture['eligible_agent_id'])['work_events']-work_counts(seed,fixture['eligible_agent_id'])['work_events']
    assert global_delta==expected_delta,f'work-event-count: expected global delta {expected_delta}; got {global_delta}'
    if case.startswith('converts a thread'):
        assert row==('planned',712064548)
        assert after==[(773523953,'work_update',None,'planned',None,None),(773523953,'work_assignment','planned','planned',None,712064548)],after
    elif case.startswith('work owner must'):
        assert row==('planned',None);assert len(after)==len(before)+2
    elif case.startswith('assigned owner'):
        assert row==('in_progress',712064548);assert len(after)==len(before)+1;assert after[-1][0]==712064548
    elif case.startswith('only a thread manager'):
        assert row==(None,None);assert len(after)==len(before)+1
    elif case.startswith('a manager can assign'):
        assert row==('planned',fixture['eligible_id']);assert len(after)==len(before)+2
    else:
        assert row==seed.execute('SELECT work_status,work_owner_id FROM channel_threads WHERE id=?',(id,)).fetchone();assert after==before
    jobs=conn.execute("SELECT COUNT(*) FROM agent_events WHERE agent_id=? AND event_type='work_assigned'",(fixture['eligible_agent_id'],)).fetchone()[0]
    assert jobs==(1 if case.startswith('a manager can assign') else 0),f'agent-event-count: {jobs}'
    message=fixture['history_message_id']
    history=conn.execute("SELECT id FROM messages WHERE thread_id=? AND client_message_id='work-history'",(id,)).fetchone()
    assert history==(message,),f'work-history identity: expected message {message}; lookup returned {history}'
    assert conn.execute('SELECT thread_id,markdown_source FROM messages WHERE id=?',(message,)).fetchone()==(id,'Keep this history')


if __name__=='__main__':
    # The browser's controller checks take synchronous, read-only snapshots
    # around each real PATCH, matching assert_difference's request boundary.
    with sqlite3.connect(f'file:{sys.argv[1]}?mode=ro',uri=True) as conn:
        print(json.dumps(work_counts(conn,int(sys.argv[2]))))
