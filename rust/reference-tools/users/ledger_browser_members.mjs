// Exact predicates from the pinned Rails member, channel and motion originals.
import assert from 'node:assert/strict'
import fs from 'node:fs'
import crypto from 'node:crypto'
import {DatabaseSync} from 'node:sqlite'
import {chromium} from 'playwright'
import {network} from './original_browser_network.mjs'
import {visit,waitForController} from './browser_navigation.mjs'
import {diagnostics} from './browser_diagnostics.mjs'
import {displayedCount,evaluateWithDisplayed} from './selenium_displayed.mjs'
import {viewportFor,resizeOriginal} from './ledger_browser_viewports.mjs'

const M='test/system/member_select_mode_test.rb'
const C='test/system/channel_members_test.rb'
const T='test/system/motion_test.rb'
const H='test/test_helpers/system_test_helper.rb'
const base=process.env.WS11UI_BROWSER_URL
const labels=JSON.parse(fs.readFileSync(process.env.WS11UI_BROWSER_LABELS,'utf8'))
const selected=process.env.WS11UI_BROWSER_CASE
const defaultMutations={
  'member-inline':'member-inline-zero','member-plain':'member-count',
  'member-modifiers':'member-hidden-checkbox','member-exit-focus':'member-focus',
  'member-rerender':'member-rerender','channel-online':'channel-presence',
  'channel-phone':'channel-composer','motion-members':'motion-member-shift',
  'motion-sticky':'motion-sticky-out-of-frame','motion-menu':'motion-menu-clamp',
  'motion-scroll':'motion-scroll-retention',
}
const mutation=process.env.WS11UI_BROWSER_MUTATION||(process.env.WS11UI_BROWSER_CONTROL==='1'?defaultMutations[selected]:'')
const mutationFailures={
  'member-count':`${M}:46`,'member-bar-scope':`${M}:46`,
  'member-hidden-checkbox':`${M}:60`,
  'member-inline-zero':`${M}:442`,'member-inline-hidden':`${M}:442`,
  'member-inline-out-of-frame':`${M}:442`,'member-focus':`${M}:159`,
  'member-rerender':`${M}:275`,'channel-presence':`${C}:33`,
  'channel-composer':`${C}:158`,'motion-member-shift':`${T}:86`,
  'motion-sticky-out-of-frame':`${T}:145`,'motion-menu-clamp':`${T}:170`,
  'motion-scroll-retention':`${T}:224`,'motion-reopen-focus':`${T}:238`,
}
const db=new DatabaseSync(process.env.WS11UI_BROWSER_DATABASE)
db.exec('PRAGMA busy_timeout=5000')
const proxy=await network(base)
const browser=await chromium.launch({headless:true,args:['--no-sandbox']})
const requests=[]
const user=u=>labels['users.'+u]
const room=r=>labels['rooms.'+r]
const panel='#channel-members'
const bar=panel+" [data-multi-select-target='bar']"
const row=u=>panel+` [data-member-id='${user(u)}']`
const name=u=>row(u)+' button.profile-card-name'
const box=u=>panel+` #select-member-${user(u)}`
const hiddenCard='#profile-card-popover[hidden]'
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms))
function equal(file,line,actual,expected) {
  assert.deepEqual(actual,expected,`${file}:${line}: original assertion`)
  console.log(`ORIGINAL_ASSERTION ${file}:${line}`)
}
async function until(file,line,fn,timeout=2000) {
  const end=Date.now()+timeout
  while(Date.now()<end) {
    try {if(await fn()){equal(file,line,true,true);return}}
    catch(error) {if(!String(error.message).includes('Execution context was destroyed'))throw error}
    await sleep(50)
  }
  equal(file,line,await fn(),true)
}
// Preserve the pinned Selenium displayed-node predicate and visible: :all.
// Geometry predicates below deliberately query every node.
async function matches(p,selector,{all=false,text=null}={}) {
  return displayedCount(p.locator(selector),text,all)
}
async function displayedIds(p,selector) {
  const nodes=p.locator(selector),ids=[]
  for(let index=0;index<await nodes.count();index++) {
    if(await displayedCount(nodes.nth(index)))ids.push(await nodes.nth(index).getAttribute('id'))
  }
  return ids
}
async function firstDisplayed(p,selector) {
  const nodes=p.locator(selector)
  for(let index=0;index<await nodes.count();index++)if(await displayedCount(nodes.nth(index)))return nodes.nth(index)
  throw Error('INVALID_CONTROL original displayed finder matched no element: '+selector)
}
async function openMembersIfDisplayed(p) {
  const end=Date.now()+5000
  while(Date.now()<end) {
    if(await matches(p,"button[aria-label='Show members']")) {
      await p.getByRole('button',{name:'Show members',exact:true}).click();return
    }
    await sleep(50)
  }
}
async function select(file,line,p,selector,opts={}) {
  const {count=null,min=1,absent=false,wait=2000}=opts
  await until(file,line,async()=>{const n=await matches(p,selector,opts);return absent?n===0:count!==null?n===count:n>=min},wait)
}
async function focused(file,line,p,selector) {
  await until(file,line,()=>p.evaluate(selector=>document.activeElement instanceof Element&&document.activeElement.matches(selector),selector))
  equal(H,41,await p.evaluate(selector=>document.activeElement instanceof Element&&document.activeElement.matches(selector),selector),true)
}
async function rowFocus(p,u,line) {
  await until(M,line,()=>p.evaluate(selector=>document.activeElement===document.querySelector(selector),u?name(u):panel+' .member-panel__member button.profile-card-name'))
  equal(M,u?405:414,await p.evaluate(selector=>document.activeElement===document.querySelector(selector),u?name(u):panel+' .member-panel__member button.profile-card-name'),true)
}
async function message(p,file,line,n) {await select(file,line,p,bar+' button',{text:`Message (${n})`})}
async function checked(p,file,line,u,{all=false}={}) {
  await until(file,line,async()=>await matches(p,box(u),{all})>0&&await p.locator(box(u)).evaluate(input=>!input.matches(':disabled')&&input.checked))
}
async function ctrl(p,u,shift=false) {await p.locator(name(u)).click({modifiers:shift?['Control','Shift']:['Control']})}
async function refresh(p) {
  await p.evaluate(()=>{window.__oldMemberRow=document.querySelector('#channel-members .member-panel__member');window.Stimulus.getControllerForElementAndIdentifier(document.body,'member-panel').refreshPresence()})
  await until(M,426,()=>p.evaluate(()=>!window.__oldMemberRow.isConnected))
}
function deleteMember(u) {db.prepare('DELETE FROM memberships WHERE user_id=? AND room_id=?').run(user(u),room('designers'))}
function addMember(u) {
  const now=labels['clock.now']
  db.prepare('INSERT INTO memberships(room_id,user_id,created_at,updated_at) VALUES (?,?,?,?)').run(room('designers'),user(u),now,now)
}
function createCaseFixtures(caseName) {
  const now=labels['clock.now']
  if(caseName==='member-range')db.prepare('INSERT INTO user_stars(user_id,starred_user_id,created_at,updated_at) VALUES (?,?,?,?)').run(user('david'),user('jason'),now,now)
  if(caseName==='channel-agent') {
    db.prepare('INSERT INTO users(id,name,role,bot_token_digest,created_at,updated_at) VALUES (?,?,?,?,?,?)').run(user('panel_bot'),'Panel Bot',2,crypto.createHash('sha256').update('ledger-panel-bot').digest('hex'),now,now)
    db.prepare('INSERT INTO memberships(room_id,user_id,created_at,updated_at) VALUES (?,?,?,?)').run(room('designers'),user('panel_bot'),now,now)
    db.prepare('INSERT INTO agents(user_id,owner_id,kind,last_seen_at,created_at,updated_at) VALUES (?,?,?,?,?,?)').run(user('panel_bot'),user('david'),'workspace',now,now,now)
  }
  if(caseName==='channel-phone') {
    db.prepare('INSERT INTO rooms(id,name,type,creator_id,created_at,updated_at) VALUES (?,?,?,?,?,?)').run(room('planning'),'Planning','Rooms::Closed',user('jz'),now,now)
    for(const u of ['jz','bender'])db.prepare('INSERT INTO memberships(room_id,user_id,created_at,updated_at) VALUES (?,?,?,?)').run(room('planning'),user(u),now,now)
  }
  if(caseName==='motion-sticky')for(let index=0;index<12;index++)db.prepare('INSERT INTO users(id,name,email_address,created_at,updated_at) VALUES (?,?,?,?,?)').run(9100010200+index,`Sticky User ${index}`,`sticky${index}@example.test`,now,now)
  if(['motion-scroll','motion-first-reveal','motion-reopen-current'].includes(caseName)) {
    for(let index=0;index<15;index++) {
      db.prepare('INSERT INTO rooms(id,name,type,creator_id,created_at,updated_at) VALUES (?,?,?,?,?,?)').run(9100010100+index,`Scroll room ${index}`,'Rooms::Closed',user('jz'),now,now)
      db.prepare('INSERT INTO memberships(room_id,user_id,created_at,updated_at) VALUES (?,?,?,?)').run(9100010100+index,user('jz'),now,now)
    }
    if(caseName==='motion-first-reveal') {
      db.prepare('INSERT INTO rooms(id,name,type,creator_id,created_at,updated_at) VALUES (?,?,?,?,?,?)').run(room('far'),'Zz far room','Rooms::Closed',user('jz'),now,now)
      db.prepare('INSERT INTO memberships(room_id,user_id,created_at,updated_at) VALUES (?,?,?,?)').run(room('far'),user('jz'),now,now)
    }
  }
}
async function settleMembers(p) {
  // Show members starts a real fetch; wait for its rows before using a native
  // element handle. The zero-duration test transition alone can settle while
  // desktop rows are still being replaced by the mobile-open response.
  await p.waitForFunction(()=>window.Stimulus.getControllerForElementAndIdentifier(document.body,'member-panel').loading===false)
  await until(M,460,()=>p.evaluate(()=>{const surface=document.querySelector('.member-panel__surface');return !surface||surface.getAnimations().every(animation=>{const timing=animation.effect?.getComputedTiming?.();return !(timing&&Number.isFinite(timing.endTime)&&(animation.playState==='running'||animation.playState==='pending'))})}))
}
async function inline(p,file,line) {
  await select(file,file===M?434:165,p,panel+' .member-panel__member .member-panel__identity')
  const broken=await p.evaluate(()=>Array.from(document.querySelectorAll('#channel-members .member-panel__member')).filter(row=>{
    const avatar=row.querySelector('.member-panel__avatar').getBoundingClientRect()
    const identity=row.querySelector('.member-panel__identity').getBoundingClientRect()
    return identity.left<avatar.right||identity.top>=avatar.bottom||identity.width<80
  }).map(row=>row.dataset.memberId))
  equal(file,file===M?442:173,broken,[])
  equal(file,line,true,true)
}
async function composer(p,line) {
  await until(C,145,async()=>await matches(p,"#composer textarea[aria-label='Write a message']:not(:disabled)")>0&&await p.getByLabel('Write a message',{exact:true}).inputValue()==='')
  const offsets=await p.evaluate(()=>{const field=document.querySelector('#composer textarea'),bounds=field.getBoundingClientRect(),style=getComputedStyle(field);const textCenter=bounds.top+parseFloat(style.borderTopWidth)+parseFloat(style.paddingTop)+parseFloat(style.lineHeight)/2;return ['.composer__attachment-btn','.composer__send'].map(selector=>{const button=document.querySelector(selector).getBoundingClientRect();return Math.abs(button.top+button.height/2-textCenter)})})
  equal(C,158,offsets.every(offset=>offset<=1),true)
  equal(C,line,true,true)
}
async function overflow(p,line) {equal(C,177,await p.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),true);equal(C,line,true,true)}
async function settleVisual(p) {
  // The original screenshot helper re-queries animations on each pass and
  // returns safely after ten seconds; no Animation object crosses the wire.
  const end=Date.now()+10000
  while(Date.now()<end) {
    try {if(await p.evaluate(()=>document.getAnimations().every(animation=>{const timing=animation.effect?.getComputedTiming?.();return !(timing&&Number.isFinite(timing.endTime)&&(animation.playState==='running'||animation.playState==='pending'))})))return}
    catch(error) {if(!String(error.message).includes('Execution context was destroyed'))throw error}
    await sleep(50)
  }
}
async function member(p,u,online,line) {
  await select(C,line,p,row(u)+`[data-online='${online}']`,{all:true,text:u==='panel_bot'?'Panel Bot':({jz:'JZ',kevin:'Kevin',bender:'Bender Bot'}[u]),wait:20000})
  await select(C,129,p,row(u)+`[data-online='${online}']`,{all:true,text:u==='panel_bot'?'Panel Bot':({jz:'JZ',kevin:'Kevin',bender:'Bender Bot'}[u]),wait:20000})
}
async function leaseCount(u,count) {
  await until(C,141,async()=>db.prepare('SELECT count(*) AS count FROM workspace_presence_leases WHERE user_id=? AND julianday(expires_at)>=julianday(?)').get(user(u),labels['clock.now']).count===count,15000)
}
async function cable(p) {
  await until(H,76,()=>p.evaluate(()=>{const sources=document.querySelectorAll('turbo-cable-stream-source');return sources.length>=3&&Array.from(sources).every(el=>el.hasAttribute('connected'))}),15000)
}
async function newSession(u,viewport={width:1440,height:1000}) {
  const context=await browser.newContext({viewport:viewportFor(labels,viewport.width,viewport.height)})
  await context.route('**/*',r=>new URL(r.request().url()).origin===new URL(base).origin?r.continue():r.abort())
  context.ledgerErrors=[]
  context.on('page',p=>{
    p.on('pageerror',error=>context.ledgerErrors.push(String(error)))
    p.on('requestfailed',request=>{if(new URL(request.url()).pathname.startsWith('/assets/')&&request.failure()?.errorText!=='net::ERR_ABORTED')context.ledgerErrors.push(request.failure().errorText)})
    p.on('response',response=>{if(response.status()>=500)context.ledgerErrors.push(response.status()+' '+response.url())})
  })
  return context
}
async function signIn(p,u) {
  await visit(p,base+'/test_session?'+new URLSearchParams({email_address:u+'@37signals.com',password:'secret123456'}))
  await select(H,65,p,'a.btn',{text:'Designers',wait:10000})
}
async function join(p,r) {
  assert.equal((await visit(p,base+'/rooms/'+room(r))).status(),200,'INVALID_CONTROL initial room request');await cable(p)
  // A room visit loads the sidebar in a separate Turbo frame. Its pending
  // replacement can remove the native focus target during a drawer gesture.
  await p.waitForFunction(()=>{const frame=document.getElementById('user_sidebar');return frame?.hasAttribute('complete')&&!frame.hasAttribute('busy')})
}
async function mutate(p,kind) {
  if(mutation!==kind)return
  const observation=await p.evaluate(kind=>{
    const body=document.body,controller=name=>window.Stimulus.getControllerForElementAndIdentifier(document.querySelector(`[data-controller~='${name}']`),name)
    const multi=()=>window.Stimulus.getControllerForElementAndIdentifier(document.querySelector('#channel-members [data-controller~=multi-select]'),'multi-select')
    if(kind==='member-count'||kind==='member-bar-scope') {
      const c=multi();if(!c||typeof c.update!=='function')throw Error('INVALID_CONTROL missing multi-select update')
      const original=c.update.bind(c)
      c.update=(...args)=>{original(...args);if(kind==='member-count')c.messageButtonTarget.textContent='Message (999)';else {let decoy=document.querySelector('#decoy-member-bar');if(!decoy){decoy=document.createElement('div');decoy.id='decoy-member-bar';body.append(decoy)}decoy.replaceChildren(c.barTarget.cloneNode(true));c.messageButtonTarget.textContent='Message (999)'}}
      return {producer:'multi-select.update',valid:true}
    }
    if(kind==='member-focus') {
      const c=multi();if(!c||typeof c.clear!=='function')throw Error('INVALID_CONTROL missing clear')
      const original=c.clear.bind(c);c.clear=(...args)=>{const result=original(...args);document.querySelector('#composer textarea').focus();return result}
      return {producer:'multi-select.clear',valid:true}
    }
    if(kind==='member-rerender') {
      const c=multi();if(!c||typeof c.checkboxTargetConnected!=='function')throw Error('INVALID_CONTROL missing checkboxTargetConnected')
      const original=c.checkboxTargetConnected.bind(c);c.checkboxTargetConnected=(...args)=>{c.selectedIds.clear();original(...args)};return {producer:'multi-select.checkboxTargetConnected',valid:true}
    }
    if(kind==='member-hidden-checkbox') {
      const input=document.querySelector('#channel-members input[type=checkbox]');if(!input)throw Error('INVALID_CONTROL missing checkbox')
      input.style.display='none';return {display:getComputedStyle(input).display}
    }
    if(kind.startsWith('member-inline-')) {
      const rows=document.querySelectorAll('#channel-members .member-panel__member');if(rows.length<2)throw Error('INVALID_CONTROL requires later row')
      const identity=rows[0].querySelector('.member-panel__identity')
      if(kind==='member-inline-zero')identity.style.cssText='width:0!important;min-width:0!important;max-width:0!important'
      if(kind==='member-inline-hidden')rows[0].style.display='none'
      if(kind==='member-inline-out-of-frame')identity.style.transform='translateY(2000px)'
      return {matched:rows.length,width:identity.getBoundingClientRect().width,laterWidth:rows[1].querySelector('.member-panel__identity').getBoundingClientRect().width}
    }
    if(kind==='channel-presence') {
      const url=document.querySelector('#channel-members').dataset.membersUrl
      if(!url)throw Error('INVALID_CONTROL missing member response URL')
      const original=window.fetch.bind(window)
      window.fetch=async (...args)=>{const response=await original(...args);if(String(args[0])===url){const json=response.json.bind(response);response.json=async()=>{const data=await json();data.members.forEach(member=>{if(member.name==='Kevin')member.online=false});return data}}return response}
      return {producer:'served members online predicate',valid:true}
    }
    if(kind==='channel-composer') {document.querySelector('.composer__send').style.transform='translateY(8px)';return {producer:'.composer__send transform',valid:true}}
    if(kind==='motion-member-shift') {document.querySelector('#channel-members .member-panel__avatar').style.transform='translateX(8px)';return {producer:'member avatar layout',valid:true}}
    if(kind==='motion-sticky-out-of-frame') {document.querySelector('.multi-select-bar').style.cssText='position:relative!important;transform:translateY(2000px)';return {producer:'directory bar layout',valid:true}}
    if(kind==='motion-menu-clamp') {document.querySelector('#room-menu').style.left=innerWidth+'px';return {producer:'room menu position',valid:true}}
    if(kind==='motion-scroll-retention') {const scroller=document.querySelector('#sidebar .sidebar__scroll');scroller.style.scrollBehavior='auto';scroller.scrollTop=0;return {producer:'closed scroller offset',scrollTop:scroller.scrollTop}}
    if(kind==='motion-reopen-focus') {document.querySelector('#sidebar a[aria-current=page]').focus({preventScroll:true});return {producer:'reopen drawer focus',valid:true}}
    throw Error('INVALID_CONTROL unknown mutation '+kind)
  },kind)
  if(kind.startsWith('member-inline-')) {
    assert.ok(observation.matched>=2,'INVALID_CONTROL requires a later original matching row')
    assert.ok(observation.laterWidth>=80,'INVALID_CONTROL later original row must retain valid geometry')
    if(kind==='member-inline-zero'||kind==='member-inline-hidden')assert.equal(observation.width,0,'INVALID_CONTROL first identity must measure zero')
  }
  if(kind==='motion-scroll-retention')assert.equal(observation.scrollTop,0,'INVALID_CONTROL closed scroller did not reset')
  console.log('ORIGINAL_MUTATION '+kind+' '+JSON.stringify(observation))
}
async function scenario(caseName,fn,{u='david',r='designers',size={width:1440,height:1000},setup='member'}={}) {
  if(selected!==caseName)return
  const context=await newSession(u,size),p=await context.newPage(),bad=context.ledgerErrors,cleanup=[]
  p.setDefaultTimeout(10000)
  const diagnose=diagnostics(p)
  try {
    await signIn(p,u)
    await join(p,r)
    await waitForController(p,'member-panel');await waitForController(p,'profile-card')
    if(setup==='member') {
      await openMembersIfDisplayed(p)
      await select(M,11,p,panel+' .member-panel__member',{min:3,wait:10000})
      await waitForController(p,'multi-select')
    } else if(setup==='channel')await select(C,13,p,panel)
    await fn(p,context,cleanup)
    equal('transport',0,[...bad,...cleanup.flatMap(c=>c.ledgerErrors)],[])
    console.log(`ORIGINAL_CASE ${caseName}: passed`)
  } catch(error) {

    await diagnose(error)
    if(bad.length||cleanup.some(c=>c.ledgerErrors.length))throw Error('INVALID_CONTROL transport: '+JSON.stringify([...bad,...cleanup.flatMap(c=>c.ledgerErrors)]),{cause:error})
    if(mutation&&!String(error.message).includes(mutationFailures[mutation]+': original assertion'))throw Error('INVALID_CONTROL setup or wrong predicate for '+mutation,{cause:error})
    throw error
  } finally {
    for(const c of cleanup)await c.close()
    await context.close()
    console.log('ORIGINAL_PRODUCER_RESTORED '+caseName+': browser context disposed')
  }
}

try {
await scenario('member-inline',async p=>{
  await select(M,19,p,panel+" input[type='checkbox']",{absent:true})
  await select(M,20,p,panel+" input[type='checkbox']",{all:true,count:3})
  await select(M,21,p,row('david')+" input[type='checkbox']",{all:true,absent:true})
  await select(M,22,p,bar,{absent:true})
  await select(M,23,p,bar+':not([aria-live])',{all:true})
  await select(M,24,p,panel+" [data-multi-select-target='status']",{text:'0 selected'})
  for(const defect of ['member-inline-zero','member-inline-hidden','member-inline-out-of-frame'])await mutate(p,defect)
  await inline(p,M,25)
  await resizeOriginal(p,390,844)
  await select(M,28,p,panel,{absent:true})
  await p.getByRole('button',{name:'Show members',exact:true}).click()
  await select(M,30,p,panel+' .member-panel__member',{min:3,wait:10000});await settleMembers(p)
  await select(M,32,p,panel+" input[type='checkbox']",{absent:true})
  await inline(p,M,33)
})
await scenario('member-plain',async p=>{
  await p.locator(name('jason')).click()
  await select(M,40,p,'#profile-card-popover:not([hidden])',{wait:10000});await select(M,41,p,'#user_card .profile-card__name',{text:'Jason'})
  await p.locator('.profile-card-popover__close').click();await select(M,43,p,hiddenCard,{all:true,wait:10000})
  await mutate(p,'member-count');await mutate(p,'member-bar-scope')
  await ctrl(p,'jason');await message(p,M,46,1);await select(M,47,p,hiddenCard,{all:true})
  await p.locator(name('kevin')).click();await message(p,M,50,2);await select(M,51,p,hiddenCard,{all:true})
  await p.locator(name('jason')).click();await message(p,M,54,1)
})
await scenario('member-modifiers',async p=>{
  await ctrl(p,'jason');await message(p,M,59,1)
  await mutate(p,'member-hidden-checkbox')
  await select(M,60,p,panel+" input[type='checkbox']",{count:3});await inline(p,M,61)
  await select(M,62,p,panel+" input[aria-label='Select Jason']");await select(M,63,p,hiddenCard,{all:true})
  await ctrl(p,'jason');await select(M,66,p,bar,{absent:true});await select(M,67,p,panel+" input[type='checkbox']",{absent:true})
  await p.locator(name('jason')).evaluate(el=>el.dispatchEvent(new MouseEvent('click',{bubbles:true,cancelable:true,metaKey:true})))
  await message(p,M,70,1);await select(M,71,p,hiddenCard,{all:true})
  await p.locator(name('kevin')).evaluate(el=>el.dispatchEvent(new MouseEvent('click',{bubbles:true,cancelable:true,metaKey:true})));await message(p,M,74,2)
})
await scenario('member-range',async p=>{
  createCaseFixtures('member-range')
  await refresh(p)
  await select(M,80,p,panel+` [aria-label='Starred members'] [data-member-id='${user('jason')}']`,{all:true,wait:10000})
  await select(M,81,p,row('david')+"[data-online='true']",{all:true,wait:20000})
  equal(M,82,await p.evaluate(()=>Array.from(document.querySelectorAll('#channel-members .member-panel__member')).map(row=>Number(row.dataset.memberId))),['jason','david','jz','kevin'].map(user))
  await ctrl(p,'jason');await message(p,M,85,1);await ctrl(p,'jz',true);await message(p,M,88,2)
  await checked(p,M,89,'jason');await checked(p,M,90,'jz')
  await ctrl(p,'kevin');await message(p,M,95,3);await ctrl(p,'kevin');await message(p,M,97,2)
  await ctrl(p,'jason',true);await message(p,M,99,3)
  await checked(p,M,100,'jason');await checked(p,M,101,'jz');await checked(p,M,102,'kevin')
})
await scenario('member-range-box',async p=>{
  await ctrl(p,'jason');await message(p,M,107,1);await p.locator(name('kevin')).click();await message(p,M,112,2)
  await p.locator(box('kevin')).click({modifiers:['Control','Shift']});await message(p,M,116,3)
  await checked(p,M,117,'jason');await checked(p,M,118,'jz');await checked(p,M,119,'kevin')
})
await scenario('member-departed-profile',async p=>{
  await ctrl(p,'jason');await message(p,M,124,1);deleteMember('jason');await refresh(p)
  await select(M,128,p,row('jason'),{all:true,absent:true});await select(M,129,p,bar,{absent:true})
  await p.locator(name('kevin')).click();await select(M,132,p,'#profile-card-popover:not([hidden])',{wait:10000});await select(M,133,p,'#user_card .profile-card__name',{text:'Kevin'})
})
await scenario('member-exits',async p=>{
  await ctrl(p,'jason');await message(p,M,138,1);await p.keyboard.press('Escape')
  await select(M,141,p,bar,{absent:true});await select(M,142,p,panel+" input[type='checkbox']",{absent:true});await select(M,143,p,panel)
  await ctrl(p,'jason');await message(p,M,146,1);await p.locator(bar).getByRole('button',{name:'Exit selection mode',exact:true}).click()
  await select(M,148,p,bar,{absent:true});await select(M,149,p,panel+" input[type='checkbox']",{absent:true});await select(M,150,p,panel)
})
await scenario('member-exit-focus',async p=>{
  await ctrl(p,'jason');await message(p,M,155,1);await mutate(p,'member-focus')
  await p.locator(bar).getByRole('button',{name:'Exit selection mode',exact:true}).click();await select(M,158,p,bar,{absent:true});await rowFocus(p,'jason',159)
})
await scenario('member-fallback-focus',async p=>{
  await ctrl(p,'jason');await ctrl(p,'kevin');await message(p,M,165,2);deleteMember('kevin');await refresh(p);await message(p,M,169,1)
  await p.locator(bar).getByRole('button',{name:'Exit selection mode',exact:true}).click();await select(M,172,p,bar,{absent:true});await rowFocus(p,null,173)
})
await scenario('member-uncheck-focus',async p=>{
  await ctrl(p,'jason');await message(p,M,178,1);await p.locator(box('jason')).click();await select(M,181,p,bar,{absent:true});await rowFocus(p,'jason',182)
})
await scenario('member-mobile-departed',async p=>{
  await resizeOriginal(p,390,844);await select(M,187,p,panel,{absent:true});await p.getByRole('button',{name:'Show members',exact:true}).click()
  await select(M,189,p,panel+' .member-panel__member',{min:3,wait:10000});await settleMembers(p)
  await ctrl(p,'jason');await message(p,M,193,1);deleteMember('jason');await refresh(p)
  await select(M,197,p,row('jason'),{all:true,absent:true});await select(M,198,p,bar,{absent:true})
  await p.keyboard.press('Escape');await select(M,201,p,panel,{absent:true})
})
await scenario('member-desktop-departed',async p=>{
  await ctrl(p,'jason');await message(p,M,208,1);deleteMember('jason');await refresh(p);await select(M,212,p,bar,{absent:true})
  await p.keyboard.press('Escape');await select(M,215,p,panel);addMember('jason');await refresh(p);await message(p,M,219,1);await checked(p,M,220,'jason')
})
await scenario('member-menu-escape',async p=>{
  await ctrl(p,'jason');await message(p,M,225,1);await p.locator(row('kevin')).click({button:'right'});await select(M,228,p,'#member-row-menu')
  await p.locator(name('kevin')).focus();await p.keyboard.press('Escape');await select(M,235,p,'#member-row-menu',{absent:true});await message(p,M,236,1)
})
await scenario('member-status',async p=>{
  const status=panel+" [data-multi-select-target='status']"
  await select(M,240,p,status,{text:'0 selected'});await ctrl(p,'jason');await message(p,M,243,1);await select(M,244,p,status,{text:'1 selected'})
  await ctrl(p,'kevin');await message(p,M,247,2);await select(M,248,p,status,{text:'2 selected'})
  await p.locator(bar).getByRole('button',{name:'Exit selection mode',exact:true}).click();await select(M,251,p,bar,{absent:true});await select(M,252,p,status,{text:'0 selected'})
})
await scenario('member-space',async p=>{
  await p.locator(name('jason')).focus();await p.locator(name('jason')).press('Space');await message(p,M,260,1);await select(M,261,p,hiddenCard,{all:true});await checked(p,M,262,'jason')
  await p.locator(name('jason')).press('Space');await select(M,265,p,bar,{absent:true});await select(M,266,p,hiddenCard,{all:true})
})
await scenario('member-rerender',async p=>{
  await ctrl(p,'jason');await message(p,M,271,1);await mutate(p,'member-rerender');await refresh(p)
  await message(p,M,275,1);await checked(p,M,276,'jason');await ctrl(p,'kevin',true);await message(p,M,281,3)
})
await scenario('member-star-menu',async p=>{
  await ctrl(p,'jason');await message(p,M,286,1);await p.locator(row('kevin')).click({button:'right'});await select(M,289,p,'#member-row-menu')
  await p.locator('#member-row-menu button').filter({hasText:'☆ Star'}).click();await select(M,292,p,'#member-row-menu button',{text:'★ Unstar',wait:10000})
  await select(M,294,p,panel+` [aria-label='Starred members'] [data-member-id='${user('kevin')}']`,{all:true,wait:10000});await message(p,M,295,1)
  await p.locator('#member-row-menu [role=menuitem]').press('Escape');await select(M,298,p,'#member-row-menu',{absent:true})
  await p.keyboard.press('Escape');await select(M,300,p,bar,{absent:true})
  await p.locator(row('kevin')).click({button:'right'});await select(M,303,p,'#member-row-menu')
  await p.locator('#member-row-menu button').filter({hasText:'★ Unstar'}).click();await select(M,306,p,'#member-row-menu button',{text:'☆ Star',wait:10000})
  await select(M,308,p,panel+" [aria-label='Starred members']",{all:true,absent:true})
})
await scenario('member-long-press',async p=>{
  await resizeOriginal(p,390,844);await select(M,313,p,panel,{absent:true});await p.getByRole('button',{name:'Show members',exact:true}).click()
  await select(M,315,p,panel+' .member-panel__member',{min:3,wait:10000});await settleMembers(p)
  // Keep the original two sleeps inside one browser evaluation: driver IPC
  // between touch release and compatibility click must not consume the app's
  // 500 ms suppression window. The immediate menu observation still occurs
  // after contextmenu and before the second sleep, as in Rails :333.
  await firstDisplayed(p,row('jason'))
  const visibleMenuCount=await evaluateWithDisplayed(p,async ({selector},isDisplayed)=>{
    const row=document.querySelector(selector)
    const touch=new Touch({identifier:1,target:row,clientX:10,clientY:10})
    row.dispatchEvent(new TouchEvent('touchstart',{touches:[touch],bubbles:true,cancelable:true}))
    await new Promise(resolve=>setTimeout(resolve,200))
    row.dispatchEvent(new MouseEvent('contextmenu',{bubbles:true,cancelable:true}))
    const count=Array.from(document.querySelectorAll('#member-row-menu')).filter(menu=>isDisplayed(menu)).length
    await new Promise(resolve=>setTimeout(resolve,500))
    row.dispatchEvent(new TouchEvent('touchend',{bubbles:true,cancelable:true}))
    row.querySelector('button.profile-card-name').dispatchEvent(new MouseEvent('click',{bubbles:true,cancelable:true,clientX:10,clientY:10}))
    return count
  },{selector:row('jason')})
  equal(M,333,visibleMenuCount,0)
  await checked(p,M,344,'jason');await message(p,M,345,1);await select(M,346,p,hiddenCard,{all:true});await select(M,347,p,'#member-row-menu',{absent:true});await inline(p,M,348)
  await p.locator(name('kevin')).click();await message(p,M,351,2);await select(M,352,p,hiddenCard,{all:true})
  await p.locator(bar).getByRole('button',{name:'Message (2)',exact:true}).click();await select(M,356,p,'.room--current',{text:'Jason, Kevin',wait:10000})
  const current=Number(new URL(p.url()).pathname.split('/').at(-1)),ids=['david','jason','kevin'].map(user).sort((a,b)=>a-b)
  const actual=db.prepare("SELECT room_id FROM memberships JOIN rooms ON rooms.id=memberships.room_id WHERE rooms.type='Rooms::Direct' AND rooms.deleted_at IS NULL GROUP BY room_id HAVING count(*)=? AND sum(user_id IN (?,?,?))=?").get(ids.length,...ids,ids.length)
  equal(M,358,new URL(p.url()).pathname,'/rooms/'+actual.room_id)
  requests.push({kind:'direct',room:current,users:['david','jason','kevin']})
})
await scenario('channel-online',async(p,context,cleanup)=>{
  await member(p,'jz',true,23);await member(p,'kevin',false,24)
  await select(C,25,p,row('bender'),{all:true,absent:true});await mutate(p,'channel-presence')
  const kevin=await newSession('kevin'),kp=await kevin.newPage();cleanup.push(kevin)
  await signIn(kp,'kevin');await join(kp,'hq')
  db.prepare('UPDATE memberships SET unread_at=? WHERE user_id=? AND room_id=?').run(labels['clock.now'],user('kevin'),room('designers'))
  await member(p,'kevin',true,33)
  equal(C,34,db.prepare('SELECT unread_at FROM memberships WHERE user_id=? AND room_id=?').get(user('kevin'),room('designers')).unread_at!==null,true)
  requests.push({kind:'unread'})
  await composer(p,35);await p.emulateMedia({colorScheme:'dark'});await settleVisual(p)
  assert.equal((await visit(kp,base+'/users/me/profile')).status(),200,'INVALID_CONTROL profile request')
  await kp.getByRole('button',{name:'Log out',exact:true}).click()
  await select(C,43,kp,'input[name=email_address]:not(:disabled)')
  await member(p,'kevin',false,46);await member(p,'jz',true,47)
},{u:'jz',setup:'channel'})
await scenario('channel-agent',async p=>{
  createCaseFixtures('channel-agent')
  await join(p,'designers');await member(p,'panel_bot',true,58)
},{u:'jz',setup:'channel'})
await scenario('channel-tabs',async(p,context,cleanup)=>{
  const kevin=await newSession('kevin'),kp=await kevin.newPage();cleanup.push(kevin)
  await signIn(kp,'kevin');await join(kp,'hq')
  const extra=await kevin.newPage();await join(extra,'hq');await leaseCount('kevin',2)
  await extra.close();await leaseCount('kevin',1);await member(p,'kevin',true,75)
  await kp.goto('about:blank');await leaseCount('kevin',0);await member(p,'kevin',false,79)
  equal(C,80,db.prepare('SELECT count(*) AS count FROM sessions WHERE user_id=?').get(user('kevin')).count>0,true)
  requests.push({kind:'session'})
},{u:'jz',setup:'channel'})
await scenario('channel-phone',async p=>{
  createCaseFixtures('channel-phone')
  await join(p,'planning');await member(p,'bender',false,86)
  await select(C,87,p,row('kevin'),{all:true,absent:true});await select(C,88,p,row('david'),{all:true,absent:true})
  await p.getByRole('button',{name:'Hide members',exact:true}).click();await select(C,91,p,panel,{absent:true})
  await p.getByRole('button',{name:'Show members',exact:true}).click();await select(C,93,p,panel);await inline(p,C,94)
  await resizeOriginal(p,390,844);await select(C,97,p,panel,{absent:true});await overflow(p,98)
  await mutate(p,'channel-composer');await composer(p,99)
  await p.getByRole('button',{name:'Show members',exact:true}).click();await select(C,101,p,"button[aria-label='Close members']:not(:disabled)")
  await member(p,'bender',false,102);await focused(C,103,p,"button[aria-label='Close members']")
  await p.keyboard.press('Shift+Tab');await focused(C,105,p,panel+' *')
  await p.keyboard.press('Escape');await select(C,107,p,panel,{absent:true});await focused(C,108,p,"button[aria-label='Show members']")
  await p.getByRole('button',{name:'Show members',exact:true}).click();await settleVisual(p);await inline(p,C,112)
  await p.getByRole('button',{name:'Close members',exact:true}).click()
  await p.locator('#composer textarea').fill('Still easy to chat on a phone.');await p.getByRole('button',{name:'Send Message',exact:true}).click()
  await select(C,116,p,'.message[data-message-id] .message__body',{text:'Still easy to chat on a phone.'})
  await select(H,123,p,'.message[data-message-id] .message__body',{text:'Still easy to chat on a phone.'});await composer(p,117)
  await resizeOriginal(p,320,740);await overflow(p,120);await select(C,121,p,"button[aria-label='Show members']:not(:disabled)");await composer(p,122)
},{u:'jz',setup:'channel'})

const avatarLefts=p=>p.evaluate(()=>Array.from(document.querySelectorAll('#channel-members .member-panel__member .member-panel__avatar')).map(avatar=>avatar.getBoundingClientRect().left))
const contentHeight=p=>p.evaluate(()=>document.querySelector('#channel-members .member-panel__content').clientHeight)
const directoryTops=p=>p.evaluate(()=>Array.from(document.querySelectorAll('.people-directory__row')).map(row=>row.getBoundingClientRect().top))
const drawerTx=p=>p.evaluate(()=>{const transform=getComputedStyle(document.querySelector('#sidebar .sidebar__container')).transform;if(transform==='none')return 0;const match=transform.match(/matrix\((.+)\)/);return match?Number(match[1].split(',').map(part=>part.trim())[4]):null})
async function motionWait(p,line,fn) {await until(T,line,fn,5000);equal(T,396,true,true)}
const inDrawer=(p,selector)=>p.evaluate(selector=>{const element=document.querySelector(selector);if(!element)return false;const rect=element.getBoundingClientRect();if(rect.width===0||rect.height===0)return false;const scroller=document.querySelector('#sidebar .sidebar__scroll');let top=0,bottom=innerHeight;if(scroller.contains(element)){top=scroller.getBoundingClientRect().top+scroller.clientTop;bottom=top+scroller.clientHeight}return rect.top>=top-1&&rect.bottom<=bottom+1},selector)
const scroller='#sidebar .sidebar__scroll'
const scrollTop=p=>p.evaluate(selector=>document.querySelector(selector).scrollTop,scroller)
const currentRoom="#sidebar a[aria-current='page']"
await scenario('motion-default',async p=>{
  equal(T,20,await p.evaluate(()=>document.documentElement.dataset.testMotion),'off')
  for(const [line,token] of [[21,'--motion-medium'],[22,'--motion-fast'],[23,'--motion-quick']])equal(T,line,await p.evaluate(token=>getComputedStyle(document.documentElement).getPropertyValue(token).trim(),token),'0ms')
},{u:'jz',r:'hq',size:{width:1400,height:1400},setup:'motion'})
await scenario('motion-drawer',async p=>{
  await resizeOriginal(p,390,844);await p.evaluate(()=>{document.documentElement.removeAttribute('data-test-motion');document.documentElement.style.setProperty('--motion-medium','30s')})
  const listeners=await p.evaluate(()=>{window.__motionRuns=[];const sidebar=document.querySelector('#sidebar'),surface=document.querySelector('#sidebar .sidebar__container');let attached=0;if(sidebar){sidebar.addEventListener('transitionrun',event=>window.__motionRuns.push('drawer:'+event.propertyName));attached++}if(surface){surface.addEventListener('transitionrun',event=>window.__motionRuns.push('surface:'+event.propertyName));attached++}return attached})
  equal(T,348,listeners,2);await p.evaluate(()=>{window.__motionRuns=[]})
  await p.getByRole('button',{name:'Open workspace navigation',exact:true}).click();await select(T,37,p,'#sidebar.open')
  const start=await drawerTx(p);equal(T,43,start<-1,true)
  await motionWait(p,44,async()=>{const tx=await drawerTx(p);return tx!==null&&tx>start+.5})
  equal(T,49,(await drawerTx(p))<-1,true)
  const transitions=await p.evaluate(()=>document.querySelector('#sidebar .sidebar__container').getAnimations().filter(a=>a.playState==='running').map(a=>a.transitionProperty))
  equal(T,362,transitions.includes('transform'),true);equal(T,51,transitions.includes('transform'),true)
  await motionWait(p,52,()=>p.evaluate(()=>{const runs=window.__motionRuns||[];return runs.includes('drawer:opacity')&&runs.includes('surface:transform')}))
  await p.evaluate(()=>{document.querySelector('#sidebar').getAnimations().forEach(a=>a.finish());document.querySelector('#sidebar .sidebar__container').getAnimations().forEach(a=>a.finish());document.documentElement.style.removeProperty('--motion-medium')})
  await motionWait(p,365,()=>p.evaluate(()=>{const transform=getComputedStyle(document.querySelector('#sidebar .sidebar__container')).transform;return transform==='none'||transform==='matrix(1, 0, 0, 1, 0, 0)'}))
  equal(T,61,await p.evaluate(()=>getComputedStyle(document.querySelector('#sidebar')).opacity),'1')
  await motionWait(p,62,()=>p.evaluate(()=>document.querySelector('#sidebar').contains(document.activeElement)))
  await p.keyboard.press('Escape');await select(T,67,p,'#sidebar.open',{absent:true})
  await motionWait(p,68,()=>p.evaluate(()=>document.activeElement?.getAttribute('aria-label')==='Open workspace navigation'))
},{u:'jz',r:'hq',size:{width:1400,height:1400},setup:'motion'})
await scenario('motion-members',async p=>{
  await openMembersIfDisplayed(p)
  await select(T,75,p,panel+' .member-panel__member',{min:2,wait:10000});await waitForController(p,'multi-select')
  await select(T,79,p,panel+" input[type='checkbox']",{absent:true});const lefts=await avatarLefts(p),height=await contentHeight(p)
  await ctrl(p,'jason');await select(T,84,p,panel+" input[type='checkbox']");await select(T,85,p,bar)
  await mutate(p,'motion-member-shift')
  equal(T,86,await avatarLefts(p),lefts);equal(T,87,await contentHeight(p),height)
  const ids=await displayedIds(p,panel+" input[type='checkbox']")
  const last=ids.filter(id=>id!=='select-member-'+user('jason')).at(-1);await p.locator('#'+last).click()
  await select(T,93,p,panel+" [data-multi-select-target='messageButton']",{text:'Message (2)'})
  equal(T,94,await avatarLefts(p),lefts);equal(T,95,await contentHeight(p),height)
  for(const id of await displayedIds(p,panel+" input[type='checkbox']"))if(await p.locator('#'+id).isChecked())await p.locator('#'+id).click()
  await select(T,98,p,bar,{absent:true});equal(T,99,await avatarLefts(p),lefts);equal(T,100,await contentHeight(p),height)
},{u:'jz',r:'hq',size:{width:1400,height:1400},setup:'motion'})
await scenario('motion-directory',async p=>{
  await visit(p,base+'/users');await select(T,105,p,'.people-directory__row',{min:2});await waitForController(p,'multi-select')
  const tops=await directoryTops(p);await p.locator('#'+(await displayedIds(p,".people-directory__row input[type='checkbox']"))[0]).click();await select(T,111,p,'.multi-select-bar')
  equal(T,112,await directoryTops(p),tops)
  for(const id of await displayedIds(p,".people-directory__row input[type='checkbox']"))if(await p.locator('#'+id).isChecked())await p.locator('#'+id).click()
  await select(T,115,p,'.multi-select-bar',{absent:true});equal(T,116,await directoryTops(p),tops)
},{u:'jz',r:'hq',size:{width:1400,height:1400},setup:'motion'})
await scenario('motion-sticky',async p=>{
  createCaseFixtures('motion-sticky')
  await resizeOriginal(p,1400,400);await visit(p,base+'/users');await select(T,123,p,'.people-directory__row',{min:10});await waitForController(p,'multi-select')
  equal(T,124,await p.evaluate(()=>{const main=document.querySelector('#main-content');return main.scrollHeight>main.clientHeight+100}),true)
  await p.locator('#'+(await displayedIds(p,".people-directory__row input[type='checkbox']"))[0]).click();await select(T,128,p,'.multi-select-bar')
  await p.evaluate(()=>{document.querySelector('#main-content').scrollTop=100});await motionWait(p,135,()=>p.evaluate(()=>document.querySelector('#main-content').scrollTop===100))
  await mutate(p,'motion-sticky-out-of-frame')
  const geometry=await p.evaluate(()=>{const bar=document.querySelector('.multi-select-bar').getBoundingClientRect(),main=document.querySelector('#main-content').getBoundingClientRect();return {barBottom:bar.bottom,mainBottom:main.bottom}})
  equal(T,145,geometry.barBottom<=geometry.mainBottom+1,true)
},{u:'jz',r:'hq',size:{width:1400,height:1400},setup:'motion'})
await scenario('motion-menu',async p=>{
  await p.evaluate(()=>document.documentElement.removeAttribute('data-test-motion'))
  await (await firstDisplayed(p,'#sidebar a[data-room-id]')).evaluate(row=>row.dispatchEvent(new MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:innerWidth-10,clientY:400})))
  await select(T,159,p,'#room-menu:not([hidden])');await motionWait(p,160,()=>p.evaluate(()=>getComputedStyle(document.querySelector('#room-menu')).transform==='none'))
  await mutate(p,'motion-menu-clamp')
  const geometry=await p.evaluate(()=>{const rect=document.querySelector('#room-menu').getBoundingClientRect();return {right:rect.right,limit:innerWidth-8}})
  equal(T,170,geometry.right<=geometry.limit+1,true)
  await p.keyboard.press('Escape');await select(T,174,p,'#room-menu[hidden]',{all:true})
},{u:'jz',r:'hq',size:{width:1400,height:1400},setup:'motion'})
await scenario('motion-scroll',async p=>{
  createCaseFixtures('motion-scroll')
  await join(p,'hq');await resizeOriginal(p,390,844);await p.getByRole('button',{name:'Open workspace navigation',exact:true}).click();await select(T,184,p,'#sidebar.open')
  await motionWait(p,189,()=>p.evaluate(()=>document.querySelector('#sidebar').contains(document.activeElement)));await p.evaluate(()=>document.activeElement.blur())
  equal(T,195,await p.evaluate(selector=>{const el=document.querySelector(selector);return el.scrollHeight>el.clientHeight},scroller),true)
  const max=await p.evaluate(selector=>{const el=document.querySelector(selector);return el.scrollHeight-el.clientHeight},scroller);equal(T,202,max>=400,true)
  await p.evaluate(selector=>{document.querySelector(selector).scrollTop=400},scroller);await motionWait(p,207,async()=>await scrollTop(p)===400)
  const before=await scrollTop(p);equal(T,211,before,400);equal(T,212,await inDrawer(p,currentRoom),false)
  await p.keyboard.press('Escape');await select(T,216,p,'#sidebar.open',{absent:true});await mutate(p,'motion-scroll-retention')
  equal(T,224,await scrollTop(p),before)
  await p.getByRole('button',{name:'Open workspace navigation',exact:true}).click();await select(T,227,p,'#sidebar.open')
  await motionWait(p,233,()=>p.evaluate(()=>document.querySelector('#sidebar').contains(document.activeElement)))
  await mutate(p,'motion-reopen-focus')
  equal(T,237,await scrollTop(p),before);equal(T,238,await inDrawer(p,':focus'),true)
},{u:'jz',r:'hq',size:{width:1400,height:1400},setup:'motion'})
await scenario('motion-first-reveal',async p=>{
  createCaseFixtures('motion-first-reveal')
  await join(p,'far');await resizeOriginal(p,390,844);await select(T,250,p,currentRoom,{all:true})
  equal(T,251,await scrollTop(p),0);equal(T,252,await inDrawer(p,currentRoom),false)
  await p.getByRole('button',{name:'Open workspace navigation',exact:true}).click();await select(T,255,p,'#sidebar.open');await focused(T,256,p,currentRoom)
  await motionWait(p,257,()=>inDrawer(p,currentRoom))
},{u:'jz',r:'hq',size:{width:1400,height:1400},setup:'motion'})
await scenario('motion-reopen-current',async p=>{
  createCaseFixtures('motion-reopen-current')
  await join(p,'hq');await resizeOriginal(p,390,844);await p.getByRole('button',{name:'Open workspace navigation',exact:true}).click()
  await select(T,270,p,'#sidebar.open');await focused(T,271,p,currentRoom)
  let last=null;await motionWait(p,274,async()=>{const now=await scrollTop(p),settled=now===last&&await inDrawer(p,currentRoom);last=now;return settled})
  await p.keyboard.press('Escape');await select(T,282,p,'#sidebar.open',{absent:true});const before=await scrollTop(p)
  await p.getByRole('button',{name:'Open workspace navigation',exact:true}).click();await select(T,286,p,'#sidebar.open');await focused(T,287,p,currentRoom)
  equal(T,288,await scrollTop(p),before)
},{u:'jz',r:'hq',size:{width:1400,height:1400},setup:'motion'})
console.log('ORIGINAL_STATE_REQUESTS '+JSON.stringify(requests))
} finally {await browser.close();await proxy?.close();db.close()}
