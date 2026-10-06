"""Original surface browser fixtures; no production responses are synthesized."""
CASES=['audit-filter','audit-phone','icon-colon','icon-room','icon-search','icon-lobehub','mobile-outside','mobile-destinations','group-ringing','worker-cache','worker-retry','timezone-once','unread-few','unread-many','unread-pill','unread-offpage','unread-menu','workspace-icons','csp-login','csp-main','csp-turbo','csp-inline','csp-surfaces']
CSP_ENV={'GOOGLE_CLIENT_ID':'test-client-id','GOOGLE_CLIENT_SECRET':'test-client-secret','LIVEKIT_URL':'ws://127.0.0.1:9','LIVEKIT_INTERNAL_URL':'http://127.0.0.1:10','LIVEKIT_API_KEY':'csp-test-key','LIVEKIT_API_SECRET':'csp-test-secret-csp-test-secret-0000','LIVEKIT_GATEWAY_SECRET':'csp-test-gateway-secret','GOOGLE_PICKER_API_KEY':'test-picker-key','GOOGLE_CLOUD_PROJECT_NUMBER':'123456789012'}

def prepare(db,labels,root):
    assert {r[0] for r in db.execute('SELECT id FROM messages WHERE room_id=?',(labels['rooms.designers'],))}=={labels['messages.'+name] for name in ['first','second','third']}, 'original Designers fixture must contain only first/second/third'
    labels['surface.stage']=9100900101;labels['surface.board']=9100900102
    labels['surface.group_huddle_environment']=dict(GROUP_HUDDLE_ENV)

def check_state(db,requests,labels,root):
    for r in requests:
        file,line=r['file'],r['line']
        if r['kind']=='message':
            row=db.execute('SELECT id FROM messages WHERE markdown_source=? ORDER BY id DESC LIMIT 1',(r['body'],)).fetchone()
            assert row and row[0]==r['id'],(file,line,'persisted original markdown',row,r)
        elif r['kind']=='message-absent':
            assert not db.execute('SELECT 1 FROM messages WHERE markdown_source=?',(r['body'],)).fetchone(),(file,line,r)
        elif r['kind']=='timezone':
            assert db.execute('SELECT time_zone FROM users WHERE id=?',(labels['users.jz'],)).fetchone()==(r['zone'],),(file,line,r)
        elif r['kind']=='huddle-activity':
            assert db.execute('SELECT 1 FROM activity_items WHERE user_id=? AND event_type="huddle_started"',(labels['users.'+r['user']],)).fetchone(),(file,line,r)
        else:raise AssertionError(r)
        print(f'ORIGINAL_ASSERTION {file}:{line}',flush=True)

GROUP_HUDDLE_ENV={'LIVEKIT_URL':'wss://huddle.example.test','LIVEKIT_INTERNAL_URL':'ws://livekit.example.test:7880','LIVEKIT_API_KEY':'test-api-key','LIVEKIT_API_SECRET':'test-api-secret','LIVEKIT_GATEWAY_SECRET':'test-gateway-secret'}
CASE_ENV={case:(dict(CSP_ENV) if case.startswith('csp-') else {}) for case in CASES}

def environment(labels,root):
    return {}

def case_environment(case,labels,root):
    return CASE_ENV.get(case,{})

from ledger_browser_lifecycle import host_command

CASES.insert(CASES.index('timezone-once'),'timezone-no-csrf')

CONTROL_CASES=['audit-filter','icon-colon','icon-room','icon-search','icon-lobehub','workspace-icons','unread-pill','worker-retry','group-ringing','timezone-no-csrf']
