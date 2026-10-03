"""Pinned threads_test.rb persisted model assertions after real browser writes."""

def assert_thread_rows(conn, seed, case, metadata):
    if case.startswith(('shows work-thread guidance', 'keeps the new-thread guidance', 'opens a shared')):
        for table, columns in [('messages', 'id,markdown_source'), ('channel_threads','id,work_status,work_owner_id')]:
            query=f'SELECT {columns} FROM {table} ORDER BY id'
            assert conn.execute(query).fetchall()==seed.execute(query).fetchall()
        return
    name = {'tracks work':'Work handoff thread','shows work assignment':'Cross-feature work handoff',
            'marks a joined':'Unread thread coverage','keeps an anchored':'Anchored unread race'}.get(next((prefix for prefix in ['tracks work','shows work assignment','marks a joined','keeps an anchored'] if case.startswith(prefix)),''))
    if case.startswith('keeps the thread drawer'):
        for title,body in [('Mobile thread','The mobile thread starter.'),('Mobile second thread','A second mobile thread.')]:
            thread=conn.execute('SELECT id FROM channel_threads WHERE name=?',(title,)).fetchall()
            assert len(thread)==1
            assert conn.execute('SELECT creator_id,thread_id FROM messages WHERE markdown_source=?',(body,)).fetchall()==[(773523953,thread[0][0])]
        return
    thread=conn.execute('SELECT id,work_status,work_owner_id FROM channel_threads WHERE name=?',(name,)).fetchall()
    assert len(thread)==1
    id,status,owner=thread[0]
    if case.startswith(('tracks work','shows work assignment')):
        assert (status,owner)==('planned' if case.startswith('tracks work') else 'in_progress',712064548)
        events=conn.execute('SELECT actor_id,event_type,from_status,to_status,from_owner_id,to_owner_id FROM work_thread_events WHERE channel_thread_id=? ORDER BY id',(id,)).fetchall()
        assert len(events)==(5 if case.startswith('tracks work') else 3), events
        assert events[1]==(773523953,'work_assignment','planned','planned',None,712064548),events
        body='The work conversation must survive completion.' if case.startswith('tracks work') else 'The assigned work message remains available.'
        assert conn.execute('SELECT creator_id,thread_id FROM messages WHERE markdown_source=?',(body,)).fetchall()==[(773523953,id)]
        if case.startswith('shows work assignment'):
            assert conn.execute("SELECT COUNT(*) FROM activity_items JOIN work_thread_events ON work_thread_events.id=activity_items.source_id WHERE activity_items.user_id=712064548 AND source_type='WorkThreadEvent' AND channel_thread_id=?",(id,)).fetchone()[0]>=1
    else:
        unread=conn.execute('SELECT unread_at IS NOT NULL FROM thread_memberships WHERE thread_id=? AND user_id=773523953',(id,)).fetchone()
        assert unread==(0 if case.startswith('keeps an anchored') else 1,)
        body='A reply during the anchored page.' if case.startswith('keeps an anchored') else 'A hidden second-user reply.'
        assert conn.execute('SELECT creator_id,thread_id FROM messages WHERE markdown_source=?',(body,)).fetchall()==[(712064548,id)]
