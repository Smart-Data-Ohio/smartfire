"""Pinned navigation fixtures and persisted observations for the original browser runner."""
import hashlib
import json
from ledger_browser_lifecycle import host_command

CASES = [
 'key-help','key-typing-help','key-typing-switcher','key-rooms','key-unread-rooms',
 'key-read','key-typing-escape','key-menu-escape','key-theater','key-fullscreen',
 'key-typing-arrows','key-ime','key-modal','key-toggle',
 'menu-admin-delete','menu-creator','menu-member','menu-cancel','menu-current-delete',
 'menu-kinds-delete','menu-group-permission','menu-dm-permission','menu-keyboard',
 'menu-phone-delete','menu-open-leave','menu-private-leave','menu-current-leave',
 'menu-solo-leave','menu-kinds-leave','menu-group-leave',
 'nav-switch','nav-back','nav-history','nav-cancel','nav-shared','nav-phone','nav-nonroom',
 'header-phone','header-tablet','header-desktop','header-items','header-landscape',
 'header-return-focus','header-keyboard','header-dot','header-tour',
 'unread-between','unread-badges','unread-live',
]
# Bounded final discrimination gate: twelve distinct original declarations.
# Geometry and scope variants run by their explicit --case/--mutation commands.
CONTROL_CASES = [case for case in CASES[:14] if case not in {'key-ime','key-modal'}]
SCRIPT = 'ledger_browser_navigation.mjs'
MUTATIONS = {
 'help-no-open':'key-help', 'help-hidden':'key-help', 'switcher-no-open':'key-typing-switcher',
 'room-movement-disabled':'key-rooms', 'mark-read-disabled':'key-read',
 'theater-steals-escape':'key-theater', 'fullscreen-steals-escape':'key-fullscreen',
 'delete-hidden':'menu-admin-delete', 'delete-no-submit':'menu-admin-delete',
 'delete-wrong-confirmation':'menu-admin-delete', 'leave-no-submit':'menu-open-leave',
 'menu-foreign-scope':'menu-creator', 'navigation-reload':'nav-switch',
 'progress-suppression-disabled':'nav-switch', 'drawer-no-close':'nav-phone',
 'header-forward-disabled':'header-return-focus', 'header-hidden':'header-phone',
 'header-zero-width':'header-phone', 'header-out-of-frame':'header-phone',
 'header-menu-out-of-frame':'header-landscape', 'unread-duplicate-badge':'unread-badges',
 'unread-wrong-columns':'unread-badges',
}
# Labels reserve identifiers without adding rows to the original fixture seed.
ROOM_FIXTURES = [
 ('own','Rooms::Closed','jz',['jz','david'],'JZ Club'),
 ('board','Rooms::Board','david',['david','jz'],'Launch'),
 ('voice','Rooms::Voice','david',['david','jz'],'Lounge'),
 ('stage','Rooms::Stage','david',['david','jz'],'Town Hall'),
 ('solo','Rooms::Closed','jz',['jz'],'Solo'),
 ('permission_group','Rooms::Direct','jz',['jz','kevin','david'],'Weekend Plans'),
 ('leave_group','Rooms::Direct','david',['david','jason','jz'],'Weekend Plans'),
 ('unread_direct','Rooms::Direct','jz',['jz','kevin'],None),
]

def prepare(db, labels, root):
 for offset,(name,*_) in enumerate(ROOM_FIXTURES):
  labels['navigation.rooms.'+name]=9200000001+offset

def environment_for_case(case, labels, root):
 if not case.startswith('header-'):return {}
 return {'LIVEKIT_URL':'wss://huddle.example.test',
  'LIVEKIT_INTERNAL_URL':'ws://livekit.example.test:7880',
  'LIVEKIT_API_KEY':'test-api-key','LIVEKIT_API_SECRET':'test-api-secret',
  'LIVEKIT_GATEWAY_SECRET':'test-gateway-secret'}

def fixture(db, case, labels, root):
 assert case in CASES
 if case=='header-tour':
  db.execute('UPDATE users SET tour_completed_at=NULL WHERE id=?',[labels['users.jz']])
 # All these originals create before their explicit declaration sign_in.
 # The unread direct is created after class sign_in by the browser script.
 needed={'menu-creator':['own'],'menu-kinds-delete':['board','voice','stage'],
  'menu-group-permission':['permission_group'],'menu-solo-leave':['solo'],
  'menu-kinds-leave':['board','voice','stage'],'menu-group-leave':['leave_group'],
  'header-landscape':['stage']}.get(case,[])
 now=labels['clock.now']
 for name,kind,creator,members,title in ROOM_FIXTURES:
  if name not in needed:continue
  if case=='menu-kinds-delete':members=['david']
  ident=labels['navigation.rooms.'+name]
  ids=sorted(labels['users.'+u] for u in members)
  key='dm:'+hashlib.sha256(','.join(map(str,ids)).encode()).hexdigest() if kind=='Rooms::Direct' else None
  db.execute('INSERT INTO rooms(id,type,name,creator_id,direct_member_key,created_at,updated_at) VALUES (?,?,?,?,?,?,?)',
             [ident,kind,title,labels['users.'+creator],key,now,now])
  for u in members:
   db.execute('INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES (?,?,?,?,?)',
              [ident,labels['users.'+u],'everything' if kind=='Rooms::Direct' else 'mentions',now,now])
   if kind=='Rooms::Stage':db.execute('UPDATE memberships SET stage_role=? WHERE room_id=? AND user_id=?',['host' if u==creator else 'listener',ident,labels['users.'+u]])


def check_state(db, requests, labels, root):
 for q in requests:
  kind=q['kind'];room=q.get('room')
  if kind=='deleted':
   actual=db.execute('SELECT deleted_at FROM rooms WHERE id=?',[room]).fetchone()
   # The browser checked room.reload at the original assertion moment. A
   # production DestroyRoomJob may physically purge it after the case closes.
   assert (actual is None and q['value']) or (actual is not None and (actual[0] is not None)==q['value']),('post-case deletion consistency',q,actual)
  elif kind=='membership':
   actual=db.execute('SELECT user_id FROM memberships WHERE room_id=?',[room]).fetchall()
   if 'empty' in q:assert actual==[],('original empty memberships',q,actual)
   else:assert (labels['users.'+q['user']] in [x[0] for x in actual])==q['value'],('original membership predicate',q,actual)
  elif kind=='audit':
   rows=db.execute('SELECT actor_id,details FROM audit_logs WHERE action=? AND target_id=? ORDER BY id DESC',[q['action'],room]).fetchall()
   assert rows,('original persisted audit existence',q)
   if 'actor' in q:assert rows[0][0]==labels['users.'+q['actor']],('original audit actor',q,rows[0])
   if 'revoked' in q:assert json.loads(rows[0][1])['revoked']==q['revoked'],('original audit revoked labels',q,rows[0])
  elif kind=='involvement':
   actual=db.execute('SELECT involvement FROM memberships WHERE room_id=? AND user_id=?',[room,labels['users.'+q['user']]]).fetchone()
   assert actual==(q['value'],),('original explicit involvement',q,actual)
  else:raise AssertionError(q)
  print('ORIGINAL_ASSERTION '+q['file']+':'+str(q['line']),flush=True)
  print('ORIGINAL_DB '+json.dumps(q,sort_keys=True),flush=True)
