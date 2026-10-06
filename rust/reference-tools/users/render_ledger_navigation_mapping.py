#!/usr/bin/env python3
"""Regenerate exact pinned navigation assertion, helper and prerequisite anchors."""
import hashlib,json,os,re,subprocess,sys
from pathlib import Path
root=Path(__file__).resolve().parents[3]
os.chdir(root)
sys.path.insert(0,str(root/'rust/reference-tools/users'))
from ledger_browser_navigation import CASES
pin=Path('rust/parity/reference.sha').read_text().strip()
mjs=Path('rust/reference-tools/users/ledger_browser_navigation.mjs');js=mjs.read_text().splitlines()
py=Path('rust/reference-tools/users/ledger_browser_navigation.py');python=py.read_text().splitlines()
test='controllers::ledger_browser_tests::original_ledger_navigation_assertions'
files={'K':'keyboard_shortcuts','M':'sidebar_room_menu','N':'channel_navigation','H':'room_header','U':'unread_rooms'}
start={'K':0,'M':14,'N':30,'H':37,'U':46}
source={}
for alias,name in files.items():source[alias]=subprocess.check_output(['git','show',pin+':test/system/'+name+'_test.rb'],text=True)
helper_src=subprocess.check_output(['git','show',pin+':test/test_helpers/system_test_helper.rb'],text=True)
records=[];current=[]
ledger=json.load(open('rust/plans/ledger-ws8br-ws17-ws11ui-remaining.json'))
def entries(x):
 if isinstance(x,dict):
  if 'id' in x and 'test' in x:yield x
  for v in x.values():yield from entries(v)
 elif isinstance(x,list):
  for v in x:yield from entries(v)
original={r['test']:r for r in entries(ledger)}
def anchor(alias,line,ruby):
 pats=[rf'\b{alias},{line}\b',rf'\[{line},']
 for pat in pats:
  hits=[(i+1,s.strip()) for i,s in enumerate(js) if re.search(pat,s)]
  if hits:return hits[0]
 # Every private helper call is an actual assertion procedure with its own
 # predicate anchors, rather than a presence-only fallback.
 function={'assert_visible_nav_actions':'navActions','assert_hidden_nav_actions':'navActions','assert_search_visible':'searchVisible','assert_visible_trailing_nav_actions':'trailing','assert_hidden_trailing_nav_actions':'trailing','assert_no_inner_scroll':'innerScroll','assert_no_horizontal_overflow':'horizontal','assert_header_inside_viewport':'inside','assert_fetch_was_delayed':'delayed','assert_no_progress_bar_shown':'noBar','assert_sentinel_alive':'sentinel','assert_progress_bar_suppression_released':'released','assert_single_badge':'singleBadge','assert_no_badge':'noBadge','assert_badge_beside_stack':'badgeGeometry'}.get(ruby.split()[0])
 if function:
  hits=[(i+1,s.strip()) for i,s in enumerate(js) if re.search(rf'\b{function}\({line},p\b',s)]
  if hits:return hits[0]
 raise ValueError((alias,line,ruby))
def q(n,s):return {'path':str(mjs)+':'+str(n),'anchor':s}
eq=q(*next((i+1,s.strip()) for i,s in enumerate(js) if s.startswith('function equal(')))
def premise(alias,n,token,caller):
 f='test/test_helpers/system_test_helper.rb' if alias=='S' else 'test/system/'+files[alias]+'_test.rb'
 src=helper_src if alias=='S' else source[alias]
 a=next((i+1,t.strip()) for i,t in enumerate(js) if token in t)
 return {'ruby_file':f,'line':n,'ruby':src.splitlines()[n-1].strip(),'caller_line':caller,'rust_helper':q(*a)}
def hs(alias,n,caller,origin=None):
 f='test/test_helpers/system_test_helper.rb' if alias=='S' else 'test/system/'+files[alias]+'_test.rb'
 src=helper_src if alias=='S' else source[alias]
 ruby=src.splitlines()[n-1].strip()
 if alias=='S' and n in [41,127,131]:
  token='p.waitForFunction(s=>document.activeElement' if n==41 else 'await yes(file,line,p.locator(unread?'
  a=next((i+1,t.strip()) for i,t in enumerate(js) if token in t)
  return {'ruby_file':f,'line':n,'ruby':ruby,'caller_line':caller,'rust_assertion':q(*a)}
 try:a=anchor(alias,n,ruby)
 except ValueError:
  if n==123 and origin:
   a=anchor(origin,caller,'assert_message_text')
   return {'ruby_file':f,'line':n,'ruby':ruby,'caller_line':caller,'rust_assertion':q(*a)}
  # External room/focus assertions are implemented inside exact shared helpers.
  tok={41:'p.waitForFunction(s=>document.activeElement',123:'.message[data-message-id] .message__body',127:'await yes(file,line,p.locator(unread?',131:'await yes(file,line,p.locator(unread?'}[n]
  a=next((i+1,s.strip()) for i,s in enumerate(js) if tok in s)
 return {'ruby_file':f,'line':n,'ruby':ruby,'caller_line':caller,'rust_assertion':q(*a)}
private={
 'H':{'assert_visible_nav_actions':[424],'assert_hidden_nav_actions':[418],'assert_search_visible':[413],'assert_visible_trailing_nav_actions':[431,432],'assert_hidden_trailing_nav_actions':[436,437],'assert_no_inner_scroll':[444],'assert_no_horizontal_overflow':[453],'assert_header_inside_viewport':[461,462]},
 'N':{'assert_fetch_was_delayed':[213],'assert_no_progress_bar_shown':[217],'assert_sentinel_alive':[221],'assert_progress_bar_suppression_released':[227]},
 'U':{'assert_single_badge':[137],'assert_no_badge':[143],'assert_badge_beside_stack':[161,162]}}
for alias,src in source.items():
 decl=list(re.finditer(r'^  test "([^"]+)" do$',src,re.M))
 for number,d in enumerate(decl):
  name=d[1];line=src[:d.start()].count('\n')+1
  all_decl=list(re.finditer(r'^  test "([^"]+)" do$',src,re.M));pos=next(i for i,x in enumerate(all_decl) if x.start()==d.start())
  end=all_decl[pos+1].start() if pos+1<len(all_decl) else src.find('\n  private',d.end());end=len(src) if end<0 else end
  body=src[d.start():end]
  assertions=[(line+i,s.strip()) for i,s in enumerate(body.splitlines()) if s.strip().startswith(('assert','refute')) or (s.strip().startswith('within') and re.search(r'\b(?:assert(?:_\w+)?|refute(?:_\w+)?)\b',s))]
  case=CASES[start[alias]+number] if alias!='U' else {'sending messages between two users':'unread-between','a live message in the current room leaves its row without a New badge':'unread-live','channel and DM rows keep exactly one unread badge next to the huddle stack':'unread-badges'}[name]
  legacy=name in original
  rec={'id':original[name]['id'] if legacy else 'runtime-unread-live','file':'test/system/'+files[alias]+'_test.rb','line':line,'ledger_declaration_line':original[name]['line'] if legacy else None,'test':name,'ledger':'rust/plans/ws8br-rails-cases.json','source_sha256':hashlib.sha256(src.encode()).hexdigest(),'record_status':'acceptance-receipt-pending','rust_tests':[test],'browser_case':case,'browser_mode':'navigation','execution':'Paired pinned Rails/Rust browser gate; pending final baseline and producer control validation.','entry_outer_window':[1400,1400],'viewport_contract':'Measured original WebDriver outer windows; browser receives their actual inner viewports from original.window_viewports, without an assumed chrome offset.','fixture_scope':'Fresh pinned Rails fixture foundation per wrapper; each declaration restores its original rows and only its own original direct-create fixtures.','assertions':[],'helper_expansion':[],'setup_expansion':[],'interaction_helper_sources':[],'original_declaration_source':body}
  signins=[line+i for i,t in enumerate(body.splitlines()) if t.strip().startswith('sign_in ')]
  if alias in ['K','N','U']:signins.insert(0,{'K':6,'N':8,'U':5}[alias])
  rec['setup_expansion']=[hs('S',65,n) for n in signins]
  if case=='unread-live':
   n,anchor_line=next((i+1,t.strip()) for i,t in enumerate(js) if 'window.unreadEventTargets.includes(dom)' in t)
   rec['interaction_helper_sources'] += [{'ruby_file':'test/system/unread_rooms_test.rb','line':129,'ruby':src.splitlines()[128].strip(),'rust_helper':q(n,anchor_line)}]
  for join_line in [line+i for i,t in enumerate(body.splitlines()) if t.strip().startswith('join_room ')]:
   rec['interaction_helper_sources'] += [premise('S',75,'const all=document.querySelectorAll',join_line),premise('S',203,'data-pwa-install-target',join_line)]
  if alias=='M':
   for wait_line in [77,239,281]:
    if line<=wait_line<line+len(body.splitlines()):rec['interaction_helper_sources'].append(premise('M',wait_line,'assertionWait(M,'+str(wait_line),wait_line))
   for match in re.finditer(r'^[ \t]+confirm_leave_and_wait_for_removal',body,re.M):
    caller=line+body[:match.start()].count('\n');rec['interaction_helper_sources'].append(premise('M',344,'expected audit within 10s',caller))
   if case=='menu-kinds-leave':rec['interaction_helper_sources'].append(premise('M',359,'const all=document.querySelectorAll',282))
  if alias=='K':
   for caller in [line+i for i,t in enumerate(body.splitlines()) if t.strip().startswith('mark_current_room_unread ')]:rec['interaction_helper_sources'].append(premise('K',270,'?.channel',caller))
  if case=='header-phone':rec['viewport_matrix']=[[360,800],[390,800],[500,800],[320,740]]
  elif case=='header-tablet':rec['viewport_matrix']=[[700,800],[768,800],[1024,800]]
  elif case=='header-desktop':rec['viewport_matrix']=[[1280,900],[1400,900]]
  elif case=='header-landscape':rec['viewport_matrix']=[[640,360]]
  elif case.startswith('header-') or case in ['nav-phone','menu-phone-delete']:rec['viewport_matrix']=[[390,844]]
  elif case=='key-fullscreen':rec['fullscreen_native_screen']=[800,600]
  if 'viewport_matrix' in rec:rec['outer_window_matrix']=rec.pop('viewport_matrix')
  # Non-asserting fixture/setup statements retain their own exact source
  # coordinates, timing and executable implementation anchors.
  rec['fixture_setup_sources']=[]
  for offset,ruby in enumerate(body.splitlines()):
   if '.create_for(' in ruby or 'tour_completed_at: nil' in ruby:
    f= mjs if case=='unread-badges' else py
    token='function createUnreadDirect(' if case=='unread-badges' else "db.execute('UPDATE users SET tour_completed_at=NULL" if 'tour_completed_at' in ruby else "db.execute('INSERT INTO rooms("
    lines=js if f==mjs else python
    n,t=next((i+1,t.strip()) for i,t in enumerate(lines) if token in t)
    rec['fixture_setup_sources'].append({'ruby_file':rec['file'],'line':line+offset,'ruby':ruby.strip(),'timing':'After class sign_in and before join_room HQ' if case=='unread-badges' else 'Before declaration sign_in','implementation':{'path':str(f)+':'+str(n),'anchor':t}})
  rec['environment_setup_sources']=[]
  if alias=='H' or case=='unread-badges':
   envlines=range(7,12) if alias=='H' else range(102,107)
   for n in envlines:
    ruby=src.splitlines()[n-1].strip();key=re.search(r'ENV\["([^"]+)"\]',ruby)[1]
    env_file=py if alias=='H' else mjs
    env_source=python if alias=='H' else js
    token="'"+key+"':" if alias=='H' else key+":'"
    at,t=next((i+1,t.strip()) for i,t in enumerate(env_source) if token in t)
    rec['environment_setup_sources'].append({'ruby_file':rec['file'],'line':n,'ruby':ruby,'timing':'Original header class setup' if alias=='H' else 'Original with_huddle_configured block','implementation':{'path':str(env_file)+':'+str(at),'anchor':t}})
  rec['environment_restoration_sources']=[]
  if case=='unread-badges':
   n,t=next((i+1,t.strip()) for i,t in enumerate(js) if 'if(previousHuddle)await fixtureAction' in t)
   rec['environment_restoration_sources'].append({'ruby_file':rec['file'],'line':110,'ruby':src.splitlines()[109].strip(),'implementation':q(n,t)})
  auth_n,auth_t=next((i+1,t.strip()) for i,t in enumerate(js) if "base+'/test_session?'" in t)
  rec['authentication_sources']=[{'ruby_file':'test/test_helpers/system_test_helper.rb','line':64,'ruby':helper_src.splitlines()[63].strip(),'implementation':q(auth_n,auth_t)}]
  rec['visibility_provenance']='rust/reference-tools/users/selenium-is-displayed.provenance.json'
  rec['text_provenance']='rust/reference-tools/users/selenium-get-text.provenance.json'
  for n,ruby in assertions:
   a=anchor(alias,n,ruby)
   rec['assertions'].append({'line':n,'ruby':ruby,'rust_test':test,'checks':'Original fixture, interaction order, scoped selector, text, visibility, focus, viewport/geometry, path, or same-server persisted predicate. Private assertions expanded below.','observation':ruby,'assertion_source':str(mjs)+':'+str(a[0]),'assertion_anchor':a[1],'assertion_scope':case,'additional_assertion_sources':[eq],'browser_case':case})
   for hn in private.get(alias,{}).get(ruby.split()[0],[]):rec['helper_expansion'].append(hs(alias,hn,n))
   if ruby.startswith('assert_focused'):rec['helper_expansion'].append(hs('S',41,n))
   if ruby.startswith('assert_room_read'):rec['helper_expansion'].append(hs('S',127,n))
   if ruby.startswith('assert_room_unread'):rec['helper_expansion'].append(hs('S',131,n))
   if ruby.startswith('assert_message_text'):rec['helper_expansion'].append(hs('S',123,n,alias))
  if alias=='K':
   for match in re.finditer(r'^[ \t]+mark_current_room_unread ',body,re.M):
    caller=line+body[:match.start()].count('\n')
    rec['helper_expansion'] += [hs('K',259,caller),hs('K',260,caller),hs('S',131,caller),hs('S',141,caller),hs('S',142,caller)]
   if case=='key-menu-escape':rec['helper_expansion'] += [hs('S',141,106),hs('S',142,106)]
   if case=='key-unread-rooms':rec['helper_expansion'].append(hs('S',131,72))
  if alias=='M':
   for match in re.finditer(r'^[ \t]+open_room_menu ',body,re.M):
    caller=line+body[:match.start()].count('\n');rec['helper_expansion'].append(hs('M',322,caller))
   for match in re.finditer(r'^[ \t]+confirm_leave_and_wait_for_removal',body,re.M):
    caller=line+body[:match.start()].count('\n');rec['helper_expansion'].append(hs('M',338,caller))
  (records if legacy else current).append(rec)
implementation_files=['ledger_browser_navigation.mjs','ledger_browser_navigation.py','selenium_displayed.mjs','selenium-is-displayed.js','selenium-is-displayed.provenance.json','selenium-get-text.js','selenium-get-text.provenance.json','ledger_browser_viewports.mjs','ledger_browser_viewports.py','render_ledger_navigation_mapping.py']
implementation_hashes={str(Path('rust/reference-tools/users')/name):hashlib.sha256((Path('rust/reference-tools/users')/name).read_bytes()).hexdigest() for name in implementation_files}
json.dump({'reference':pin,'runtime_reference':pin,'implementation_sha256':implementation_hashes,'stack_base':'5f5c18635','gate':'controllers::ledger_browser_tests::original_ledger_navigation_assertions through rust/parity/system/ws12; required ignored correctness selector integration is owned by the lead.','records':records,'current_pin_records':current},open('rust/plans/ledger-ws8br-ws17-ws11ui-d-navigation-receipts.json','w'),indent=2)
print(len(records),'legacy declarations',len(current),'current pin declarations',sum(len(r['assertions']) for r in records),'direct assertions',sum(len(r['helper_expansion']) for r in records),'helper expansions')
