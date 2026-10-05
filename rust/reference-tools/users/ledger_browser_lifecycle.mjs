import assert from 'node:assert/strict'
import fs from 'node:fs'
import {DatabaseSync} from 'node:sqlite'
import path from 'node:path'
import {chromium} from 'playwright'
import {network} from './original_browser_network.mjs'
import {visit,waitForController} from './browser_navigation.mjs'
import {fixtureAction} from './ledger_fixture_bridge.mjs'
import {displayedCount} from './selenium_displayed.mjs'
import {viewportFor} from './ledger_browser_viewports.mjs'
const base=process.env.WS11UI_BROWSER_URL
const proxy=await network(base)
const labels=JSON.parse(fs.readFileSync(process.env.WS11UI_BROWSER_LABELS,'utf8'))
const selected=process.env.WS11UI_BROWSER_CASE
const mutation=process.env.WS11UI_BROWSER_MUTATION||(process.env.WS11UI_BROWSER_CONTROL==='1'?({'muted-delivery':'muted-writer','calendar-lifecycle':'calendar-opt-in-writer'}[selected]):'')
const database=new DatabaseSync(process.env.WS11UI_BROWSER_DATABASE)
const browser=await chromium.launch({headless:true,args:['--no-sandbox','--mute-audio']})
const context=await browser.newContext({viewport:viewportFor(labels,1400,1400)})
await context.route('**/*',r=>new URL(r.request().url()).origin===new URL(base).origin?r.continue():r.abort())
const page=await context.newPage();page.setDefaultTimeout(10000)
function equal(file,line,actual,expected) {assert.deepEqual(actual,expected,`${file}:${line}: original assertion`);console.log(`ORIGINAL_ASSERTION ${file}:${line}`)}
async function selector(file,line,css,text=null,{wait=2000,absent=false}={}) {
 const deadline=performance.now()+wait
 while(true) {
  const count=await displayedCount(page.locator(css),text)
  if(absent?count===0:count>0){equal(file,line,absent?count===0:count>0,true);return}
  if(performance.now()>=deadline)throw Error(`${file}:${line}: original assertion`)
  await new Promise(resolve=>setTimeout(resolve,20))
 }
}
async function unread(message,room,file,line) {
 const deadline=performance.now()+10000
 while(true) {
  await fixtureAction({action:'broadcast',message})
  try {
   await selector('test/test_helpers/system_test_helper.rb',131,'.rooms a.unread',room,{wait:1000})
   console.log(`ORIGINAL_ASSERTION ${file}:${line}`);return
  } catch(e) {if(performance.now()>deadline)throw e}
 }
}
const state=[]
try {
 await visit(page,base+'/test_session?'+new URLSearchParams({email_address:(selected==='muted-delivery'?'jz':'david')+'@37signals.com',password:'secret123456'}))
 await selector('test/test_helpers/system_test_helper.rb',65,'a.btn','Designers',{wait:10000})
 if(selected==='muted-delivery') {
  const file='test/system/sidebar_organize_test.rb'
  // Original class setup signs in JZ before this declaration signs in David.
  await visit(page,base+'/test_session?'+new URLSearchParams({email_address:'david@37signals.com',password:'secret123456'}))
  await selector('test/test_helpers/system_test_helper.rb',65,'a.btn','Designers',{wait:10000})
  await visit(page,base+'/rooms/'+labels['rooms.pets'])
  await page.waitForFunction(()=>{const all=[...document.querySelectorAll('turbo-cable-stream-source')];return all.length>=3&&all.every(el=>el.hasAttribute('connected'))},null,{timeout:15000})
  database.prepare('UPDATE memberships SET connected_at=NULL,connections=0 WHERE user_id=? AND room_id=?').run(labels['users.david'],labels['rooms.designers'])
  await waitForController(page,'room-menu')
  if(mutation==='muted-writer') {
   database.exec("CREATE TRIGGER ledger_broken_mute AFTER UPDATE OF involvement ON memberships WHEN NEW.involvement='muted' BEGIN UPDATE memberships SET involvement='everything' WHERE id=NEW.id; END")
   console.log('ORIGINAL_MUTATION membership involvement writer')
  }
  await page.locator('#sidebar a[data-room-id="'+labels['rooms.designers']+'"]').click({button:'right'})
  await selector(file,148,'#room-menu:not([hidden])',null,{wait:5000})
  await page.locator('#room-menu').getByText('Mute',{exact:true}).click()
  await selector(file,70,'#sidebar a.muted[data-room-id="'+labels['rooms.designers']+'"]',null,{wait:10000})
  state.push({kind:'muted'})
  await fixtureAction({action:'message',room:labels['rooms.designers'],creator:labels['users.kevin'],body:'Muted noise',key:'mute-quiet-1'})
  const loud=await fixtureAction({action:'message',room:labels['rooms.watercooler'],creator:labels['users.kevin'],body:'Loud hello',key:'mute-quiet-2'})
  await unread(loud.id,database.prepare('SELECT name FROM rooms WHERE id=?').get(labels['rooms.watercooler']).name,file,76)
  await selector('test/test_helpers/system_test_helper.rb',127,'.rooms a:not(.unread)','Designers',{wait:5000})
  console.log(`ORIGINAL_ASSERTION ${file}:77`)
  const mention=JSON.parse(fs.readFileSync(path.join(path.dirname(process.env.WS11UI_BROWSER_DATABASE),'ledger-mention.json'),'utf8'))
  const message=await fixtureAction({action:'message',room:labels['rooms.designers'],creator:labels['users.kevin'],body:'Hey '+mention,key:'mute-quiet-3'})
  await unread(message.id,'Designers',file,82)
 } else if(selected==='calendar-lifecycle') {
  const file='test/system/meeting_status_test.rb'
  await visit(page,base+'/users/me/profile')
  if(mutation==='calendar-opt-in-writer') {
   database.exec('CREATE TRIGGER ledger_broken_meeting AFTER UPDATE OF meeting_status_enabled ON users BEGIN UPDATE users SET meeting_status_enabled=0 WHERE id=NEW.id; END')
   console.log('ORIGINAL_MUTATION Calendar opt-in writer')
  }
  await page.locator('#user_meeting_status_enabled').locator('xpath=ancestor::label').click()
  await page.locator('form[action="/users/me/status"]').locator('button[type="submit"]').first().click()
  const deadline=performance.now()+10000
  while(database.prepare('SELECT meeting_status_enabled FROM users WHERE id=?').get(labels['users.david']).meeting_status_enabled!==1) {assert.ok(performance.now()<deadline,`${file}:46: original assertion meeting status was not enabled`);await new Promise(r=>setTimeout(r,50))}
  equal(file,49,true,true);state.push({kind:'calendar-enabled'})
  await fixtureAction({action:'refresh',user:labels['users.david']})
  await visit(page,base+'/users/'+labels['users.david'])
  await selector(file,24,'.user-status-badge','In a meeting')
  const result=await fixtureAction({action:'advance',user:labels['users.david']})
  assert.equal(result.clock,'2026-03-02T16:06:00Z','injected clock advances six minutes')
  await visit(page,base+'/users/'+labels['users.david'])
  await selector(file,30,'.user-status-badge__custom','In a meeting',{absent:true})
 } else throw Error('unknown original sequence')
 console.log(`ORIGINAL_CASE ${selected}: passed`)
 console.log('ORIGINAL_STATE_REQUESTS '+JSON.stringify(state))
} finally {
 database.exec('DROP TRIGGER IF EXISTS ledger_broken_mute; DROP TRIGGER IF EXISTS ledger_broken_meeting');
 console.log(`ORIGINAL_PRODUCER_RESTORED ${selected}: fixture triggers removed`)
 database.close();await context.close();await browser.close();await proxy.close()
}
