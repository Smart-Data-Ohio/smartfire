"""Model/event assertions from pinned controller declarations :304-505."""
def assert_work_rows(conn,seed,case,fixture):
    id=fixture['thread_id']
    row=conn.execute('SELECT work_status,work_owner_id FROM channel_threads WHERE id=?',(id,)).fetchone()
    events=lambda db:db.execute('SELECT actor_id,event_type,from_status,to_status,from_owner_id,to_owner_id FROM work_thread_events WHERE channel_thread_id=? ORDER BY id',(id,)).fetchall()
    before=events(seed);after=events(conn)
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
    assert jobs==(1 if case.startswith('a manager can assign') else 0),jobs
    message=fixture['history_message_id']
    assert conn.execute('SELECT thread_id,markdown_source FROM messages WHERE id=?',(message,)).fetchone()==(id,'Keep this history')
