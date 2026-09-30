import { chromium } from '../parity/node_modules/playwright/index.mjs';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
const db=process.argv[2], base='http://127.0.0.1:52471', DAVID=127326141,JASON=149087659,ROOM=654632876,ALL_TALK=486777696,DM=186869642;
const vectors=JSON.parse(readFileSync(new URL('../vectors/campfire_sessions.json',import.meta.url)));
const sql=(query,args=[])=>JSON.parse(execFileSync('python3',['-c','import sqlite3,json,sys; c=sqlite3.connect(sys.argv[1]); r=c.execute(sys.argv[2],json.loads(sys.argv[3])); print(json.dumps(r.fetchall())); c.commit()',db,query,JSON.stringify(args)],{encoding:'utf8'}));
const browser=await chromium.launch({executablePath:process.env.WS17_CHROMIUM||'/usr/bin/chromium',headless:true,args:['--no-sandbox']});
const results=[];
async function cookie(ctx,name='David') {
 const header=vectors.sessions.find(s=>s.user_name===name&&!s.expired&&s.two_factor_verified).cookie_header;
 const [key,...parts]=header.split('='); await ctx.addCookies([{name:key,value:parts.join('=').split(';')[0],url:base}]);
}
async function context() {
 const ctx=await browser.newContext({viewport:{width:1400,height:1400}});
 await ctx.addInitScript(()=>{const NativeDate=Date; window.Date=class extends NativeDate {constructor(...args){super(...(args.length?args:['2026-03-02T16:00:00Z']));} static now(){return new NativeDate('2026-03-02T16:00:00Z').getTime();}};});
 await cookie(ctx); return ctx;
}
async function run(file,title,test) {
 sql("UPDATE users SET presence_setting='auto',custom_status_emoji=NULL,custom_status_text=NULL,custom_status_expires_at=NULL,dnd_enabled=0,dnd_until=NULL,quiet_hours_enabled=0,quiet_hours_start_minute=NULL,quiet_hours_end_minute=NULL,ooo_until=NULL,ooo_note=NULL,ooo_broadcast=NULL,ooo_calendar_enabled=0,ooo_notify_enabled=0,meeting_status_enabled=0,meeting_dnd_enabled=0,theme='system',text_size='default',time_zone='UTC'");
 sql('DELETE FROM calendar_meeting_caches'); sql('DELETE FROM workspace_presence_leases');
 const ctx=await context(),page=await ctx.newPage(); page.setDefaultTimeout(10000);
 try {await test(page,ctx); results.push({file,title});console.log(`PASS ${file}: ${title}`);}
 catch(error) {console.error(`FAIL ${file}: ${title}`);throw error;}
 finally {await ctx.close();}
}
const visit=(p,path)=>p.goto(base+path);
const submit=(p,action)=>p.locator(`form[action='${action}'] button[type=submit]`).first().click();
async function until(fn) {for(let i=0;i<100;i++){if(await fn())return;await new Promise(r=>setTimeout(r,50));}assert.fail('condition did not become true');}
const stored=(column)=>sql(`SELECT ${column} FROM users WHERE id=?`,[DAVID])[0][0];
try {
 const file='test/system/status_notifications_test.rb';
 await run(file,'setting presence and a custom status',async p=>{
  const session=sql('SELECT id FROM sessions WHERE user_id=? LIMIT 1',[DAVID])[0][0];
  sql("INSERT INTO workspace_presence_leases(connection_id,session_id,user_id,expires_at,last_active_at,created_at,updated_at) VALUES ('ws17-browser',?,?,'2026-03-02 16:01:30.000000','2026-03-02 16:00:00.000000','2026-03-02 16:00:00.000000','2026-03-02 16:00:00.000000')",[session,DAVID]);
  await visit(p,'/users/me/profile');await p.selectOption('#user_presence_setting','dnd');await p.fill('#user_custom_status_emoji','🚂');await p.fill('#user_custom_status_text','On a train');await p.selectOption('#user_custom_status_expires_in','hour_1');await submit(p,'/users/me/status');
  await until(()=>stored('custom_status_text')==='On a train');assert.equal(stored('presence_setting'),'dnd');
  await visit(p,`/users/${DAVID}`);assert.match(await p.locator('.user-status-badge').innerText(),/Do not disturb/);assert.match(await p.locator('.user-status-badge').innerText(),/🚂 On a train/);
 });
 await run(file,'enabling DND mutes sounds and persists quiet hours',async p=>{
  await visit(p,'/users/me/profile');await p.locator('label').filter({has:p.locator('#user_dnd_enabled')}).click();await submit(p,'/users/me/notification_settings');await until(()=>stored('dnd_enabled')===1);
  await visit(p,`/rooms/${ROOM}`);assert.equal(await p.locator('meta[name=notification-dnd]').getAttribute('content'),'muted');
  await visit(p,'/users/me/profile');assert.equal(await p.locator('#user_dnd_enabled').isChecked(),true);
 });
 await run(file,'chat sounds follow the live quiet-hours window without a reload',async p=>{
  sql("UPDATE users SET quiet_hours_enabled=1,quiet_hours_start_minute=900,quiet_hours_end_minute=1020 WHERE id=?",[DAVID]);
  await visit(p,`/rooms/${ROOM}`);
  const response=await p.evaluate(async room=>{const token=document.querySelector('meta[name=csrf-token]').content;const body=new URLSearchParams({'message[body]':'/play tada','message[client_message_id]':'ws17-browser-sound'});const r=await fetch(`/rooms/${room}/messages`,{method:'POST',headers:{'X-CSRF-Token':token,'Accept':'text/vnd.turbo-stream.html'},body});return {status:r.status,text:await r.text()};},ROOM);
  assert.equal(response.status,200,response.text);await p.reload();
  await p.evaluate(()=>{window.playedSounds=[];window.Audio=class{constructor(url){window.playedSounds.push(url);}play(){return Promise.resolve();}};});
  const sound=p.getByRole('button',{name:'🔊'}).last();await sound.click();await p.waitForTimeout(500);assert.deepEqual(await p.evaluate(()=>window.playedSounds),[]);
  await p.evaluate(()=>document.querySelector('meta[name=quiet-hours]').setAttribute('content','210-270'));await sound.click();await until(async()=>await p.evaluate(()=>window.playedSounds.length)===1);
 });
 await run(file,'switching the theme applies without a reload flash',async p=>{
  await visit(p,'/users/me/profile');assert.equal(await p.locator('html').getAttribute('data-theme'),'system');await p.locator('label').filter({has:p.locator('#user_theme_dark')}).click();await p.locator('form').filter({has:p.locator('#user_theme_dark')}).locator('button[type=submit]').click();await until(()=>stored('theme')==='dark');
  await until(async()=>await p.locator('html').getAttribute('data-theme')==='dark');assert.equal(await p.locator('meta[name=color-scheme]').getAttribute('content'),'dark');
 });
 await run(file,'switching the text size rescales the page',async p=>{
  await visit(p,'/users/me/profile');assert.equal(await p.locator('html').getAttribute('data-text-size'),'default');await p.locator('label').filter({has:p.locator('#user_text_size_larger')}).click();await p.locator('form').filter({has:p.locator('#user_text_size_larger')}).locator('button[type=submit]').click();await until(()=>stored('text_size')==='larger');await until(async()=>await p.locator('html').getAttribute('data-text-size')==='larger');await until(async()=>await p.locator('html').evaluate(e=>getComputedStyle(e).fontSize)==='18px');assert.equal(await p.locator('html').evaluate(e=>getComputedStyle(e).fontSize),'18px');
 });
 await run(file,'button icons follow the manual theme, not the OS',async p=>{
  for(const [theme,os,nav,submit] of [['light','dark','none','invert(1)'],['dark','light','invert(1)','none']]) {
   sql('UPDATE users SET theme=? WHERE id=?',[theme,DAVID]);await p.emulateMedia({colorScheme:os});await visit(p,'/users/me/profile');assert.equal(await p.locator('html').getAttribute('data-theme'),theme);
   // Media emulation can finish before the next style recalculation. Keep exact
   // computed-filter assertions after bounded observation of the final CSS state.
   const navIcon=p.locator('#nav > .flex-item-justify-start a.btn img');
   const submitIcon=p.locator('form').filter({has:p.locator('#user_theme_dark')}).locator('.btn--reversed img');
   await until(async()=>await navIcon.evaluate(e=>getComputedStyle(e).filter)===nav);
   await until(async()=>await submitIcon.evaluate(e=>getComputedStyle(e).filter)===submit);
   assert.equal(await navIcon.evaluate(e=>getComputedStyle(e).filter),nav);assert.equal(await submitIcon.evaluate(e=>getComputedStyle(e).filter),submit);
  }
 });
 await run(file,'the status form works at phone width',async p=>{await p.setViewportSize({width:390,height:844});await visit(p,'/users/me/profile');for(const id of ['user_presence_setting','user_custom_status_text','user_theme_system'])assert.equal(await p.locator('#'+id).isVisible(),true);});
 await run('test/system/meeting_status_test.rb','the profile links to connect without a Google account',async p=>{
  assert.equal(sql('SELECT COUNT(*) FROM google_identities WHERE user_id=?',[DAVID])[0][0],0);
  await visit(p,'/users/me/profile');
  assert.ok(await p.locator('a[href="#google-calendar-title"]').filter({hasText:/^Connect Google Calendar$/}).count()>0);
  assert.equal(await p.locator('#user_meeting_status_enabled').count(),0);
 });
 await run('test/system/out_of_office_test.rb','set OOO until tomorrow, badge and DM notice show for another user, then clear it',async(p,ctx)=>{
  await visit(p,'/users/me/profile');await p.selectOption('#user_ooo_preset','tomorrow');await p.fill('#user_ooo_note','Back soon');await submit(p,'/users/me/status');await until(()=>stored('ooo_until')!==null);await visit(p,`/users/${DAVID}`);assert.match(await p.locator('.user-status-badge').innerText(),/Out of office/);
  await cookie(ctx,'Jason');await visit(p,`/rooms/${DM}`);assert.match(await p.locator('.ooo-notice').innerText(),/David is out of office/);assert.match(await p.locator('.ooo-notice').innerText(),/Back soon/);
  await cookie(ctx);await visit(p,'/users/me/profile');await p.getByRole('button',{name:'Clear out of office',exact:true}).click();await until(()=>stored('ooo_until')===null);await visit(p,`/users/${DAVID}`);assert.equal(await p.locator('.user-status-badge__custom').count(),0);
  await cookie(ctx,'Jason');await visit(p,`/rooms/${DM}`);assert.equal(await p.locator('.ooo-notice').count(),0);
 });
 await run('test/system/service_worker_test.rb','the worker caches static assets and never authenticated responses',async(p,ctx)=>{
  await cookie(ctx,'JZ');await ctx.addCookies([{name:'enable_service_worker',value:'1',url:base}]);await visit(p,`/rooms/${ROOM}`);await p.waitForFunction(()=>navigator.serviceWorker.controller!==null);await visit(p,`/rooms/${ALL_TALK}`);
  const urls=await p.evaluate(async()=>{const urls=[];for(const name of await caches.keys()){for(const request of await(await caches.open(name)).keys())urls.push(new URL(request.url).pathname);}return urls;});
  assert.ok(urls.includes('/offline.html'));assert.ok(urls.some(u=>u.startsWith('/assets/')),JSON.stringify(urls));assert.deepEqual(urls.filter(u=>u!=='/offline.html'&&!u.startsWith('/assets/')),[]);
 });
 await run('test/system/service_worker_test.rb','the offline shell renders with working retry behavior',async p=>{await visit(p,'/offline.html');assert.equal(await p.locator('h1').innerText(),'You’re offline — reconnecting…');assert.match(await p.locator('[role=status]').innerText(),/automatically when your connection returns/);await p.getByRole('button',{name:'Try again now'}).click();await p.waitForLoadState('load');assert.equal(await p.locator('h1').innerText(),'You’re offline — reconnecting…');});
 for(const file of [...new Set(results.map(r=>r.file))])console.log(`${file}: ${results.filter(r=>r.file===file).length} passed; 0 failed`);
 console.log(`WS17 Chromium: ${results.length} passed; 0 failed`);
} finally {await browser.close();}
