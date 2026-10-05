// Individual assertions from the runtime Rails pin, through real forms and shipped assets.
import assert from 'node:assert/strict'
import fs from 'node:fs'
import { DatabaseSync } from 'node:sqlite'
import { chromium } from 'playwright'
import { diagnostics } from './browser_diagnostics.mjs'
import { network } from './original_browser_network.mjs'; import { visit, waitForController } from './browser_navigation.mjs'
const base=process.env.WS11UI_BROWSER_URL
const proxy=await network(base)
const labels=JSON.parse(fs.readFileSync(process.env.WS11UI_BROWSER_LABELS,'utf8'))
const selected=process.env.WS11UI_BROWSER_CASE
const mode=process.env.WS11UI_BROWSER_MODE
const broken=process.env.WS11UI_BROWSER_CONTROL==='1'
const mutation=process.env.WS11UI_BROWSER_MUTATION
const browser=await chromium.launch({headless:true,args:['--no-sandbox']})
const requests=[]
// Rails uses Capybara's two-second default, with explicit ten-second waits at
// these original assertion sites. Keep that distinction in this translation.
function assertionWait(file,line) {
 const explicit={
  'test/system/people_group_dms_test.rb':[16,34,38,68,85,97,107,238,284,340,341,372,377,381,396,400],
  'test/system/starred_people_test.rb':[26,36,53,59,100,105,115,120,134,140,144,154]
 }
 return explicit[file]?.includes(line)?10000:2000
}
function equal(file,line,actual,expected) {
  assert.deepEqual(actual,expected,`${file}:${line}: original assertion`)
  console.log(`ORIGINAL_ASSERTION ${file}:${line}`)
}
async function visible(file,line,locator,text=null) {
  if(text!==null)locator=locator.filter({hasText:text})
  // Selenium treats the zero-size popover wrapper with a visible absolute child
  // as displayed. Playwright's wrapper visibility uses its own rectangle.
  if(locator.toString().includes('profile-card-popover')) {
    await locator.first().waitFor({state:'attached',timeout:assertionWait(file,line)})
    await locator.locator('.profile-card-popover__panel').waitFor({timeout:assertionWait(file,line)})
    equal(file,line,await locator.locator('.profile-card-popover__panel').isVisible(),true)
  } else {
    await locator.first().waitFor({timeout:assertionWait(file,line)})
    equal(file,line,await locator.first().isVisible(),true)
  }
}
async function present(file,line,locator) {
  await locator.first().waitFor({state:'attached',timeout:assertionWait(file,line)})
  equal(file,line,await locator.count()>0,true)
}
async function visibleMinimum(file,line,p,selector,minimum) {
 await p.waitForFunction(({selector,minimum})=>Array.from(document.querySelectorAll(selector)).filter(el=>{
  const rect=el.getBoundingClientRect();return rect.width>0&&rect.height>0&&!['hidden','collapse'].includes(getComputedStyle(el).visibility)
 }).length>=minimum,{selector,minimum},{timeout:assertionWait(file,line)})
 equal(file,line,await p.locator(selector+':visible').count()>=minimum,true)
}
async function focused(file,line,p,selector) {
  equal(file,line,await p.locator(selector).evaluate(el=>el===document.activeElement),true)
}
async function controller(p,name) {
  await waitForController(p,name)
}
async function mutate(p,name,method) {
  const scope=mode==='pickers'&&name==='multi-select'?'#direct_rooms_control ':''
  await p.evaluate(({name,method,scope})=>{
    const el=document.querySelector(`${scope}[data-controller~='${name}']`)
    const c=window.Stimulus.getControllerForElementAndIdentifier(el,name)
    if(!c)throw Error('mutation controller absent')
    if(method==='focus-cycle')c.trapFocus=e=>{e.preventDefault();c.panelTarget.focus()}
    else if(method==='closed-escape')c.close=e=>e.preventDefault()
    else if(method==='counts'){
      const original=c.update.bind(c)
      c.update=(...args)=>{original(...args);c.messageButtonTarget.textContent='Message (999)';c.huddleButtonTarget.textContent='Start huddle (999)'}
    } else if(method==='all-rows'){
      const original=c.filter.bind(c)
      c.filter=(...args)=>{original(...args);c.rowTargets.forEach(row=>row.hidden=false)}
    } else {if(typeof c[method]!=='function')throw Error('INVALID_CONTROL missing producer method');c[method]=()=>{}}
  },{name,method,scope})
  console.log(`ORIGINAL_MUTATION ${name}.${method}`)
}
async function scenario(name,fn,{user='david',phone=false,path='/rooms/'+labels['rooms.designers']}={}) {
  if(selected&&selected!==name)return
  const context=await browser.newContext({viewport:phone?{width:390,height:844}:{width:1440,height:1000}})
  await context.route('**/*',r=>new URL(r.request().url()).origin===new URL(base).origin?r.continue():r.abort())
  await context.addCookies([{name:'session_token',value:labels[`session_cookies.${user}`],url:base}])
  const p=await context.newPage();p.setDefaultTimeout(10000)
  const diagnose=diagnostics(p);const bad=[]
  p.on('pageerror',e=>bad.push(String(e)))
  p.on('requestfailed',r=>{if(new URL(r.url()).pathname.startsWith('/assets/')&&r.failure()?.errorText!=='net::ERR_ABORTED')bad.push(r.failure().errorText)})
  p.on('response',r=>{if(r.status()>=500)bad.push(`${r.status()} ${r.url()}`)})
  try {
    assert.equal((await visit(p,base+path)).status(),200,'real initial request')
    await controller(p,'profile-card')
    await fn(p)
    equal('transport',0,bad,[])
    console.log(`ORIGINAL_CASE ${name}: passed`)
  } catch(e) {
    await diagnose(e)
    if(bad.length)throw Error(`INVALID_CONTROL transport: ${JSON.stringify(bad)}`,{cause:e})
    throw e
  } finally {await context.close()}
}
const P='test/system/people_group_dms_test.rb'
const T='test/system/first_run_tour_test.rb'
const S='test/system/starred_people_test.rb'
const card=p=>p.locator('#profile-card-popover:not([hidden])')
const hiddenCard=p=>p.locator('#profile-card-popover[hidden]')
const name=p=>p.locator('#user_card .profile-card__name')
const checkbox=(p,u)=>p.locator('#select_user_'+labels['users.'+u])
const directoryBar=p=>p.locator('.people-directory').locator('..').locator("[data-multi-select-target='bar']")
const roomTitle=p=>p.locator('.room--current')
function state(kind,room,users) {requests.push({kind,room,users})}
async function people() {
 await scenario('author',async p=>{
  if(broken)await mutate(p,'profile-card','open')
  await p.locator('#message_0001 .message__avatar a').click()
  await visible(P,16,card(p));await visible(P,18,name(p),'Jason')
  await visible(P,19,p.locator('#user_card').getByRole('button',{name:'Start call',exact:true}))
  await p.locator('#user_card').getByRole('button',{name:'Message',exact:true}).click()
  await p.waitForURL(base+'/rooms/'+labels['rooms.david_and_jason'])
  equal(P,23,new URL(p.url()).pathname,'/rooms/'+labels['rooms.david_and_jason'])
  await visible(P,24,roomTitle(p),'Jason')
 })
 await scenario('author-keyboard',async p=>{
  if(broken)await mutate(p,'profile-card','open')
  await p.locator('#message_0001 .message__author button').press('Enter')
  await visible(P,34,card(p));await visible(P,35,name(p),'Jason')
  await p.locator('.profile-card-popover__panel').press('Escape')
  await present(P,38,hiddenCard(p))
  equal(P,39,await p.evaluate(()=>document.activeElement.tagName),'BUTTON')
  equal(P,40,(await p.evaluate(()=>document.activeElement.textContent)).includes('Jason'),true)
 })
 await scenario('closed-escape',async p=>{
  await present(P,47,hiddenCard(p))
  if(broken)await mutate(p,'profile-card','closed-escape')
  equal(P,59,await p.evaluate(()=>{const e=new KeyboardEvent('keydown',{key:'Escape',bubbles:true,cancelable:true});window.Stimulus.getControllerForElementAndIdentifier(document.body,'profile-card').close(e);return e.defaultPrevented}),false)
 })
 await scenario('sidebar-keyboard',async p=>{
  if(broken)await mutate(p,'profile-card','open')
  await p.locator('#direct_rooms button.profile-card-avatar').first().press('Enter')
  await visible(P,68,card(p));await visible(P,69,name(p))
 })
 await scenario('directory-three',async p=>{
  await controller(p,'multi-select');if(broken)await mutate(p,'multi-select','counts')
  for(const u of ['jason','kevin','jz'])await checkbox(p,u).check()
  await visible(P,81,directoryBar(p).getByRole('button',{name:'Message (3)',exact:true}))
  await directoryBar(p).getByRole('button',{name:'Message (3)',exact:true}).click()
  await p.waitForURL(/\/rooms\/\d+$/)
  await visible(P,85,roomTitle(p),'Jason, JZ, Kevin')
  state('direct',Number(new URL(p.url()).pathname.split('/').at(-1)),['david','jason','kevin','jz'])
 },{path:'/users'})
 await scenario('member-huddle',async p=>{
  await controller(p,'member-panel');await controller(p,'multi-select')
  if(await p.getByRole('button',{name:'Show members',exact:true}).isVisible())await p.getByRole('button',{name:'Show members',exact:true}).click()
  await visibleMinimum(P,97,p,'#channel-members .member-panel__member',3)
  if(broken)await mutate(p,'multi-select','counts')
  for(const u of ['jason','kevin'])await p.locator(`#channel-members [data-member-id='${labels['users.'+u]}'] button.profile-card-name`).click({modifiers:['Control']})
  const bar=p.locator("#channel-members [data-multi-select-target='bar']")
  await visible(P,103,bar.getByRole('button',{name:'Start huddle (2)',exact:true}))
  await bar.getByRole('button',{name:'Start huddle (2)',exact:true}).click()
  await p.waitForURL(/\/rooms\/\d+\?huddle=start$/)
  await visible(P,107,roomTitle(p),'Jason, Kevin')
  equal(P,109,new URL(p.url()).search,'?huddle=start')
  state('direct',Number(new URL(p.url()).pathname.split('/').at(-1)),['david','jason','kevin'])
 })
 await scenario('shift-range',async p=>{
  await controller(p,'multi-select');if(broken)await mutate(p,'multi-select','counts')
  await checkbox(p,'bender').check();await checkbox(p,'kevin').click({modifiers:['Shift']})
  await visible(P,121,directoryBar(p).getByRole('button',{name:'Message (4)',exact:true}))
  await visible(P,122,directoryBar(p).getByRole('button',{name:'Start huddle (3)',exact:true}))
 },{path:'/users'})
 await scenario('long-press',async p=>{
  await controller(p,'multi-select');if(broken)await mutate(p,'multi-select','pressStart')
  const row=p.locator('.people-directory__row').filter({hasText:'Jason'})
  await row.evaluate(el=>{const t=new Touch({identifier:1,target:el,clientX:10,clientY:10});el.dispatchEvent(new TouchEvent('touchstart',{touches:[t],bubbles:true,cancelable:true}))})
  await p.waitForTimeout(700) // The original deliberately holds the finger for 0.7s.
  await row.evaluate(el=>el.dispatchEvent(new TouchEvent('touchend',{bubbles:true,cancelable:true})))
  await row.locator('button.profile-card-name').evaluate(el=>el.dispatchEvent(new MouseEvent('click',{bubbles:true,cancelable:true,clientX:10,clientY:10})))
  equal(P,147,await checkbox(p,'jason').isChecked(),true)
  await visible(P,149,directoryBar(p).getByRole('button',{name:'Message (1)',exact:true}))
  await present(P,151,hiddenCard(p))
 },{path:'/users'})
 await scenario('agent',async p=>{
  await controller(p,'multi-select');if(broken)await mutate(p,'multi-select','counts')
  await checkbox(p,'bender').check()
  await visible(P,161,directoryBar(p).getByRole('button',{name:'Message (1)',exact:true}))
  await visible(P,162,directoryBar(p).locator("[data-multi-select-target='huddleButton'][disabled]").filter({hasText:'Start huddle (0)'}))
  equal(P,163,(await directoryBar(p).innerText()).includes("Agents can't join huddles."),true)
 },{path:'/users'})
 await scenario('mixed',async p=>{
  await controller(p,'multi-select');if(broken)await mutate(p,'multi-select','counts')
  await checkbox(p,'jason').check();await checkbox(p,'bender').check()
  await visible(P,175,directoryBar(p).getByRole('button',{name:'Message (2)',exact:true}))
  await visible(P,176,directoryBar(p).getByRole('button',{name:'Start huddle (1)',exact:true}))
  equal(P,177,(await directoryBar(p).innerText()).includes("1 agent stays in the DM but won't be rung."),true)
 },{path:'/users'})
}
const filter=p=>p.locator('#dm_picker_filter')
const rows=p=>p.locator('.dm-picker__row:not([hidden]):visible')
const pick=(p,u)=>p.locator('#pick_user_'+labels['users.'+u])
const pickerBar=p=>p.locator("#direct_rooms_control [data-multi-select-target='bar']")
async function picker(p,phone=false) {
 if(phone){await p.getByRole('button',{name:'Open workspace navigation',exact:true}).click();await visible(P,292,p.locator('#sidebar.open'))}
 await p.getByRole('link',{name:'New direct message',exact:true}).click();await controller(p,'dm-picker');await controller(p,'multi-select')
}
async function pickers() {
 await scenario('picker-filter',async p=>{
  await picker(p);if(broken)await mutate(p,'dm-picker','all-rows')
  equal(P,189,await p.locator('#direct_rooms_control suggestion-option:visible').count(),0)
  equal(P,190,(await p.locator('#direct_rooms_control').innerText()).includes('Start Ping'),false)
  if(mutation==='picker-visibility') {
   await p.addStyleTag({content:`.dm-picker__row:not(:has(#pick_user_${labels['users.chad']})):not(:has(#pick_user_${labels['users.renee']})) { visibility: hidden !important; }`})
   assert.equal(await p.locator('.dm-picker__row:visible').count(),2,'INVALID_CONTROL two visible initial picker rows')
   console.log('ORIGINAL_MUTATION only two initial picker rows visible; filter matches retained')
  }
  const total=await rows(p).count();equal(P,192,total>2,true)
  for(const [q,line,text] of [['chad',195,'Chad Puterbaugh'],['CHA',198,'Chad Puterbaugh'],['renee',201,'Renée Dupont']]) {
   await filter(p).fill(q);equal(P,line,await rows(p).count(),1);equal(P,line,(await rows(p).innerText()).includes(text),true)
  }
  await filter(p).fill('zzz-no-one');equal(P,204,await rows(p).count(),0)
  await visible(P,205,p.locator("[data-dm-picker-target='empty']"),'No one matches')
  await filter(p).fill('');equal(P,208,await rows(p).count(),total)
  equal(P,209,await p.locator("[data-dm-picker-target='empty']:not([hidden]):visible").count(),0)
 })
 await scenario('picker-post',async p=>{
  await picker(p);if(broken)await mutate(p,'multi-select','counts')
  await pick(p,'chad').check();await filter(p).fill('kevin')
  equal(P,223,await rows(p).filter({hasText:'Chad Puterbaugh'}).count(),0)
  await visible(P,226,pickerBar(p).getByRole('button',{name:'Message (1)',exact:true}))
  await filter(p).fill('');equal(P,232,await pick(p,'chad').isChecked(),true)
  await pickerBar(p).getByRole('button',{name:'Message (1)',exact:true}).click();await p.waitForURL(/\/rooms\/\d+$/)
  await visible(P,238,roomTitle(p),'Chad');state('direct',Number(new URL(p.url()).pathname.split('/').at(-1)),['david','chad'])
 })
 await scenario('picker-enter',async p=>{
  await picker(p);if(broken)await mutate(p,'dm-picker','selectSingle')
  await filter(p).fill('j');equal(P,252,await rows(p).count()>=2,true)
  await filter(p).press('Enter');equal(P,254,await p.locator("#direct_rooms_control [data-multi-select-target='bar']:not([hidden]):visible").count(),0)
  await filter(p).fill('chad');await filter(p).press('Enter');equal(P,260,await pick(p,'chad').isChecked(),true)
  await filter(p).fill('kevin');await filter(p).press('Enter')
  equal(P,267,await pick(p,'chad').isChecked(),true);equal(P,268,await pick(p,'kevin').isChecked(),false)
  await visible(P,270,pickerBar(p).getByRole('button',{name:'Message (1)',exact:true}))
 })
 await scenario('picker-row',async p=>{
  await picker(p);if(broken)await mutate(p,'dm-picker','toggleRow')
  const row=p.locator('.dm-picker__row').filter({hasText:'Kevin'})
  await row.evaluate(el=>el.click());equal(P,281,await pick(p,'kevin').isChecked(),true)
  await row.locator('button.profile-card-name').click();await visible(P,284,card(p));await visible(P,285,name(p),'Kevin')
 })
 await scenario('picker-phone',async p=>{
  await picker(p,true);await filter(p).fill('j')
  if(broken){console.log('ORIGINAL_MUTATION actual responsive row height rule');await p.addStyleTag({content:'.dm-picker__row { height: 1px !important; min-height: 1px !important; overflow: hidden !important; }'})}
  equal(P,299,await p.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),true)
  equal(P,303,(await rows(p).first().boundingBox()).height>=44,true)
  equal(P,305,Math.abs((await rows(p).first().locator('.avatar').boundingBox()).width-32)<=1,true)
 },{phone:true})
}
async function settled(p) {
 await p.waitForFunction(()=>{const el=document.querySelector('.member-panel__surface');return !el||el.getAnimations().every(a=>{const t=a.effect?.getComputedTiming?.();return !(t&&Number.isFinite(t.endTime)&&(a.playState==='running'||a.playState==='pending'))})},null,{timeout:2000})
}
async function members() {
 for(const kind of ['mobile-escape','mobile-tab'])await scenario(kind,async p=>{
  await controller(p,'member-panel');if(await p.getByRole('button',{name:'Show members',exact:true}).isVisible())await p.getByRole('button',{name:'Show members',exact:true}).click()
  if(mutation==='member-visibility') {
   await p.locator('#channel-members .member-panel__member').first().waitFor()
   const first=await p.locator('#channel-members .member-panel__member').first().getAttribute('data-member-id')
   await p.addStyleTag({content:`#channel-members .member-panel__member:not([data-member-id='${first}']):not([data-member-id='${labels['users.kevin']}']) { visibility: hidden !important; }`})
   const rows=p.locator('#channel-members .member-panel__member')
   assert.ok(await rows.count()>=3,'INVALID_CONTROL attached fixture rows')
   assert.equal(await p.locator('#channel-members .member-panel__member:visible').count(),2,'INVALID_CONTROL expected exactly two visible rows')
   console.log('ORIGINAL_MUTATION visible members reduced to two with attached rows retained')
  }
  await visibleMinimum(P,kind==='mobile-escape'?372:396,p,'#channel-members .member-panel__member',3)
  await settled(p);if(broken)await mutate(p,'profile-card',kind==='mobile-escape'?'close':'focus-cycle')
  const trigger=`#channel-members [data-member-id='${labels['users.kevin']}'] button.profile-card-name`
  await p.locator(trigger).click();await visible(P,kind==='mobile-escape'?377:400,card(p))
  if(kind==='mobile-escape') {
   await p.locator('.profile-card-popover__panel').press('Escape');await present(P,381,hiddenCard(p))
   await visible(P,382,p.locator('#channel-members'));await visible(P,383,p.getByRole('button',{name:'Close members',exact:true}));await focused(P,384,p,trigger)
  } else {
   await visible(P,401,name(p),'Kevin');await p.locator('.profile-card-popover__panel').press('Tab')
   await focused(P,406,p,'.profile-card-popover__close');await p.locator('.profile-card-popover__close').press('Tab')
   await focused(P,408,p,'#user_card .profile-card__actions form:nth-of-type(1) button');await p.locator('#user_card .profile-card__actions form:nth-of-type(1) button').press('Tab')
   await focused(P,410,p,'#user_card .profile-card__actions form:nth-of-type(2) button')
  }
 },{phone:true})
}
async function group() {
 await scenario('group-lifecycle',async p=>{
  await controller(p,'multi-select');await p.locator("[data-multi-select-target='checkbox']").first().waitFor()
  for(const u of ['jason','kevin'])await checkbox(p,u).check()
  await directoryBar(p).getByRole('button',{name:'Message (2)',exact:true}).click();await p.waitForURL(/\/rooms\/\d+$/)
  const room=Number(new URL(p.url()).pathname.split('/').at(-1))
  await p.locator(`nav a[href='/rooms/directs/${room}/edit']:visible`).click()
  if(mutation==='group-notice') {
   await p.route(`**/rooms/directs/${room}`,async route=>{
    const response=await route.fetch();const body=await response.text()
    assert.ok(body.includes('Group renamed.'),'INVALID_CONTROL real rename response notice')
    await route.fulfill({response,body:body.replaceAll('Group renamed.','Notice removed by producer control')})
    console.log('ORIGINAL_MUTATION actual rename response loses notice')
   })
  }
  if(broken){await p.locator("form:has(input[name='room[name]'])").evaluate(el=>el.addEventListener('submit',e=>e.preventDefault()));console.log('ORIGINAL_MUTATION real rename form submission suppressed')}
  await p.getByLabel('Group name',{exact:true}).fill('Weekend Plans');await p.getByRole('button',{name:'Save',exact:true}).click()
  await visible(P,321,p.getByText('Group renamed.',{exact:true}).first())
  equal(P,322,await p.getByLabel('Group name',{exact:true}).inputValue(),'Weekend Plans')
  await p.locator('#add_member_'+labels['users.jz']).check();await p.getByRole('button',{name:'Add to group',exact:true}).click()
  await visible(P,326,p.locator('.directs--edit'),'JZ')
  await p.goto(base+'/rooms/'+room);await visible(P,329,roomTitle(p),'Weekend Plans')
  await visible(P,330,p.locator('.message__system-note'),'renamed the group to Weekend Plans')
  await visible(P,331,p.locator('.message__system-note'),'added JZ to the group')
  equal(P,332,/David\s+renamed the group to Weekend Plans/.test(await p.locator('body').innerText()),true)
  equal(P,333,/David\s+added JZ to the group/.test(await p.locator('body').innerText()),true)
  await p.goto(base+`/rooms/directs/${room}/edit`);p.on('dialog',d=>d.accept())
  if(mutation==='group-navigation') {
   await p.evaluate(()=>{document.addEventListener('turbo:before-visit',e=>e.preventDefault());document.addEventListener('turbo:before-render',e=>e.preventDefault())})
   // Still submit the real leave action and verify its persisted membership below.
   const leave=p.waitForResponse(r=>r.request().method()==='POST'&&new URL(r.url()).pathname.endsWith('/leave'))
   await p.getByRole('button',{name:'Leave group',exact:true}).click();await leave
   console.log('ORIGINAL_MUTATION real leave saved but Turbo navigation suppressed')
  } else await p.getByRole('button',{name:'Leave group',exact:true}).click()
  await p.waitForURL(url=>url.pathname!==`/rooms/directs/${room}/edit`,{timeout:assertionWait(P,340)})
  equal(P,340,new URL(p.url()).pathname!==`/rooms/directs/${room}/edit`,true)
  await p.waitForURL(url=>url.pathname!='/rooms/'+room,{timeout:assertionWait(P,341)})
  equal(P,341,new URL(p.url()).pathname!='/rooms/'+room,true);state('left',room,['david'])
 },{path:'/users'})
}
const tour=p=>p.locator('#tour .tour__card')
const next=p=>p.locator("#tour [data-tour-target='next']")
async function step(p,line,n) {await visible(T,line,p.locator('.tour__progress'),`Step ${n} of 5`)}
async function stamped(p,action,hiddenLine,stampLine) {
 const response=p.waitForResponse(r=>new URL(r.url()).pathname==='/users/me/tour'&&r.request().method()!=='GET',{timeout:2000})
 await action();assert.equal((await response).status(),204,'real stamp request')
 await tour(p).waitFor({state:'hidden',timeout:2000});equal(T,hiddenLine,await tour(p).isVisible(),false)
 // The 204 follows the save. Read the actual HTTP server's database now, before
 // restart/navigation, matching assert_tour_completed's reloaded user predicate.
 const db=new DatabaseSync(process.env.WS11UI_BROWSER_DATABASE,{readOnly:true})
 try {equal(T,95,db.prepare('SELECT tour_completed_at FROM users WHERE id=?').get(labels['users.jz']).tour_completed_at!==null,true)} finally {db.close()}
 requests.push({kind:'tour',user:'jz',line:stampLine})
}
async function complete(p,action,hiddenLine,stampLine,attributeLine,finalLine) {
 await stamped(p,action,hiddenLine,stampLine)
 await p.reload();await controller(p,'tour')
 await present(T,attributeLine,p.locator('#tour[data-tour-auto-start-value="false"]'))
 equal(T,finalLine,await tour(p).isVisible(),false)
}
async function tours() {
 await scenario('tour-finish',async p=>{
  await controller(p,'tour');if(broken)await mutate(p,'tour','finish')
  await visible(T,12,tour(p));await step(p,13,1);await visible(T,14,p.locator('.tour__title'),'Your rooms live here');await visible(T,15,p.locator('#sidebar.tour__target'))
  await next(p).click();await step(p,18,2);await visible(T,19,p.locator('#composer.tour__target'))
  await next(p).press('ArrowRight');await step(p,22,3);await visible(T,23,p.locator('.tour__card--center'))
  await next(p).press('ArrowRight');await step(p,26,4);await visible(T,27,p.locator('.tour__title'),'Jump anywhere with Ctrl+K')
  await next(p).press('ArrowLeft');await step(p,30,3);await next(p).press('ArrowRight');await next(p).press('ArrowRight');await step(p,33,5)
  await visible(T,34,p.locator('.tour__title'),'Shortcuts live under ?');await visible(T,35,p.locator('#help-menu-button.tour__target'))
  await complete(p,()=>next(p).click(),38,39,42,43)
 },{user:'jz'})
 await scenario('tour-escape',async p=>{
  await controller(p,'tour');await visible(T,50,tour(p));if(broken)await mutate(p,'tour','skip')
  await complete(p,()=>next(p).press('Escape'),53,54,57,58)
 },{user:'jz'})
 await scenario('tour-restart',async p=>{
  if(mutation==='tour-stamp') {
   await p.route('**/users/me/tour',route=>{console.log('ORIGINAL_MUTATION tour completion PATCH acknowledged without persistence');return route.fulfill({status:204,body:''})})
  }
  await controller(p,'tour');await stamped(p,()=>p.getByRole('button',{name:'Skip tour',exact:true}).click(),66,67)
  if(broken)await mutate(p,'tour','start')
  await p.locator('#help-menu-button').click();await p.getByRole('menuitem',{name:'Restart tour',exact:true}).click()
  await visible(T,72,tour(p));await step(p,73,1)
 },{user:'jz'})
 await scenario('tour-completed',async p=>{
  await controller(p,'tour')
  if(mutation==='tour-auto-start') {
   await p.locator('#tour').evaluate(el=>el.setAttribute('data-tour-auto-start-value','true'))
   console.log('ORIGINAL_MUTATION completed page auto-start attribute incorrect')
  }
  if(broken){await p.evaluate(()=>{const c=window.Stimulus.getControllerForElementAndIdentifier(document.querySelector('#tour'),'tour');c.start()});console.log('ORIGINAL_MUTATION completed tour started')}
  await present(T,82,p.locator('#tour[data-tour-auto-start-value="false"]'))
  equal(T,83,await tour(p).isVisible(),false);await visible(T,84,p.locator('#help-menu-button'))
 },{user:'jz'})
}
async function corruptStarPayload(p) {
  await p.route(/\/rooms\/\d+\/members(?:\.json)?(?:\?.*)?$/,async route=>{
    const response=await route.fetch();assert.equal(response.status(),200,'INVALID_CONTROL members transport')
    const payload=await response.json();assert.ok(Array.isArray(payload.members),'INVALID_CONTROL members source')
    payload.members.forEach(member=>{member.starred=false})
    await route.fulfill({response,json:payload})
    console.log('ORIGINAL_MUTATION actual polled member payload loses starred flags')
  })
}
// Keep the four starred-people translations separate to retain each helper's checks.
async function stars() {
 let inlineCalls=0
 const id=labels['users.kevin'];const row=p=>p.locator(`#channel-members [data-member-id='${id}']`);const menu=p=>p.locator('#member-row-menu:visible')
 async function dismissed(p,line) {await menu(p).waitFor({state:'hidden',timeout:assertionWait(S,line)});equal(S,line,await menu(p).count(),0)}
 async function open(p) {await row(p).locator('button.profile-card-name').click();await visible(S,134,card(p));await visible(S,135,name(p),'Kevin')}
 async function close(p) {await p.locator('.profile-card-popover__close').click();await present(S,140,hiddenCard(p))}
 async function marked(p,yes) {
  if(yes) {
   await present(S,144,p.locator(`#channel-members [aria-label='Starred members'] [data-member-id='${id}'][data-starred='true']`).filter({hasText:'Kevin'}))
   equal(S,146,await p.locator(`#channel-members [aria-label='Online members'] [data-member-id='${id}']`).count(),0)
   equal(S,147,await p.locator(`#channel-members [aria-label='Offline members'] [data-member-id='${id}']`).count(),0)
   await present(S,148,p.locator(`#channel-members [aria-label='Starred members'] [data-member-id='${id}'] .member-panel__presence`))
  } else {
   await p.locator("#channel-members [aria-label='Starred members']").waitFor({state:'detached',timeout:assertionWait(S,153)});equal(S,153,await p.locator("#channel-members [aria-label='Starred members']").count(),0)
   await present(S,154,p.locator(`#channel-members [aria-label='Offline members'] [data-member-id='${id}'][data-starred='false']`).filter({hasText:'Kevin'}))
  }
 }
 async function inline(p) {
  if(mutation==='identity-visibility'&&++inlineCalls===2) {
   await p.addStyleTag({content:'#channel-members .member-panel__identity { visibility: hidden !important; }'})
   console.log('ORIGINAL_MUTATION member identities hidden but layout retained')
  }
  await visible(S,168,p.locator('#channel-members .member-panel__member .member-panel__identity:visible'))
  const brokenRows=await p.locator('#channel-members .member-panel__member').evaluateAll(rows=>rows.filter(r=>{const a=r.querySelector('.member-panel__avatar').getBoundingClientRect();const i=r.querySelector('.member-panel__identity').getBoundingClientRect();return i.left<a.right||i.top>=a.bottom||i.width<80}).map(r=>r.dataset.memberId))
  equal(S,176,brokenRows,[])
 }
 async function ready(p) {await visible(S,11,p.locator('#channel-members'));await controller(p,'member-panel');await p.locator('#channel-members .member-panel__member').first().waitFor();await settled(p)}
 await scenario('star-card',async p=>{
  await ready(p);equal(S,21,await p.locator("#channel-members [aria-label='Starred members']").count(),0)
  if(broken)await corruptStarPayload(p)
  await open(p);await p.locator('#user_card').getByRole('button',{name:'☆ Star',exact:true}).click();await visible(S,26,p.locator('#user_card').getByRole('button',{name:'★ Unstar',exact:true}))
  await marked(p,true);await inline(p);await close(p);await open(p)
  await p.locator('#user_card').getByRole('button',{name:'★ Unstar',exact:true}).click();await visible(S,36,p.locator('#user_card').getByRole('button',{name:'☆ Star',exact:true}));await close(p);await marked(p,false);await inline(p)
 },{user:'jz'})
 await scenario('star-menu',async p=>{
  await ready(p);if(broken)await mutate(p,'member-panel','toggleRowMenuStar')
  if(mutation==='menu-rendered') {
   await p.addStyleTag({content:'#member-row-menu[hidden] { display: block !important; visibility: visible !important; opacity: 1 !important; pointer-events: none !important; }'})
   console.log('ORIGINAL_MUTATION hidden menu CSS still renders dismissed menu')
  }
  await row(p).click({button:'right'});await visible(S,48,menu(p));await focused(S,49,p,"#member-row-menu [role='menuitem']")
  await menu(p).getByRole('menuitem',{name:'☆ Star',exact:true}).click();await visible(S,53,menu(p).getByRole('menuitem',{name:'★ Unstar',exact:true}));await marked(p,true)
  await menu(p).getByRole('menuitem',{name:'★ Unstar',exact:true}).click();await visible(S,59,menu(p).getByRole('menuitem',{name:'☆ Star',exact:true}));await marked(p,false)
  await menu(p).locator("[role='menuitem']").press('Escape');await dismissed(p,64)
  equal(S,159,await row(p).evaluate(el=>el.contains(document.activeElement)),true)
  await row(p).locator('button.profile-card-name').focus();await p.keyboard.press('Shift+F10');await visible(S,72,menu(p));await focused(S,73,p,"#member-row-menu [role='menuitem']")
  await menu(p).getByRole('menuitem',{name:'☆ Star',exact:true}).click();await marked(p,true);await menu(p).locator("[role='menuitem']").press('Escape');await dismissed(p,79)
  await p.locator(`#channel-members [data-member-id='${labels['users.jz']}']`).click({button:'right'});await dismissed(p,83);await inline(p)
 },{user:'jz'})
 await scenario('star-escape',async p=>{
  await ready(p);await row(p).click({button:'right'});await visible(S,93,menu(p))
  await row(p).locator('button.profile-card-avatar').click();await visible(S,100,card(p));await present(S,101,p.locator('#member-row-menu:not([hidden])'))
  if(broken)await mutate(p,'profile-card','close')
  await p.locator('.profile-card-popover__close').press('Escape');await present(S,105,hiddenCard(p));await dismissed(p,106)
 },{user:'jz'})
 await scenario('star-phone',async p=>{
  await ready(p);await p.setViewportSize({width:390,height:844});await p.locator('#channel-members').waitFor({state:'hidden',timeout:assertionWait(S,112)});
  equal(S,112,await p.locator('#channel-members').isVisible(),false)
  await p.getByRole('button',{name:'Show members',exact:true}).click();await visible(S,115,row(p));await settled(p)
  if(broken)await corruptStarPayload(p)
  await open(p);await p.locator('#user_card').getByRole('button',{name:'☆ Star',exact:true}).click();await visible(S,120,p.locator('#user_card').getByRole('button',{name:'★ Unstar',exact:true}));await close(p);await marked(p,true);await inline(p)
  equal(S,180,await p.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),true)
 },{user:'jz'})
}
try {
 const suites={people,pickers,members,group,tours,stars};assert.ok(suites[mode]);await suites[mode]()
 console.log('ORIGINAL_STATE_REQUESTS '+JSON.stringify(requests))
} finally {await browser.close();proxy.close()}
