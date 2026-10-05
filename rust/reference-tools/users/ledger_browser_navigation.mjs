// Exact navigation declarations at rust/parity/reference.sha. The browser acts
// through the shipped controllers; direct SQLite writes below are Rails fixtures.
import assert from 'node:assert/strict'
import fs from 'node:fs'
import { createHash } from 'node:crypto'
import { DatabaseSync } from 'node:sqlite'
import { chromium } from 'playwright'
import { diagnostics } from './browser_diagnostics.mjs'
import { fixtureAction } from './ledger_fixture_bridge.mjs'
import { network } from './original_browser_network.mjs'
import { visit, waitForController } from './browser_navigation.mjs'
import { displayedCount } from './selenium_displayed.mjs'
import { viewportFor, resizeOriginal } from './ledger_browser_viewports.mjs'
const base=process.env.WS11UI_BROWSER_URL
const proxy=await network(base)
const labels=JSON.parse(fs.readFileSync(process.env.WS11UI_BROWSER_LABELS,'utf8'))
const selected=process.env.WS11UI_BROWSER_CASE
const db=new DatabaseSync(process.env.WS11UI_BROWSER_DATABASE)
const browser=await chromium.launch({headless:true,args:['--no-sandbox','--mute-audio','--window-size=1400,1400','--screen-info={800x600}']})
const requests=[]
const contextUsers=new WeakMap()
const K='test/system/keyboard_shortcuts_test.rb', M='test/system/sidebar_room_menu_test.rb'
const N='test/system/channel_navigation_test.rb', H='test/system/room_header_test.rb', U='test/system/unread_rooms_test.rb'
const S='test/test_helpers/system_test_helper.rb'
const id=n=>labels['navigation.rooms.'+n]??labels['rooms.'+n]
const user=n=>labels['users.'+n]
const roomPath=r=>'/rooms/'+r
const roomDom=r=>'list_'+db.prepare('SELECT type FROM rooms WHERE id=?').get(r).type.replaceAll('::','_').toLowerCase()+'_'+r
const row=(p,r)=>p.locator(`#sidebar a[data-room-id='${r}']`)
const menu=p=>p.locator('#room-menu')
const overflow=p=>p.locator('#header-overflow-menu')
async function assertionWait(file,line,promise){try{return await promise}catch(error){throw Error(`${file}:${line}: original assertion`,{cause:error})}}
function trace(file,line){console.log(`ORIGINAL_ASSERTION ${file}:${line}`)}
function equal(file,line,actual,expected){assert.deepEqual(actual,expected,`${file}:${line}: original assertion`);trace(file,line)}
async function matching(loc,value,all){
 return displayedCount(loc,value,all)
}
async function originalFind(items,label=null,first=false){
 const deadline=performance.now()+2000
 while(true){
  const matches=[]
  for(let i=0;i<await items.count();i++)if(await matching(items.nth(i),label,false))matches.push(i)
  if(matches.length){if(!first)assert.equal(matches.length,1,'original visible find must be unambiguous');return items.nth(matches[0])}
  if(performance.now()>=deadline)throw Error('original visible find: '+(label??items))
  await new Promise(r=>setTimeout(r,20))
 }
}
async function synchronized(file,line,wait,observe,predicate){
 const deadline=performance.now()+wait
 while(true){const value=await observe();if(predicate(value))return value
  if(performance.now()>=deadline)throw Error(`${file}:${line}: original assertion`)
  await new Promise(resolve=>setTimeout(resolve,20))
 }
}
async function yes(file,line,loc,text=null,wait=2000,all=false){
 const count=await synchronized(file,line,wait,()=>matching(loc,text,all),n=>n>0)
 equal(file,line,count>0,true)
}
async function no(file,line,loc,text=null,wait=2000,all=false){
 const count=await synchronized(file,line,wait,()=>matching(loc,text,all),n=>n===0)
 equal(file,line,count,0)
}
async function text(file,line,loc,value,wait=2000){await yes(file,line,loc,value,wait)}
async function focus(file,line,p,selector,wait=2000){
 await assertionWait(file,line,p.waitForFunction(s=>document.activeElement?.matches(s),selector,{timeout:wait}))
 equal(file,line,await p.evaluate(s=>document.activeElement?.matches(s),selector),true);trace(S,41)
}
async function title(file,line,p,value,wait=10000){
 await assertionWait(file,line,p.waitForFunction(v=>document.title.includes(v),value,{timeout:wait}))
 equal(file,line,(await p.title()).includes(value),true)
}
async function path(file,line,p,expected,wait=10000){
 await assertionWait(file,line,p.waitForURL(u=>expected instanceof RegExp?expected.test(u.pathname):u.pathname===expected,{timeout:wait}))
 equal(file,line,expected instanceof RegExp?expected.test(new URL(p.url()).pathname):new URL(p.url()).pathname,expected instanceof RegExp?true:expected)
}
async function streams(p,min=3,wait=15000){
 await p.waitForFunction(min=>{const all=document.querySelectorAll('turbo-cable-stream-source');return all.length>=min&&[...all].every(s=>s.hasAttribute('connected'))},min,{timeout:wait})
}
async function join(p,r){
 assert.equal((await visit(p,base+roomPath(r))).status(),200,'real join request');await streams(p)
 if(await matching(p.locator("[data-pwa-install-target~='dialog']"),null,false))await p.getByRole('button',{name:'Close',exact:true}).click()
}
async function signedContext(name='jz'){
 const c=await browser.newContext({viewport:selected==='key-fullscreen'?null:viewportFor(labels,1400,1400)})
 await c.route('**/*',r=>new URL(r.request().url()).origin===new URL(base).origin?r.continue():r.abort())
 contextUsers.set(c,name)
 return c
}
async function signIn(p,name=contextUsers.get(p.context())){contextUsers.set(p.context(),name);await visit(p,base+'/test_session?'+new URLSearchParams({email_address:name+'@37signals.com',password:'secret123456'}));await yes(S,65,p.locator('a.btn'),'Designers',10000)}
function state(file,line,kind,room,details={}){const key=q=>q.kind===kind&&q.room===room&&q.user===details.user;const old=requests.findIndex(key);if(old>=0)requests.splice(old,1);requests.push({file,line,kind,room,...details})}
function immediateState(file,line,kind,room,details={}){
 const q={file,line,kind,room,...details}
 let actual
 if(kind==='deleted'){actual=db.prepare('SELECT deleted_at FROM rooms WHERE id=?').get(room);equal(file,line,actual!==undefined&&((actual.deleted_at!==null)===details.value),true)}
 else if(kind==='membership'){
  actual=db.prepare('SELECT user_id FROM memberships WHERE room_id=?').all(room).map(x=>x.user_id)
  equal(file,line,details.empty?actual:actual.includes(user(details.user)),details.empty?[]:details.value)
 } else if(kind==='audit'){
  actual=db.prepare('SELECT actor_id,details FROM audit_logs WHERE action=? AND target_id=? ORDER BY id DESC LIMIT 1').get(details.action,room)
  equal(file,line,actual!==undefined,true)
  if(details.actor)equal(file,line,actual.actor_id,user(details.actor))
  if(details.revoked)equal(file,line,JSON.parse(actual.details).revoked,details.revoked)
 } else if(kind==='involvement')equal(file,line,db.prepare('SELECT involvement FROM memberships WHERE room_id=? AND user_id=?').get(room,user(details.user)).involvement,details.value)
 state(file,line,kind,room,details)
}
async function auditWait(room,action){
 const until=Date.now()+10000
 while(!db.prepare('SELECT id FROM audit_logs WHERE target_id=? AND action=? LIMIT 1').get(room,action)){
  assert.ok(Date.now()<until,`${M}:344: original assertion expected audit within 10s`)
  await new Promise(r=>setTimeout(r,50))
 }
 trace(M,344)
}
async function roomRead(file,line,p,r,unread=false,all=false){
 const name=db.prepare('SELECT name FROM rooms WHERE id=?').get(r).name
 await yes(file,line,p.locator(unread?'.rooms a.unread':'.rooms a:not(.unread)'),name,5000,all)
 trace(S,unread?131:127)
}
async function messageMenu(p,message){
 await (await originalFind(p.locator('#message_'+db.prepare('SELECT client_message_id FROM messages WHERE id=?').get(message).client_message_id).locator("[data-message-edit-format], [data-reply-target='body']"),null,true)).click({button:'right'})
 await yes(S,141,p.locator("[data-message-actions-target='menu']"),null,10000)
 await yes(S,142,p.locator('.message[data-message-actions-open]'))
}
async function markUnread(p,message){
 await p.waitForFunction(()=>!!window.Stimulus.getControllerForElementAndIdentifier(document.querySelector("[data-controller~='presence']"),'presence')?.channel,null,{timeout:10000})
 await messageMenu(p,message)
 await p.getByText('Mark unread',{exact:true}).click()
 await yes(K,259,p.locator("[data-message-actions-target='status']"),'Marked unread',10000,true)
 await roomRead(K,260,p,id('designers'),true)
}
async function send(p,value){
 await p.locator('#message_markdown_source').fill(value)
 const [response]=await Promise.all([p.waitForResponse(r=>r.request().method()==='POST'&&/^\/rooms\/\d+\/messages$/.test(new URL(r.url()).pathname)),p.getByRole('button',{name:'Send Message',exact:true}).click()])
 assert.equal(response.ok(),true,'real original send-message request completed')
}

const defaults={
 'key-help':'help-no-open','key-typing-help':'typing-opens-help','key-typing-switcher':'switcher-no-open',
 'key-rooms':'room-movement-disabled','key-unread-rooms':'room-movement-disabled','key-read':'mark-read-disabled',
 'key-typing-escape':'typing-marks-read','key-menu-escape':'message-menu-no-close','key-theater':'theater-steals-escape',
 'key-fullscreen':'fullscreen-steals-escape','key-typing-arrows':'typing-arrows-corrupt-editor','key-ime':'ime-opens-switcher',
 'key-modal':'modal-opens-switcher','key-toggle':'switcher-no-close',
 'menu-admin-delete':'delete-wrong-confirmation','menu-creator':'delete-hidden','menu-member':'delete-forbidden-visible',
 'menu-cancel':'cancel-no-close','menu-current-delete':'delete-no-submit','menu-kinds-delete':'delete-hidden',
 'menu-group-permission':'delete-hidden','menu-dm-permission':'delete-hidden','menu-keyboard':'menu-end-disabled',
 'menu-phone-delete':'delete-no-submit','menu-open-leave':'leave-wrong-confirmation','menu-private-leave':'leave-wrong-confirmation',
 'menu-current-leave':'leave-no-submit','menu-solo-leave':'leave-no-submit','menu-kinds-leave':'leave-no-submit','menu-group-leave':'leave-no-submit',
 'nav-switch':'progress-suppression-disabled','nav-back':'navigation-reload','nav-history':'progress-suppression-disabled',
 'nav-cancel':'suppression-not-released','nav-shared':'progress-suppression-disabled','nav-phone':'drawer-no-close','nav-nonroom':'progress-always-hidden',
 'header-phone':'header-hidden','header-tablet':'header-hidden','header-desktop':'header-help-hidden',
 'header-items':'header-forward-disabled','header-landscape':'header-menu-out-of-frame','header-return-focus':'header-forward-disabled',
 'header-keyboard':'header-button-disabled','header-dot':'pins-count-broken','header-tour':'tour-restart-disabled',
 'unread-between':'unread-handler-disabled','unread-live':'current-room-badge','unread-badges':'unread-duplicate-badge',
}
const mutation=process.env.WS11UI_BROWSER_MUTATION||(process.env.WS11UI_BROWSER_CONTROL==='1'?defaults[selected]:'')
async function mutate(p){
 if(!mutation)return
 const observation=await p.evaluate(({m,designers,hq})=>{
  const c=n=>{const el=document.querySelector(`[data-controller~='${n}']`);const x=el&&window.Stimulus.getControllerForElementAndIdentifier(el,n);if(!x)throw Error('INVALID_CONTROL missing '+n);return x}
  const replace=(n,method,fn=()=>{})=>{const x=c(n);if(typeof x[method]!=='function')throw Error('INVALID_CONTROL missing '+n+'.'+method);x[method]=fn}
  const css=s=>{const style=document.createElement('style');style.textContent=s;document.head.append(style)}
  const read=e=>{e.preventDefault();fetch(`/rooms/${document.querySelector("meta[name='current-room-id']").content}/read`,{method:'POST',headers:{Accept:'application/json',...(document.querySelector("meta[name='csrf-token']")?{'X-CSRF-Token':document.querySelector("meta[name='csrf-token']").content}:{})}})}
  if(m==='help-no-open')replace('keyboard-shortcuts','openHelp')
  else if(m==='help-hidden')css('#keyboard-shortcuts { visibility:hidden!important }')
  else if(m==='switcher-no-open')replace('quick-switcher','toggle')
  else if(m==='typing-opens-help')replace('keyboard-shortcuts','handle',e=>{if(e.key==='?')c('keyboard-shortcuts').openHelp()})
  else if(m==='room-movement-disabled')replace('keyboard-shortcuts','handle',e=>{if(e.altKey)e.preventDefault()})
  else if(m==='mark-read-disabled')replace('keyboard-shortcuts','handle',e=>{if(e.key==='Escape')e.preventDefault()})
  else if(m==='typing-marks-read')replace('keyboard-shortcuts','handle',e=>{if(e.key==='Escape'){c('rooms-list').read({detail:{roomId:document.querySelector("meta[name='current-room-id']").content}});read(e)}})
  else if(['theater-steals-escape','fullscreen-steals-escape'].includes(m))replace('keyboard-shortcuts','handle',e=>{if(e.key==='Escape')read(e)})
  else if(m==='typing-arrows-corrupt-editor')replace('keyboard-shortcuts','handle',e=>{if(e.altKey)document.getElementById('message_markdown_source').value=''})
  else if(m==='typing-navigates')replace('keyboard-shortcuts','handle',e=>{if(e.altKey)document.querySelector(`#sidebar a[data-room-id='${designers}']`).click()})
  else if(['ime-opens-switcher','modal-opens-switcher'].includes(m)){
   const x=c('keyboard-shortcuts');window.removeEventListener('keydown',x.onCaptureKeydown,true)
   x.onCaptureKeydown=e=>{if(e.ctrlKey&&e.key==='k'){e.preventDefault();e.stopPropagation();c('quick-switcher').open()}}
   window.addEventListener('keydown',x.onCaptureKeydown,true)
  } else if(m==='switcher-no-close')replace('quick-switcher','toggle',()=>c('quick-switcher').open())
  else if(m==='message-menu-no-close'){const x=c('message-actions');x.menuTarget.removeEventListener('keydown',x.onMenuKeydown);window.removeEventListener('keydown',x.onWindowKeydown)}
  else if(['delete-hidden','delete-forbidden-visible'].includes(m))document.querySelectorAll('#sidebar a[data-room-id]').forEach(el=>el.dataset.menuCanDelete=m==='delete-hidden'?'false':'true')
  else if(['delete-no-submit','leave-no-submit'].includes(m))replace('room-menu','confirmPending')
  else if(m==='delete-wrong-confirmation'||m==='leave-wrong-confirmation'){
   const x=c('room-menu'),method=m.startsWith('delete')?'askDelete':'askLeave',original=x[method].bind(x)
   replace('room-menu',method,(...args)=>{original(...args);x.confirmMessageTarget.textContent='Wrong room confirmation'})
  } else if(m==='cancel-no-close')replace('room-menu','cancelPending')
  else if(m==='menu-end-disabled'){const x=c('room-menu');x.menuTarget.removeEventListener('keydown',x.onMenuKeydown)}
  else if(m==='menu-foreign-scope'){
   const x=c('room-menu'),original=x.onContextMenu;x.element.removeEventListener('contextmenu',original)
   x.element.addEventListener('contextmenu',e=>{e.preventDefault();const foreign=x.menuTarget.cloneNode(true);foreign.removeAttribute('id');foreign.hidden=false;document.body.append(foreign)})
  } else if(m==='navigation-reload')document.querySelectorAll('#sidebar a[data-room-id]').forEach(a=>a.dataset.turbo='false')
  else if(m==='progress-suppression-disabled')css('html[data-channel-navigation] .turbo-progress-bar { display:block!important }')
  else if(m==='progress-always-hidden')css('.turbo-progress-bar { display:none!important }')
  else if(m==='suppression-not-released')document.documentElement.setAttribute('data-channel-navigation','')
  else if(m==='drawer-no-close')replace('workspace-navigation','navigate')
  else if(m==='header-forward-disabled')replace('header-overflow','forward')
  else if(m==='header-hidden')css('#header-overflow-button { display:none!important }')
  else if(m==='header-zero-width')css('.room-header__actions { position:absolute!important;left:0!important;right:auto!important;width:0!important;max-width:0!important;overflow:visible!important }')
  else if(m==='header-out-of-frame')css('#nav { transform:translateX(-25px)!important }')
  else if(m==='header-menu-out-of-frame')css('#header-overflow-menu { max-height:none!important;height:600px!important;inset-block-start:0!important }')
  else if(m==='header-help-hidden')css('#help-menu-button { visibility:hidden!important }')
  else if(m==='header-button-disabled')replace('header-overflow','buttonKeydown')
  else if(m==='pins-count-broken'){
   const x=c('header-overflow');x.pinsCountTarget.hidden=false;x.pinsCountTarget.textContent='0';x.toggle=(e)=>{e.preventDefault();x.menuTarget.hidden=false}
  } else if(m==='tour-restart-disabled')replace('tour','start')
  else if(['unread-duplicate-badge','current-room-badge'].includes(m)){
   const x=c('workspace-navigation'),original=x.updateUnreadStatus.bind(x)
   replace('workspace-navigation','updateUnreadStatus',e=>{original(e);const row=document.getElementById(e.detail.targetId);if(row){const badge=document.createElement('span');badge.className='sidebar-item__status';badge.textContent='New';row.append(badge)}})
  } else if(m==='unread-handler-disabled'){const x=c('rooms-list');if(!x.channel)throw Error('INVALID_CONTROL unread channel absent');x.channel.received=()=>{}}
  else if(m==='unread-wrong-columns')css('.sidebar-item { grid-template-columns: 1fr 1fr!important }')
  else throw Error('INVALID_CONTROL unknown mutation '+m)
  return {actionsWidth:m==='header-zero-width'?document.querySelector('.room-header__actions').getBoundingClientRect().width:undefined,navLeft:m==='header-out-of-frame'?document.querySelector('#nav').getBoundingClientRect().left:undefined,helpVisibility:m==='help-hidden'?getComputedStyle(document.getElementById('keyboard-shortcuts')).visibility:undefined}
 },{m:mutation,designers:id('designers'),hq:id('hq')})
 console.log('ORIGINAL_MUTATION '+mutation+' '+JSON.stringify(observation))
}
function createUnreadDirect(){
 const room=id('unread_direct'),members=[user('jz'),user('kevin')].sort((a,b)=>a-b),now=labels['clock.now']
 const key='dm:'+createHash('sha256').update(members.join(',')).digest('hex')
 db.prepare('INSERT INTO rooms(id,type,name,creator_id,direct_member_key,created_at,updated_at) VALUES (?, ?, NULL, ?, ?, ?, ?)').run(room,'Rooms::Direct',user('jz'),key,now,now)
 for(const member of members)db.prepare('INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES (?, ?, ?, ?, ?)').run(room,member,'everything',now,now)
}
async function scenario(fn,{as='jz',room=id('hq')}={}){
 const ctx=await signedContext(as),p=await ctx.newPage();p.setDefaultTimeout(10000)
 const diagnose=diagnostics(p),bad=[];let previousHuddle
 p.on('pageerror',e=>bad.push(String(e)))
 p.on('requestfailed',r=>{if(new URL(r.url()).pathname.startsWith('/assets/')&&r.failure()?.errorText!=='net::ERR_ABORTED')bad.push(r.failure().errorText)})
 p.on('response',r=>{if(r.status()>=500)bad.push(`${r.status()} ${r.url()}`)})
 try{
  await signIn(p);if(selected==='unread-badges'){const change=await fixtureAction({action:'environment',values:{LIVEKIT_URL:'wss://huddle.example.test',LIVEKIT_INTERNAL_URL:'ws://livekit.example.test:7880',LIVEKIT_API_KEY:'test-api-key',LIVEKIT_API_SECRET:'test-api-secret',LIVEKIT_GATEWAY_SECRET:'test-gateway-secret'}});previousHuddle=change.previous;createUnreadDirect()}await join(p,room);await waitForController(p,'keyboard-shortcuts');await waitForController(p,'quick-switcher');await waitForController(p,'room-menu');await waitForController(p,'header-overflow');await mutate(p)
  await fn(p,ctx)
  equal('transport',0,bad,[]);console.log(`ORIGINAL_CASE ${selected}: passed`)
 }catch(e){await diagnose(e);if(bad.length)throw Error('INVALID_CONTROL transport: '+JSON.stringify(bad),{cause:e});throw e}
 finally{try{if(previousHuddle)await fixtureAction({action:'environment',values:previousHuddle})}finally{await ctx.close()}console.log('ORIGINAL_PRODUCER_RESTORED '+selected+': browser context disposed')}
}

async function keyboard(p){
 const editor=p.locator('#message_markdown_source')
 if(selected==='key-help'){
  await row(p,id('hq')).press('?');await yes(K,16,p.locator('#keyboard-shortcuts[open]'),null,5000)
  for(const [line,label] of [[18,'Quick switcher'],[19,'Previous / next room'],[20,'Previous / next unread room'],[21,'Mark current room read'],[22,'Message menu']])await text(K,line,p.locator('#keyboard-shortcuts'),label)
  await p.keyboard.press('Escape');await no(K,26,p.locator('#keyboard-shortcuts[open]'),null,5000)
 }else if(selected==='key-typing-help'){
  await editor.fill('');await editor.press('?');await no(K,34,p.locator('#keyboard-shortcuts[open]'),null,2000);equal(K,35,await assertionWait(K,35,originalFind(p.locator('#message_markdown_source:enabled')).then(field=>field.inputValue())),'?')
 }else if(selected==='key-typing-switcher'){
  await editor.click();await editor.pressSequentially('hello');await editor.press('Control+k')
  await yes(K,46,p.locator('#quick-switcher[open]'),null,5000);equal(K,47,await editor.inputValue(),'hello')
 }else if(selected==='key-rooms'){
  await p.locator('.room-header__name').click();await p.keyboard.press('Alt+ArrowUp')
  await yes(K,56,p.locator('.room-header__name'),'Designers',10000);await focus(K,59,p,'#message_markdown_source',10000)
  await p.locator('.room-header__name').click();await p.keyboard.press('Alt+ArrowDown');await yes(K,63,p.locator('.room-header__name'),'HQ',10000)
 }else if(selected==='key-unread-rooms'){
  db.prepare('UPDATE memberships SET connected_at=NULL,connections=0 WHERE user_id=? AND room_id=?').run(user('jz'),id('designers'))
  const created=await fixtureAction({action:'message',room:id('designers'),creator:user('kevin'),body:'Unread me',key:'unread-jump-1'})
  // Original broadcast_until_unread re-emits this same producer broadcast until
  // this room's badge lands; no unrelated event is substituted for the premise.
  const deadline=Date.now()+10000
  while(true){
   await fixtureAction({action:'broadcast',message:created.id})
   try{await roomRead(S,131,p,id('designers'),true);break}catch(e){if(Date.now()>=deadline)throw e}
  }
  await p.locator('.room-header__name').click();await p.keyboard.press('Alt+Shift+ArrowDown');await yes(K,76,p.locator('.room-header__name'),'Designers',10000)
 }else if(['key-read','key-typing-escape','key-menu-escape','key-theater','key-fullscreen'].includes(selected)){
  const message=labels['messages.'+(selected==='key-read'?'second':'first')]
  await markUnread(p,message)
  if(selected==='key-read'){await p.keyboard.press('Escape');await roomRead(K,86,p,id('designers'))}
  else if(selected==='key-typing-escape'){await editor.click();await p.keyboard.press('Escape');await roomRead(K,97,p,id('designers'),true)}
  else if(selected==='key-menu-escape'){
   await messageMenu(p,labels['messages.second']);await p.keyboard.press('Escape')
   await no(K,108,p.locator('.message[data-message-actions-open]'),null,5000);await roomRead(K,109,p,id('designers'),true)
  }else if(selected==='key-theater'){
   await p.evaluate(()=>{let panel=document.getElementById('channel-huddle');if(!panel){panel=document.createElement('aside');panel.id='channel-huddle';document.body.append(panel)}panel.classList.add('huddle--theater');window.__theaterEscapePrevented='unseen';window.addEventListener('keydown',e=>{if(e.key==='Escape')window.__theaterEscapePrevented=e.defaultPrevented})})
   await p.keyboard.press('Escape');equal(K,138,await p.evaluate(()=>window.__theaterEscapePrevented),false);await roomRead(K,139,p,id('designers'),true)
   await p.evaluate(()=>document.getElementById('channel-huddle').classList.remove('huddle--theater'));await p.keyboard.press('Escape');await roomRead(K,146,p,id('designers'))
  }else{
   await p.evaluate(()=>document.documentElement.requestFullscreen());const fullscreenDriver=await p.context().newCDPSession(p);await fullscreenDriver.send('Emulation.clearDeviceMetricsOverride');console.log('ORIGINAL_FULLSCREEN_DRIVER '+JSON.stringify(await p.evaluate(()=>({width:innerWidth,height:innerHeight,screen:[screen.width,screen.height],fullscreen:!!document.fullscreenElement}))));equal(K,156,await p.evaluate(()=>document.fullscreenElement!==null),true)
   await p.waitForFunction(()=>!matchMedia('(min-width: 80rem)').matches,null,{timeout:10000})
   await no(K,166,p.locator('body.member-panel-open'),null,5000);await no(K,169,p.locator(".thread-panel__surface[aria-modal='true']"),null,2000,true)
   await p.evaluate(()=>{window.__fullscreenEscapePrevented='unseen';window.addEventListener('keydown',e=>{if(e.key==='Escape')window.__fullscreenEscapePrevented=e.defaultPrevented});window.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true,cancelable:true}))})
   equal(K,182,await p.evaluate(()=>window.__fullscreenEscapePrevented),false);await yes(K,185,p.locator('.rooms a.unread'),'Designers',5000,true)
   await p.evaluate(()=>document.exitFullscreen().catch(()=>{}))
  }
 }else if(selected==='key-typing-arrows'){
  await editor.click();await editor.pressSequentially('hello');await p.keyboard.press('Alt+ArrowUp')
  await yes(K,198,p.locator('.room-header__name'),'HQ',5000);equal(K,199,await assertionWait(K,199,originalFind(p.locator('#message_markdown_source:enabled')).then(field=>field.inputValue())),'hello')
  await p.keyboard.press('Alt+Shift+ArrowDown');await yes(K,202,p.locator('.room-header__name'),'HQ',5000)
 }else if(selected==='key-ime'){
  await p.evaluate(()=>document.activeElement.dispatchEvent(new KeyboardEvent('keydown',{key:'k',ctrlKey:true,isComposing:true,bubbles:true,cancelable:true})))
  await no(K,214,p.locator('#quick-switcher[open]'),null,2000)
 }else if(selected==='key-modal'){
  await row(p,id('hq')).press('?');await yes(K,222,p.locator('#keyboard-shortcuts[open]'),null,5000)
  await p.keyboard.press('Control+k');await no(K,225,p.locator('#quick-switcher[open]'),null,2000);await yes(K,226,p.locator('#keyboard-shortcuts[open]'))
  await p.keyboard.press('Escape');await no(K,229,p.locator('#keyboard-shortcuts[open]'),null,5000)
 }else if(selected==='key-toggle'){
  await p.keyboard.press('Control+k');await yes(K,236,p.locator('#quick-switcher[open]'),null,5000)
  await p.keyboard.press('Control+k');await no(K,239,p.locator('#quick-switcher[open]'),null,5000)
 }
}
async function delay(p,pattern,ms){
 await p.evaluate(({pattern,ms})=>{
  if(window.__turboResponseDelayerInstalled)return
  window.__turboResponseDelayerInstalled=true;window.__fetchDelayedCount=0
  const regex=new RegExp(pattern)
  document.addEventListener('turbo:before-fetch-request',e=>{
   let pathname;try{pathname=new URL(e.detail.url,window.location.origin).pathname}catch{return}
   if(!regex.test(pathname))return
   window.__fetchDelayedCount+=1
   const url=e.detail.url,options=e.detail.fetchOptions
   e.detail.fetchRequest={response:new Promise((resolve,reject)=>setTimeout(()=>window.fetch(url,options).then(resolve,reject),ms))}
  })
 },{pattern,ms})
}
async function watch(p){
 await p.evaluate(()=>{
  window.__progressBarSeen=0;window.__progressBarObserver?.disconnect()
  const o=new MutationObserver(ms=>{for(const m of ms)for(const node of m.addedNodes){if(node.nodeType!==1)continue;const bars=node.matches('.turbo-progress-bar')?[node]:[...node.querySelectorAll('.turbo-progress-bar')];for(const b of bars)if(getComputedStyle(b).display!=='none')window.__progressBarSeen+=1}})
  o.observe(document.documentElement,{childList:true,subtree:true});window.__progressBarObserver=o
 })
}
async function delayed(line,p){equal(N,213,(await p.evaluate(()=>window.__fetchDelayedCount))>=1,true);trace(N,line)}
async function noBar(line,p){equal(N,217,await p.evaluate(()=>window.__progressBarSeen),0);trace(N,line)}
async function sentinel(line,p){equal(N,221,await p.evaluate(()=>window.__channelNavSentinel),'alive');trace(N,line)}
async function released(line,p){await no(N,227,p.locator('html[data-channel-navigation]'),null,10000,true);trace(N,line)}
async function navigation(p){
 if(selected==='nav-switch')await p.evaluate(()=>window.__channelNavSentinel='alive')
 if(selected==='nav-switch'){
  await delay(p,'^/rooms/[^/]+/?$',1500);await watch(p);await p.locator('#sidebar').getByRole('link',{name:'Designers',exact:true}).click()
  await title(N,22,p,'Designers');await yes(N,23,p.locator('.room-header__name'),'Designers',10000)
  await yes(N,24,p.locator('.message[data-message-id] .message__body'),"Third time's a charm.",10000);trace(S,123)
  await yes(N,26,p.locator('#sidebar a[aria-current="page"]'),'Designers',10000)
  await delayed(29,p);await noBar(30,p);await sentinel(31,p);await released(32,p)
 }else if(selected==='nav-back'){
  await p.locator('#sidebar').getByRole('link',{name:'Designers',exact:true}).click();await title(N,41,p,'Designers')
  await p.evaluate(()=>window.__channelNavSentinel='alive');await p.goBack({waitUntil:'domcontentloaded'})
  await title(N,46,p,'HQ');await yes(N,47,p.locator('.room-header__name'),'HQ',10000)
  await yes(N,49,p.locator('#sidebar a[aria-current="page"]'),'HQ',10000);await sentinel(51,p)
 }else if(selected==='nav-history'){
  await p.locator('#sidebar').getByRole('link',{name:'Designers',exact:true}).click();await title(N,57,p,'Designers')
  await p.evaluate(()=>window.__channelNavSentinel='alive');await delay(p,'^/rooms/[0-9]+/?$',1200)
  for(const [direction,name] of [['back','HQ'],['forward','Designers']]){
   await p.evaluate(async()=>{const {Turbo}=await import('@hotwired/turbo-rails');Turbo.cache.clear()});await watch(p);await p.evaluate(()=>window.__fetchDelayedCount=0)
   if(direction==='back')await p.goBack({waitUntil:'domcontentloaded'});else await p.goForward({waitUntil:'domcontentloaded'})
   await title(N,70,p,name);await yes(N,71,p.locator('.room-header__name'),name,10000)
   await delayed(72,p);await noBar(73,p);await sentinel(74,p);await released(75,p)
  }
 }else if(selected==='nav-cancel'){
  await p.evaluate(()=>document.addEventListener('turbo:before-visit',e=>e.preventDefault(),{once:true}))
  await p.locator('#sidebar').getByRole('link',{name:'Designers',exact:true}).click();await title(N,85,p,'HQ',2000);await released(86,p)
  await delay(p,'^/activity/?$',1200);await watch(p);await p.locator('#sidebar').getByRole('link',{name:'Activity inbox',exact:true}).click()
  await title(N,91,p,'Activity inbox');await delayed(92,p);equal(N,93,(await p.evaluate(()=>window.__progressBarSeen))>=1,true)
 }else if(selected==='nav-shared'){
  await row(p,id('designers')).evaluate((el,href)=>el.href=href,roomPath(id('designers'))+'/@'+labels['messages.third'])
  await delay(p,'^/rooms/[0-9]+/@[0-9]+/?$',1200);await watch(p);await p.locator('#sidebar').getByRole('link',{name:'Designers',exact:true}).click()
  await title(N,104,p,'Designers');await yes(N,105,p.locator('.message[data-message-id] .message__body'),"Third time's a charm.",10000);trace(S,123)
  await delayed(106,p);await noBar(107,p);await released(108,p)
 }else if(selected==='nav-phone'){
  await resizeOriginal(p,390,844);await p.evaluate(()=>window.__channelNavSentinel='alive');await p.getByRole('button',{name:'Open workspace navigation',exact:true}).click();await yes(N,118,p.locator('#sidebar.open'))
  await p.locator('#sidebar').getByRole('link',{name:'Designers',exact:true}).click();await title(N,124,p,'Designers')
  await no(N,125,p.locator('#sidebar.open'));await sentinel(126,p)
 }else if(selected==='nav-nonroom'){
  await delay(p,'^/activity/?$',1200);await watch(p);await p.locator('#sidebar').getByRole('link',{name:'Activity inbox',exact:true}).click()
  await title(N,141,p,'Activity inbox');await yes(N,142,p.locator('#activity-inbox-title'),null,10000)
  await delayed(144,p);equal(N,145,(await p.evaluate(()=>window.__progressBarSeen))>=1,true)
 }
}
async function openRoomMenu(p,r){
 for(let attempt=0;attempt<3;attempt++){
  try{await row(p,r).click({button:'right'});await yes(M,322,menu(p).locator(':scope:not([hidden])'),null,5000);return}
  catch(e){if(attempt===2)throw e}
 }
}
async function confirmLeave(p,r){
 await p.locator('.room-menu__confirm').getByRole('button',{name:'Leave',exact:true}).click()
 await auditWait(r,'room.membership.change');await no(M,338,row(p,r),null,10000)
}
async function menuScenario(p,ctx){
 const designers=id('designers'),hq=id('hq')
 const del=p=>menu(p).locator("button[data-room-menu-target='deleteAction']")
 const confirm=p=>p.locator('.room-menu__confirm')
 const deleteRoom=async r=>{await openRoomMenu(p,r);await del(p).click();await confirm(p).getByRole('button',{name:'Delete',exact:true}).click()}
 const leaveRoom=async r=>{await openRoomMenu(p,r);await menu(p).locator("button[data-room-menu-target='leaveAction']").click()}
 if(selected==='menu-admin-delete'){
  await openRoomMenu(p,designers);await del(p).click()
  await text(M,13,confirm(p),"Delete #Designers and all its messages? This can't be undone.")
  await confirm(p).getByRole('button',{name:'Delete',exact:true}).click()
  await no(M,17,row(p,designers),null,10000);await yes(M,18,p.locator('.flash'),'Deleted #Designers',10000)
  immediateState(M,19,'deleted',designers,{value:true});immediateState(M,20,'audit',designers,{action:'room.destroy'})
 }else if(selected==='menu-creator'){
  await openRoomMenu(p,id('own'));await yes(M,29,menu(p).locator('button'),'Delete…')
  await p.keyboard.press('Escape');await no(M,31,p.locator('#room-menu:not([hidden])'),null,5000)
  await openRoomMenu(p,designers);await no(M,34,menu(p).locator('button'),'Delete…')
 }else if(selected==='menu-member'){
  await openRoomMenu(p,designers);await no(M,42,menu(p).locator('button'),'Delete…')
 }else if(selected==='menu-cancel'){
  await openRoomMenu(p,designers);await del(p).click();await confirm(p).getByRole('button',{name:'Cancel',exact:true}).click()
  await no(M,54,p.locator('.room-menu__confirm[open]'),null,5000);await yes(M,55,row(p,designers))
  immediateState(M,56,'deleted',designers,{value:false})
  await openRoomMenu(p,designers);await del(p).click();await yes(M,60,p.locator('.room-menu__confirm[open]'),null,5000)
  await p.keyboard.press('Escape');await no(M,63,p.locator('.room-menu__confirm[open]'),null,5000);immediateState(M,64,'deleted',designers,{value:false})
 }else if(selected==='menu-current-delete'){
  await deleteRoom(designers);await assertionWait(M,77,p.waitForURL(u=>u.pathname!==roomPath(designers),{timeout:10000}));trace(M,77)
  await yes(M,79,p.locator('.flash'),'Deleted #Designers',10000);immediateState(M,80,'deleted',designers,{value:true})
 }else if(selected==='menu-kinds-delete'){
  for(const r of [hq,designers,id('board'),id('voice'),id('stage')]){
   await openRoomMenu(p,r);await yes(M,94,menu(p).locator('button'),'Delete…')
   await p.keyboard.press('Escape');await no(M,96,p.locator('#room-menu:not([hidden])'),null,5000)
  }
  await deleteRoom(id('stage'));await no(M,104,row(p,id('stage')),null,10000);immediateState(M,105,'deleted',id('stage'),{value:true})
 }else if(selected==='menu-group-permission'){
  await openRoomMenu(p,id('permission_group'));await yes(M,117,menu(p).locator('button'),'Delete…')
  await signIn(p,'jz');await join(p,hq)
  await openRoomMenu(p,id('permission_group'));await no(M,122,menu(p).locator('button'),'Delete…')
 }else if(selected==='menu-dm-permission'){
  await openRoomMenu(p,id('bender_and_kevin'));await yes(M,132,menu(p).locator('button'),'Delete…')
  await p.keyboard.press('Escape');await no(M,134,p.locator('#room-menu:not([hidden])'),null,5000)
  await openRoomMenu(p,id('david_and_kevin'));await no(M,137,menu(p).locator('button'),'Delete…')
 }else if(selected==='menu-keyboard'){
  await row(p,designers).press('Shift+F10');await yes(M,145,p.locator('#room-menu:not([hidden])'),null,5000)
  await focus(M,151,p,"#room-menu [data-room-menu-target='favoriteAction']")
  await p.keyboard.press('End');await focus(M,153,p,"#room-menu [data-room-menu-target='deleteAction']")
  await p.keyboard.press('Enter');await yes(M,156,p.locator('.room-menu__confirm[open]'),null,5000)
 }else if(selected==='menu-phone-delete'){
  await resizeOriginal(p,390,844);await p.getByRole('button',{name:'Open workspace navigation',exact:true}).click()
  await yes(M,165,p.locator('#sidebar.open'),null,5000)
  await row(p,designers).evaluate(el=>el.dispatchEvent(new MouseEvent('contextmenu',{bubbles:true,cancelable:true})))
  await yes(M,173,p.locator('#room-menu:not([hidden])'),null,5000);await del(p).click()
  await confirm(p).getByRole('button',{name:'Delete',exact:true}).click()
  await no(M,178,row(p,designers),null,10000);immediateState(M,179,'deleted',designers,{value:true})
 }else if(selected==='menu-open-leave'){
  await openRoomMenu(p,hq);await yes(M,190,menu(p).locator('button'),'Leave');await menu(p).locator("button[data-room-menu-target='leaveAction']").click()
  await text(M,194,confirm(p),"Leave #HQ? You'll stop seeing it in your sidebar.")
  await confirmLeave(p,hq);immediateState(M,198,'membership',hq,{user:'jz',value:false});immediateState(M,199,'deleted',hq,{value:false})
  await visit(p,base+roomPath(hq));await yes(M,202,p.locator('h2'),'#HQ',10000);await p.getByRole('button',{name:'Join channel',exact:true}).click()
  await yes(M,205,row(p,hq),null,10000);immediateState(M,206,'membership',hq,{user:'jz',value:true})
 }else if(selected==='menu-private-leave'){
  await leaveRoom(designers);await text(M,218,confirm(p),"Leave #Designers? You'll stop seeing it in your sidebar. Someone will need to add you back.")
  await confirmLeave(p,designers);immediateState(M,222,'deleted',designers,{value:false})
  immediateState(M,225,'audit',designers,{action:'room.membership.change',actor:'jz'});immediateState(M,226,'audit',designers,{action:'room.membership.change',actor:'jz',revoked:['JZ']})
 }else if(selected==='menu-current-leave'){
  await leaveRoom(hq);await confirm(p).getByRole('button',{name:'Leave',exact:true}).click()
  await assertionWait(M,239,p.waitForURL(u=>u.pathname!==roomPath(hq),{timeout:10000}));trace(M,239)
  immediateState(M,241,'membership',hq,{user:'jz',value:false})
 }else if(selected==='menu-solo-leave'){
  await leaveRoom(id('solo'));await confirmLeave(p,id('solo'))
  immediateState(M,253,'deleted',id('solo'),{value:false});immediateState(M,254,'membership',id('solo'),{empty:true})
 }else if(selected==='menu-kinds-leave'){
  for(const r of [hq,designers,id('board'),id('voice'),id('stage')]){
   await leaveRoom(r);const current=new URL(p.url()).pathname===roomPath(r);await confirmLeave(p,r)
   if(current){await assertionWait(M,281,p.waitForURL(u=>u.pathname!==roomPath(r),{timeout:10000}));trace(M,281);await streams(p,1,15000)}
   immediateState(M,287,'deleted',r,{value:false});immediateState(M,288,'membership',r,{user:'jz',value:false})
  }
 }else if(selected==='menu-group-leave'){
  await leaveRoom(id('leave_group'));await confirmLeave(p,id('leave_group'))
  immediateState(M,304,'membership',id('leave_group'),{user:'jz',value:false});immediateState(M,305,'deleted',id('leave_group'),{value:false})
 }
}
async function navActions(line,p,labels,shown){
 for(const label of labels){const loc=p.locator(`.room-header__actions [aria-label='${label}']`);if(shown)await yes(H,424,loc);else await no(H,418,loc)}trace(H,line)
}
async function searchVisible(line,p){await yes(H,413,p.locator('#global-search-input, #global-search .global-search__toggle'));trace(H,line)}
async function trailing(line,p,shown){
 if(shown){await yes(H,431,p.locator(".room-header__actions a[href$='/edit']"));await yes(H,432,p.locator('.room-header__actions .button_to_change_notifying'))}
 else{await no(H,436,p.locator(".room-header__actions a[href$='/edit']"));await no(H,437,p.locator('.room-header__actions .button_to_change_notifying'))}trace(H,line)
}
async function innerScroll(line,p){equal(H,444,await p.evaluate(()=>{const row=document.querySelector('.room-header__actions');return row.scrollWidth-row.clientWidth<=1}),true);trace(H,line)}
async function horizontal(line,p){equal(H,453,await p.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth),true);trace(H,line)}
async function inside(line,p){
 const [box,width]=await p.evaluate(()=>[document.querySelector('#nav').getBoundingClientRect().toJSON(),window.innerWidth])
 equal(H,461,box.left>=0,true);equal(H,462,box.right<=width,true);trace(H,line)
}
async function headerMenuItem(p,label){
 await (await originalFind(overflow(p).locator("[role='menuitem']"),label)).click()
}
async function more(p){await p.getByRole('button',{name:'More actions',exact:true}).click()}
function pinThird(){
 const now=labels['clock.now'],room=id('designers')
 db.prepare('INSERT INTO message_pins(room_id,message_id,pinner_id,created_at,updated_at) VALUES (?,?,?,?,?)').run(room,labels['messages.third'],user('jz'),now,now)
 db.prepare('UPDATE rooms SET pins_changed_at=?,updated_at=? WHERE id=?').run(now,now,room)
}
function dotThread(){
 const now=labels['clock.now'],room=id('designers'),thread=9300000001,message=9300000002
 db.prepare('INSERT INTO channel_threads(id,room_id,creator_id,name,last_activity_at,messages_count,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?)').run(thread,room,user('kevin'),'Dot check',now,1,now,now)
 db.prepare('INSERT INTO thread_memberships(thread_id,user_id,joined_at,unread_at,created_at,updated_at) VALUES (?,?,?,?,?,?)').run(thread,user('jz'),now,now,now,now)
 db.prepare('INSERT INTO messages(id,room_id,thread_id,creator_id,markdown_source,client_message_id,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?)').run(message,room,thread,user('kevin'),'News for the dot.','dot-check-1',now,now)
 db.prepare('INSERT INTO action_text_rich_texts(name,body,record_type,record_id,created_at,updated_at) VALUES (?,?,?,?,?,?)').run('body','<p>News for the dot.</p>','Message',message,now,now)
}
async function header(p){
 if(['header-phone','header-tablet','header-desktop'].includes(selected)){
  const phone=selected==='header-phone',tablet=selected==='header-tablet'
  for(const width of phone?[360,390,500]:tablet?[700,768,1024]:[1280,1400]){
   await resizeOriginal(p,width,phone||tablet?800:900)
   if(phone){
    await yes(H,27,p.locator('#header-overflow-button'));await yes(H,28,p.locator('.workspace-navigation__open'));await yes(H,29,p.locator('.room-header__name'))
    await navActions(30,p,['Show members','Join huddle'],true);await searchVisible(31,p)
    await navActions(32,p,['Show threads','Show events','Show pinned messages','Show files'],false)
    await no(H,33,p.locator('#nav .help-menu'));await no(H,34,p.locator(".room-header__actions [aria-label^='Quick switcher']"));await trailing(35,p,false)
    await more(p);await yes(H,39,overflow(p))
    for(const [line,label] of [[41,'Threads'],[42,'Events'],[43,'Pins'],[44,'Files'],[45,'Notifications'],[46,'Room settings'],[47,'Keyboard shortcuts'],[48,'Restart tour'],[49,'Quick switcher']])await yes(H,line,overflow(p).locator("[role='menuitem']"),label)
    await p.keyboard.press('Escape');await no(H,52,overflow(p))
    await innerScroll(54,p);await horizontal(55,p);await inside(56,p)
   }else if(tablet){
    await yes(H,79,p.locator('#header-overflow-button'));await navActions(80,p,['Show members','Join huddle'],true);await searchVisible(81,p)
    await navActions(82,p,['Show threads','Show events','Show pinned messages','Show files'],false)
    await no(H,83,p.locator('#nav .help-menu'));await no(H,84,p.locator(".room-header__actions [aria-label^='Quick switcher']"));await trailing(85,p,true)
    await more(p);await yes(H,88,overflow(p))
    for(const [line,label] of [[90,'Threads'],[91,'Events'],[92,'Pins'],[93,'Files'],[94,'Keyboard shortcuts'],[95,'Restart tour'],[96,'Quick switcher']])await yes(H,line,overflow(p).locator("[role='menuitem']"),label)
    await no(H,98,overflow(p).locator("[role='menuitem']"),'Notifications');await no(H,99,overflow(p).locator("[role='menuitem']"),'Room settings')
    await p.keyboard.press('Escape');await innerScroll(103,p);await horizontal(104,p);await inside(105,p)
   }else{
    await no(H,120,p.locator('#header-overflow-button'));await no(H,121,overflow(p));await yes(H,122,p.locator('#help-menu-button'))
    await yes(H,124,p.locator('body.member-panel-open'));await navActions(125,p,['Hide members','Join huddle'],true);await searchVisible(126,p)
    await yes(H,127,p.locator(".room-header__actions [data-thread-panel-target='browserToggle']"));await navActions(128,p,['Show events','Show pinned messages','Show files'],true)
    await yes(H,129,p.locator(".room-header__actions [aria-label^='Quick switcher']"));await trailing(130,p,true);await horizontal(132,p);await inside(133,p)
   }
  }
  if(phone){await resizeOriginal(p,320,740);await yes(H,61,p.locator('#header-overflow-button'));await navActions(62,p,['Show members','Join huddle'],true);await searchVisible(63,p);await horizontal(64,p);await inside(65,p)}
 }else if(selected==='header-items'){
  await resizeOriginal(p,390,844);await more(p);await headerMenuItem(p,'Threads')
  await yes(H,152,p.locator('body.thread-panel-open'),null,5000);await p.locator('.thread-panel__close').click();await no(H,154,p.locator('body.thread-panel-open'),null,5000)
  pinThird();await visit(p,base+roomPath(id('designers')));await resizeOriginal(p,390,844);await more(p)
  await yes(H,162,overflow(p).locator("[data-header-overflow-target='pinsCount']"),'1');await headerMenuItem(p,'Pins')
  await text(H,166,p.locator('.pins-panel'),"Third time's a charm.");await p.locator('.pins-panel__close').click();await no(H,169,p.locator('.pins-panel[open]'))
  await more(p);await overflow(p).getByRole('menuitem',{name:'Files',exact:true}).click();await yes(H,176,p.locator('#room-files-title'),'Files',10000);await path(H,177,p,/\/rooms\/\d+\/files/)
  await join(p,id('designers'));await resizeOriginal(p,390,844);await more(p);await overflow(p).getByRole('menuitem',{name:'Events',exact:true}).click()
  await text(H,186,p.locator('body'),'Upcoming',10000);await path(H,187,p,/\/rooms\/\d+\/events/)
  await join(p,id('designers'));await resizeOriginal(p,390,844);await more(p);await overflow(p).getByRole('menuitem',{name:'Room settings',exact:true}).click();await path(H,196,p,/\/edit/)
  await join(p,id('designers'));await resizeOriginal(p,390,844);await more(p);await headerMenuItem(p,'Keyboard shortcuts')
  await yes(H,205,p.locator('#keyboard-shortcuts[open]'),null,5000);await p.getByRole('button',{name:'Close shortcuts',exact:true}).click();await no(H,207,p.locator('#keyboard-shortcuts[open]'))
  await more(p);await headerMenuItem(p,'Quick switcher');await yes(H,214,p.locator('#quick-switcher[open]'),null,5000);await p.keyboard.press('Escape');await no(H,216,p.locator('#quick-switcher[open]'))
  await more(p);await overflow(p).getByRole('menuitem',{name:'Notifications: Everything',exact:true}).click()
  await yes(H,224,overflow(p).locator("#header-overflow-notifications [role='menuitemradio'][aria-checked='true']"),'Everything')
  await overflow(p).getByRole('menuitemradio',{name:'Muted',exact:true}).click();await yes(H,226,overflow(p).locator("[data-header-overflow-target='notificationsLabel']"),'Notifications: Muted')
  immediateState(H,228,'involvement',id('designers'),{user:'jz',value:'muted'})
  await yes(H,229,p.locator(`#user_sidebar a[data-room-id='${id('designers')}']`),null,2000,true)
  await yes(H,231,p.locator('.room-header__actions .button_to_change_notifying button.muted'),null,10000,true)
 }else if(selected==='header-landscape'){
  await resizeOriginal(p,640,360);await more(p);await yes(H,245,overflow(p));await yes(H,250,overflow(p).locator("[role='menuitem']"),'Stage')
  const [bottom,height]=await p.evaluate(()=>[document.getElementById('header-overflow-menu').getBoundingClientRect().bottom,window.innerHeight]);equal(H,256,bottom<=height,true)
  // Settle the render task queued by the real menu click before dispatching
  // the global End action; the harness never assigns focus itself.
  await p.evaluate(()=>new Promise(requestAnimationFrame));await p.keyboard.press('End');await focus(H,260,p,"#header-overflow-menu [aria-label='Quick switcher']")
  equal(H,261,await p.evaluate(()=>{const menu=document.getElementById('header-overflow-menu').getBoundingClientRect(),item=document.querySelector("#header-overflow-menu [aria-label='Quick switcher']").getBoundingClientRect();return item.top>=menu.top&&item.bottom<=menu.bottom}),true)
  await headerMenuItem(p,'Quick switcher');await yes(H,273,p.locator('#quick-switcher[open]'),null,5000)
 }else if(selected==='header-return-focus'){
  await resizeOriginal(p,390,844);await more(p);await headerMenuItem(p,'Threads')
  await yes(H,289,p.locator('body.thread-panel-open'),null,5000);await p.locator('.thread-panel__close').click();await no(H,292,p.locator('body.thread-panel-open'),null,5000);await focus(H,293,p,'#header-overflow-button')
 }else if(selected==='header-keyboard'){
  await resizeOriginal(p,390,844);await p.locator('#header-overflow-button').press('ArrowDown')
  await yes(H,307,overflow(p));equal(H,308,await p.locator('#header-overflow-button').getAttribute('aria-expanded'),'true');await focus(H,309,p,"#header-overflow-menu [role='menuitem']")
  await p.keyboard.press('ArrowDown');await focus(H,312,p,"#header-overflow-menu a[href*='/events']")
  await p.keyboard.press('Escape');await no(H,315,overflow(p));await focus(H,316,p,'#header-overflow-button');equal(H,317,await p.locator('#header-overflow-button').getAttribute('aria-expanded'),'false')
  await more(p);await yes(H,320,overflow(p));await p.locator('.room-header__name').click();await no(H,322,overflow(p))
 }else if(selected==='header-dot'){
  await resizeOriginal(p,390,844);await yes(H,334,p.locator('#header-overflow-button'));await yes(H,335,p.locator('#header-overflow-button .header-overflow__dot[hidden]'),null,2000,true)
  await more(p);await yes(H,340,overflow(p).locator("[data-header-overflow-target='pinsCount'][hidden]"),null,2000,true);await p.keyboard.press('Escape')
  pinThird();await visit(p,base+roomPath(id('designers')));await resizeOriginal(p,390,844);await more(p)
  await yes(H,350,overflow(p).locator("[data-header-overflow-target='pinsCount']:not([hidden])"),'1');await p.keyboard.press('Escape')
  await yes(H,353,p.locator('#header-overflow-button .header-overflow__dot[hidden]'),null,2000,true)
  dotThread();await more(p);await headerMenuItem(p,'Threads')
  await yes(H,365,p.locator("#thread-panel [data-thread-panel-target='browserList'] .thread-panel__thread-item[data-unread='true']"),'Dot check',10000)
  await p.locator('.thread-panel__close').click();await no(H,368,p.locator('body.thread-panel-open'),null,5000)
  await yes(H,370,p.locator('#header-overflow-button .header-overflow__dot:not([hidden])'),null,10000)
 }else if(selected==='header-tour'){
  await resizeOriginal(p,390,844);await p.getByRole('button',{name:'Skip tour',exact:true}).click();await no(H,384,p.locator('#tour .tour__card'))
  equal(H,386,(await synchronized(H,386,2000,async()=>await matching(p.locator('#help-menu-button'),null,true)-await matching(p.locator('#help-menu-button'),null,false),n=>n>0))>0,true)
  await yes(H,387,p.locator('#header-overflow-button'));await more(p);await headerMenuItem(p,'Restart tour')
  await yes(H,393,p.locator('#tour .tour__card'));await yes(H,394,p.locator('.tour__progress'),'Step 1 of 5')
  for(let i=0;i<3;i++)await p.getByRole('button',{name:'Next',exact:true}).click()
  await yes(H,399,p.locator('.tour__progress'),'Step 4 of 5');await yes(H,400,p.locator('#header-overflow-button.tour__target'))
  await p.getByRole('button',{name:'Next',exact:true}).click();await yes(H,402,p.locator('.tour__progress'),'Step 5 of 5');await yes(H,403,p.locator('#header-overflow-button.tour__target'))
 }
}
async function singleBadge(line,p,r){
 const loc=p.locator(`#${roomDom(r)} .sidebar-item__status`)
 let count;try{count=await synchronized(U,137,5000,()=>matching(loc,null,false),n=>n===1)}catch(error){console.log('ORIGINAL_MEMBERSHIP_DIAGNOSTIC '+JSON.stringify(db.prepare('SELECT room_id,user_id,involvement,unread_at,created_at,updated_at,connected_at,connections FROM memberships WHERE room_id=?').all(r)));console.log('ORIGINAL_BADGE_DIAGNOSTIC '+JSON.stringify(await p.locator('#sidebar a[data-room-id]').evaluateAll(rows=>rows.map(row=>({id:row.id,room:row.dataset.roomId,classes:row.className,badges:row.querySelectorAll('.sidebar-item__status').length,text:row.innerText})))));throw error}
 equal(U,137,count,1);trace(U,line)
}
async function noBadge(line,p,r){await no(U,143,p.locator(`#${roomDom(r)} .sidebar-item__status`),null,5000);trace(U,line)}
async function badgeGeometry(line,p,r){
 const g=await p.evaluate(dom=>{const row=document.getElementById(dom);return {badgeDirect:!!row.querySelector(':scope > .sidebar-item__status'),columns:getComputedStyle(row).gridTemplateColumns.split(' ').length}},roomDom(r))
 equal(U,161,g.badgeDirect,true);equal(U,162,g.columns,4);trace(U,line)
}
async function unread(p){
 const designers=id('designers'),hq=id('hq'),direct=id('unread_direct')
 const c=await signedContext('kevin'),kp=await c.newPage();kp.setDefaultTimeout(10000)
 try{
  if(selected==='unread-between'){
   await roomRead(U,13,p,hq);await signIn(kp);await join(kp,designers);await send(kp,'Hello!!');await send(kp,'Talking to myself?')
   await roomRead(U,22,p,designers,true);await join(p,designers);await roomRead(U,25,p,designers)
  }else if(selected==='unread-live'){
   await p.evaluate(()=>{window.unreadEventTargets=[];window.addEventListener('rooms-list:unread',e=>window.unreadEventTargets.push(String(e.detail.targetId)))})
   await signIn(kp);await join(kp,designers);await send(kp,"Here while you're here")
   await yes(U,41,p.locator('.message[data-message-id] .message__body'),"Here while you're here",10000);trace(S,123)
   await p.waitForFunction(dom=>window.unreadEventTargets.includes(dom),roomDom(designers),{timeout:10000});trace(U,42);trace(U,130)
   await roomRead(U,43,p,designers);await noBadge(44,p,designers)
  }else{
   db.prepare('UPDATE memberships SET connected_at=NULL,connections=0 WHERE user_id=? AND room_id=?').run(user('jz'),designers)
   await signIn(kp);await join(kp,designers);await send(kp,'Channel one');await join(kp,direct);await send(kp,'DM one')
   await singleBadge(68,p,designers);await singleBadge(69,p,direct)
   await visit(p,base+roomPath(hq));await streams(p);await singleBadge(76,p,designers);await singleBadge(77,p,direct)
   // Reapply the producer defect to the new document, preserving the baseline
   // reload/server-rendered badge premise before the second real cable wave.
   if(mutation)await mutate(p)
   await send(kp,'DM two');await join(kp,designers);await send(kp,'Channel two')
   await singleBadge(85,p,designers);await singleBadge(86,p,direct);await badgeGeometry(87,p,designers);await badgeGeometry(88,p,direct)
   await join(p,designers);await noBadge(91,p,designers);await join(p,direct);await noBadge(94,p,direct)
  }
 }finally{await c.close()}
}
try{
 const designerCases=['key-read','key-typing-escape','key-menu-escape','key-theater','key-fullscreen','menu-current-delete','menu-open-leave','menu-kinds-leave','unread-live']
 const adminCases=['menu-admin-delete','menu-cancel','menu-current-delete','menu-kinds-delete','menu-group-permission','menu-keyboard','menu-phone-delete']
 const room=selected==='header-landscape'?id('stage'):selected.startsWith('header-')||designerCases.includes(selected)?id('designers'):id('hq')
 await scenario(async(p,ctx)=>{
  if(selected.startsWith('key-'))await keyboard(p)
  else if(selected.startsWith('menu-'))await menuScenario(p,ctx)
  else if(selected.startsWith('nav-'))await navigation(p)
  else if(selected.startsWith('header-'))await header(p)
  else if(selected.startsWith('unread-'))await unread(p)
  else throw Error('unknown case '+selected)
 },{room,as:adminCases.includes(selected)?'david':selected==='menu-dm-permission'?'kevin':'jz'})
 console.log('ORIGINAL_STATE_REQUESTS '+JSON.stringify(requests))
}finally{db.close();await browser.close();await new Promise(r=>proxy.close(r))}
