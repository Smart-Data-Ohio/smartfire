#!/usr/bin/env python3
"""Validate the original named assertions' positive fixtures and wire/state premises."""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]

def check():
    rows=json.loads((ROOT/'rust/vectors/agent_next6_named.json').read_text())['cases']
    cases={r['key']:r for r in rows}
    assert len(cases)==len(rows)==31
    def wire(key,index=0):
        o=cases[key]['observations'][index]
        return json.loads(o['response_body'])
    def titles(key,index=0):return [r['title'] for r in wire(key,index)]
    def events(key,index=0):return wire(key,index)['events']
    assert titles('posts_order')==['Blocked','Newer','Older']
    assert titles('posts_reply',0)==['Newer','Older'] and titles('posts_reply',1)==['Older','Newer']
    assert [titles('posts_status',i) for i in range(4)]==[['Planned'],['Finished'],['Finished','Progressing','Planned'],['Progressing','Planned']]
    assert [titles('posts_owner',i) for i in range(4)]==[['Mine'],['Theirs','Mine'],['Humans'],[]]
    assert len(wire('posts_cap'))==100 and titles('posts_cap')[0]=='Capped 100' and titles('posts_cap')[-1]=='Capped 1'
    assert titles('posts_legacy')==['Legacy visible'] and cases['posts_legacy']['observations'][1]['status']==403
    assert len(wire('posts_left'))==1 and cases['posts_left']['observations'][1]['status']==404
    created=wire('shared_payload');listed=wire('shared_payload',1);assigned=events('shared_payload',2)[0]['work']
    assert listed==[created] and assigned['assigned_by']=='Bender Bot'
    assert {k:v for k,v in assigned.items() if k not in ('thread_id','status','assigned_by')}==created
    assert titles('work_order')==['Newer work','Older work']
    assert wire('work_order')[0]['work_status']=='planned' and wire('work_order')[0]['updated_at']
    assert wire('work_empty')==[] and cases['work_empty']['observations'][1]['status']==403
    links=wire('work_links_show')['links'];assert [l['kind'] for l in links]==['pull_request','event','drive_file']
    assert links[0]['pull_request']['owner']=='rails' and links[0]['pull_request']['repo']=='rails' and links[0]['pull_request']['number']==7
    assert links[0]['title']==links[0]['pull_request']['title']=='Fix login' and links[0]['pull_request']['state']=='open'
    assert links[1]['title']=='Watercooler sync' and links[1]['event']['starts_at'] and links[1]['event']['cancelled'] is False
    assert links[2]['title']=='Q3 Planning' and links[2]['event'] is None and links[2]['pull_request'] is None
    listed={r['title']:r for r in wire('work_links_list')};assert [r['kind'] for r in listed['Listed links']['links']]==['event'] and listed['Unlinked work']['links']==[]
    assert wire('work_note_history')['work_status']=='in_progress'
    assert wire('work_note_history',1)[0]['event_type']=='work_update' and wire('work_note_history',1)[0]['actor']['id']==394959859 and wire('work_note_history',1)[0]['note']=='Digging into the bug'
    assert wire('work_note_history',2)=={'note':True}
    assert titles('work_orphaned')==['Kept work']
    for key in ('result_permissions','result_precedence'):
        assert [r['status'] for r in cases[key]['observations']]==[404,403]
        assert all(t['result'] is None for t in cases[key]['extra_state']['threads'])
    assigned=events('assignment_poll')[0];assert assigned['event_type']=='work_assigned' and assigned['outcome']=='delivered' and assigned['actor']['name']=='David'
    assert assigned['work']['title']=='Polled work' and assigned['work']['status']=='planned' and assigned['work']['assigned_by']=='David' and 'message' not in assigned
    links=events('assignment_links')[0]['work']['links'];assert [l['kind'] for l in links]==['event','drive_file'] and links[1]['title'] is None
    assert [e['event_type'] for e in events('unassignment_poll')]==['work_assigned','work_unassigned']
    assert wire('assignment_ack')['outcome']=='acknowledged' and cases['assignment_ack']['state']['ledger'][0]['outcome']=='acknowledged'
    hook=cases['assignment_webhook']['observations'][0]['output'];assert hook['queued']==1 and len(hook['requests'])==1
    body=json.loads(hook['requests'][0]['body']);assert body['agent']['name']=='Bender Bot' and body['event_type']=='work_assigned' and body['work']['title']=='Hooked work' and body['work']['assigned_by']=='David'
    assert cases['assignment_no_read']['observations'][0]['output']=={'queued':0,'requests':[]}
    assert len(cases['assignment_no_read']['state']['ledger'])==1 and cases['assignment_no_read']['state']['ledger'][0]['webhook_status']=='none'
    for key in ('assignment_revoked','assignment_left'):assert len(events(key,0))==1 and events(key,1)==[]
    rate=cases['work_message_rate']['state']['ledger']
    for agent in (773018776,1901100002):
        owned=[r for r in rate if r['agent']==agent]
        assert sum(r['kind']=='mention' for r in owned)==20
        assert sum(r['kind'] in ('work_assigned','work_unassigned') for r in owned)==2
        assert sum(r['kind']=='delivery_suppressed_rate_limit' for r in owned)==1
    handoff=events('handoff_poll',1)[0];assert handoff['event_type']=='work_handed_off' and handoff['handoff']['summary']=='Halfway' and handoff['handoff']['links']==['https://example.com/a'] and handoff['handoff']['open_questions']==['Why?'] and handoff['handoff']['sender_name']=='Bender Bot'
    assert wire('handoff_ack',1)['outcome']=='acknowledged' and cases['handoff_ack']['state']['ledger'][-1]['outcome']=='acknowledged'
    hook=cases['handoff_webhook']['observations'][1]['output'];assert hook['queued']==1 and len(hook['requests'])==1
    assert json.loads(hook['requests'][0]['body'])['handoff']['summary']=='Halfway' and cases['handoff_webhook']['state']['ledger'][-1]['webhook_status']=='delivered'
    rest=cases['handoff_throttle']['observations'];assert [o['status'] for o in rest]==[201]*60+[429] and rest[-1]['response_headers']['Retry-After']=='60'
    mcp=cases['handoff_shared_throttle']['observations'];assert all(json.loads(o['response_body'])['result']['isError'] is False for o in mcp[:60]);assert json.loads(mcp[60]['response_body'])['result']['isError'] is True and mcp[61]['status']==429
    flow=cases['board_flow']['observations'];assert wire('board_flow',1)==dict(title=True,owner=True,badge='agent') and wire('board_flow',2)=={'brief':True,'run':[['https://example.com/runs/11','Run']]}
    history=wire('board_flow',3);assert history[0]['event_type']=='work_assignment' and history[0]['actor']['name']=='Bender Bot' and history[0]['after']['owner']['id']==394959859
    assert flow[4]['status']==302 and flow[5]['status']==201 and flow[6]['status']==200
    assert wire('board_flow',7)==dict(reply=True,result='Shipped on Friday',history=True)
    assert flow[-1]['output']['status']==200 and len(flow[-1]['output']['items'])==1 and flow[-1]['output']['items'][0]['event_type']=='work_update' and flow[-1]['output']['items'][0]['present'] is True
    assert all(o.get('cache_hits',0)==0 for r in rows for o in r['observations'])
    print('WS11 next6 original assertion premises: 31 declarations; real positive selections, mutations, delivery and HTML/JSON projections; 0 missing clauses')

if __name__=='__main__':check()
