"""Muted delivery and real Calendar refresh with an injected server clock."""
import json,os,sqlite3
CASES=['muted-delivery','calendar-lifecycle']

def prepare(db,labels,root):
    pass

def environment_for_case(case,labels,root):
    return {'LEDGER_BROWSER_CLOCK':'1','GOOGLE_CLIENT_ID':'test-client-id','GOOGLE_CLIENT_SECRET':'test-client-secret'} if case=='calendar-lifecycle' else {}

def host_command(binary,env,root):
    env['WS11UI_LEDGER_HOST']='1'
    return [os.environ['WS11UI_LEDGER_TEST_HOST'],'controllers::ledger_browser_tests::lifecycle_host::original_ledger_injected_clock_host','--exact','--ignored','--nocapture']

def after_start_case(database,root,case):
    if case!='calendar-lifecycle':return
    # The original setup's encrypted GoogleAccount, as Rails created it.
    rows=json.loads((root/'test-support/ledger-lifecycle.json').read_text())['google_accounts']
    with sqlite3.connect(database) as db:
        db.execute('DELETE FROM google_accounts')
        for row in rows:
            db.execute('INSERT INTO google_accounts('+','.join(row)+') VALUES('+','.join('?' for _ in row)+')',list(row.values()))

def fixture(db,case,labels,root):
    pass

def check_state(db,requests,labels,root):
    for r in requests:
        if r['kind']=='calendar-enabled':
            assert db.execute('SELECT meeting_status_enabled FROM users WHERE id=?',(labels['users.david'],)).fetchone()==(1,)
        elif r['kind']=='muted':
            assert db.execute('SELECT involvement FROM memberships WHERE room_id=? AND user_id=?',(labels['rooms.designers'],labels['users.david'])).fetchone()==('muted',)
        else:raise AssertionError(r)
