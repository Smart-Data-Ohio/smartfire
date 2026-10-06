// Original WS12 browser assertions run unchanged on production Rails and Rust UI.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {chromium} from 'playwright';
import {startProxy} from '../../parity/capture/proxy.ts';
const target=process.env.WS12_BROWSER_TARGET, key=process.env.WS12_BROWSER_CASE;
const labels=JSON.parse(fs.readFileSync(process.env.WS12_BROWSER_LABELS,'utf8'));
const proxy=await startProxy(target), base='http://127.0.0.1';
const browser=await chromium.launch({headless:true,args:['--no-sandbox']});
const contexts=[];
async function session(name='david'){
 const context=await browser.newContext({proxy:{server:proxy.server,bypass:'<-loopback>'},timezoneId:'UTC',locale:'en-US',viewport:{width:1400,height:1000}});
 contexts.push(context);await context.addCookies([{name:'session_token',value:labels[`session_cookies.${name}`],url:base}]);
 await context.route('**/*',r=>new URL(r.request().url()).origin===base?r.continue():r.abort());
 const page=await context.newPage();page.setDefaultTimeout(15000);return page;
}
async function text(p,selector,pattern,label){assert.match(await p.locator(selector).textContent(),pattern,`${key}: ${label}`);}
async function absent(p,selector,label){assert.equal(await p.locator(selector).count(),0,`${key}: ${label}`);}
const path='/rooms/699448332';
try{
 const p=await session(key==='c222'?'jz':key==='c227'?'kevin':'david');
 await p.goto(base+path);
 if(key==='c221'){
  await p.getByRole('link',{name:'Automations',exact:true}).click();
  await p.locator('#automations-title').waitFor();
  await text(p,'#automations-title',/Automations for Launch/,'automations title');
  await p.getByLabel('Tag',{exact:true}).fill('bug');
  await p.locator('[name="assignee_id"]').selectOption({label:'JZ'});
  await p.getByRole('button',{name:'Add rule',exact:true}).click();
  await text(p,'ul[aria-labelledby="tag-rules-title"]',/bug/,'saved tag rule');
  await p.locator('[name="sla_rules[planned][nudge_after_minutes]"]').fill('60');
  await p.locator('[name="sla_rules[planned][escalate_after_minutes]"]').fill('240');
  await p.locator('[name="sla_rules[blocked][nudge_after_minutes]"]').fill('30');
  await p.locator('[name="sla_rules[blocked][escalate_after_minutes]"]').fill('120');
  await p.getByRole('button',{name:'Save SLA timers',exact:true}).click();
  await p.locator('.flash').filter({hasText:'SLA timers saved.'}).waitFor();
  await p.reload();
  assert.equal(await p.locator('[name="sla_rules[planned][nudge_after_minutes]"]').inputValue(),'60',`${key}: persisted planned nudge`);
  assert.equal(await p.locator('[name="sla_rules[blocked][escalate_after_minutes]"]').inputValue(),'120',`${key}: persisted blocked escalation`);
 }else if(key==='c222'){
  await absent(p,'a[href="/rooms/boards/699448332/automations"]','plain member has no settings link');
  await p.goto(base+'/rooms/boards/699448332/automations');
  await absent(p,'#automations-title','plain member denied direct settings');
 }else if(key==='c223'){
  await p.getByRole('link',{name:'New post',exact:true}).click();
  await p.getByLabel('Title',{exact:true}).fill('Broken login');
  await p.locator('[name="thread[tags]"]').fill('bug');
  await p.getByRole('button',{name:'Create post',exact:true}).click();
  await p.getByRole('heading',{name:'Broken login',exact:true}).waitFor();
  await text(p,'.board-post__facts',/JZ/,'tag assignee shown');
  await text(p,'.board-post__history',/Auto-assigned by board tag rule/,'tag assignment history');
 }else if(key==='c224'){
  await p.goto(base+path+'/threads/4');
  await p.locator('.board-post__work summary').filter({hasText:'Update work'}).click();
  await p.getByRole('link',{name:'Hand off to an agent',exact:true}).click();
  await p.locator('[name="receiver_agent_id"]').selectOption({label:'Board Agent'});
  await p.getByLabel('Summary',{exact:true}).fill('Halfway there, tests are green');
  await p.getByLabel('Links (one URL per line, up to 10)',{exact:true}).fill('https://example.com/spec');
  await p.getByLabel('Open questions (one per line, up to 10)',{exact:true}).fill('Which API ships first?');
  await p.getByRole('button',{name:'Hand off',exact:true}).click();
  await p.locator('.board-post__facts').filter({hasText:'Board Agent'}).waitFor();
  await text(p,'.board-post__facts',/Board Agent/,'handoff receiving owner');
  await text(p,'.board-post__history',/David handed off/,'handoff actor');
  await text(p,'.board-post__history',/Halfway there, tests are green/,'handoff context');
 }else if(key==='c225'){
  await text(p,'.board__digest',/Stale-work digest/,'digest heading');
  await text(p,'.board__digest',/Stuck migration/,'stale post in digest');
  await text(p,'.board__digest',/In progress/,'stale status in digest');
 }else if(key==='c226'){
  await p.locator("section[aria-labelledby='boards-heading'] a[aria-label='New board']").click();
  await p.locator('[name="room[name]"]').fill('Launch');
  await p.locator("li[data-value='board agent'] label.switch").click();
  await p.locator('form:has(input[name="room[name]"]) button[type="submit"]').click();
  await p.locator('.board__header h1').filter({hasText:'Launch'}).waitFor();
  await text(p,'#board_rooms',/Launch/,'created board sidebar');
  const boardPath=new URL(p.url()).pathname;
  await p.getByRole('link',{name:'New post',exact:true}).click();
  await p.getByLabel('Title',{exact:true}).fill('Ship the launch');
  await p.locator('[name="thread[first_message]"]').fill('Everything must go out on Friday.');
  await p.locator('[name="thread[work_owner_id]"]').selectOption({label:'Board Agent'});
  await p.locator('[name="thread[tags]"]').fill('launch, api');
  await p.getByRole('button',{name:'Create post',exact:true}).click();
  await p.getByRole('heading',{name:'Ship the launch',exact:true}).waitFor();
  await text(p,'.board-post__facts',/Board Agent/,'created agent owner');
  assert.equal((await p.locator('.board-post__facts .agent-badge').textContent()).trim(),'agent',`${key}: agent badge`);
  assert.deepEqual(await p.locator('.board-tag').allTextContents(),['api','launch'],`${key}: post tags`);
  await text(p,'.board-post__messages',/Everything must go out on Friday./,'opening brief');
  const id=new URL(p.url()).pathname.split('/').at(-1);
  await p.getByText('Edit result',{exact:true}).click();
  await p.getByLabel('Result in Markdown',{exact:true}).fill('## Shipped on Friday');
  await p.getByRole('button',{name:'Save result',exact:true}).click();
  await p.locator('.board-post__result-body').filter({hasText:'Shipped on Friday'}).waitFor();
  await text(p,'.board-post__history',/David updated the result/,'result audit');
  const viewer=await session();await viewer.goto(base+boardPath);
  await viewer.waitForFunction(()=>[...document.querySelectorAll('turbo-cable-stream-source')].some(e=>e.hasAttribute('connected')));
  await text(viewer,`#board_row_channel_thread_${id} .board-row__status`,/Planned/,'initial live row');
  await p.getByText('Update work',{exact:true}).click();
  await p.locator('[name="thread[work_status]"]').selectOption('in_progress');
  await p.getByRole('button',{name:'Save status',exact:true}).click();
  await p.locator('.board-post__facts').filter({hasText:'In progress'}).waitFor();
  await viewer.locator(`#board_row_channel_thread_${id} .board-row__status`).filter({hasText:'In progress'}).waitFor();
  await text(viewer,`#board_row_channel_thread_${id} .board-row__status`,/In progress/,'live status row');
  await viewer.getByRole('link',{name:'Board',exact:true}).click();
  await text(viewer,'.board__column[aria-label="In progress"]',/Ship the launch/,'updated status column');
  await absent(viewer,'.board__column[aria-label="Planned"] .board-row','empty planned column');
 }else if(key==='c227'){
  await absent(p,'.board','stranger denied board');await absent(p,'.board-post','stranger denied post');
  assert.notEqual(new URL(p.url()).pathname,path,`${key}: forbidden board destination`);
 }else throw Error('unknown browser declaration '+key);
 console.log(`WS12_BROWSER_NAMED ${key}: passed; real forms, saved reloads and Cable; 0 masks`);
}finally{for(const c of contexts)await c.close();await browser.close();await proxy.close();}
