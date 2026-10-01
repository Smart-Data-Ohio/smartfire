// Behavior only: the original Rails system assertions, no screenshots or pixels.
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
const require = createRequire(path.resolve('rust/parity/package.json'));
const { chromium } = require('playwright');
// Playwright's assertion library lives in @playwright/test, which this harness
// deliberately does not add. Poll the same selector/text predicates directly.
const [base, labelsFile, database] = process.argv.slice(2);
const labels = JSON.parse(fs.readFileSync(labelsFile, 'utf8'));
const browser = await chromium.launch({executablePath: process.env.CHROMIUM_PATH || '/usr/bin/chromium', headless: true, args: ['--no-sandbox']});
let passed = 0, failed = 0;
const groups = new Map();
const frozen = Date.parse('2026-03-02T16:00:00Z');
async function contains(locator, text, timeout = 2000) {
  await locator.filter({hasText: text}).first().waitFor({state: 'visible', timeout});
}
async function visit(page, url) {
  // Selenium's page load is separate from Capybara's two-second selector wait.
  return page.goto(url, {waitUntil: 'domcontentloaded', timeout: 60000});
}
async function joinRoom(page, room) {
  await visit(page, `${base}/rooms/${room}`);
  // The original SystemTestHelper#join_room waits for all streams (15 seconds).
  await page.waitForFunction(() => {const all = [...document.querySelectorAll('turbo-cable-stream-source')]; return all.length >= 3 && all.every(node => node.hasAttribute('connected'));}, null, {timeout:15000});
  if (await page.locator("[data-pwa-install-target~='dialog']:visible").count()) await page.getByRole('button',{name:'Close',exact:true}).click();
}
async function run(file, name, user, check) {
  const counts = groups.get(file) || [0,0]; groups.set(file, counts);
  const context = await browser.newContext({viewport: {width: 1440, height: 1000}});
  await context.addCookies([{name: 'session_token', value: labels[`session_cookies.${user}`], url: base, httpOnly: true}]);
  await context.addInitScript(ms => {const Real = Date; globalThis.Date = class extends Real {constructor(...args) {super(...(args.length ? args : [ms]));} static now() {return ms;}};}, frozen);
  const page = await context.newPage(); page.setDefaultTimeout(2000);
  try { await check(page); passed++; counts[0]++; console.log(`PASS ${file}: ${name}`); }
  catch (error) { failed++; counts[1]++; console.log(`FAIL ${file}: ${name}: ${`${new URL(page.url()).pathname}: ${String(error.message).split('\n').slice(0,6).join(' | ')}`}`); }
  finally { await context.close(); }
}
try {
  await run('agents_test.rb', 'member opens the agent directory and an agent profile', 'kevin', async page => {
    await joinRoom(page, labels['rooms.designers']);
    await page.getByRole('link', {name: 'Agents', exact: true}).click();
    await contains(page.locator('h1'), 'Agents');
    const row = page.locator('.agent-directory-row').filter({hasText:'Bender Bot'});
    await contains(row, 'Workspace agent, managed by David'); await contains(row, 'Working');
    await row.locator('.txt-large a').filter({hasText:'Bender Bot'}).click();
    await contains(page.locator('h1'), 'Bender Bot');
    for (const text of ['Workspace agent, managed by David', 'OpenAI', 'Does things', 'Working']) await contains(page.locator('body'), text);
  });
  await run('agent_approvals_test.rb', 'owner sees the request in the inbox, approves it, and the agent is notified', 'david', async page => {
    await visit(page,`${base}/activity`);
    await contains(page.locator('#activity-inbox-title'), 'Activity inbox');
    const item = page.locator(`#activity_item_${labels['system.activity_item']}`);
    for (const text of ['Approval request', 'Bender Bot', 'Ship the release', 'Expires in']) await contains(item, text);
    await item.getByRole('button', {name:'Approve',exact:true}).click();
    await item.waitFor({state:'detached',timeout:10000});
    await visit(page,`${base}/activity?status=handled`);
    await contains(page.locator(`#activity_item_${labels['system.activity_item']}`), 'Approved by David',10000);
    const {spawnSync} = await import('node:child_process');
    const result = spawnSync('python3',['-c',`import sqlite3,json,sys\nc=sqlite3.connect(sys.argv[1]);id=int(sys.argv[2]);assert c.execute('SELECT status FROM agent_approvals WHERE id=?',(id,)).fetchone()[0]=='approved'\nrows=c.execute("SELECT metadata FROM agent_events WHERE event_type='approval_decided'").fetchall();assert any(json.loads(r[0]).get('approval_id')==id and json.loads(r[0]).get('status')=='approved' for r in rows)`,database,String(labels['system.approval'])]);
    if (result.status !== 0) throw new Error('approval row or approval_decided ledger assertion failed');
  });
  await run('agent_streaming_test.rb','agent steps render as a collapsible list','david',async page => {
    await joinRoom(page, labels['system.room']);
    const message = page.locator(`#message_${labels['system.message']}`);
    console.log(`Steps DOM: ${await message.count()} target(s); ${await message.locator('details.agent-steps').count()} details; ${await message.textContent({timeout:2000}).catch(() => 'target absent')}`);
    await contains(message.locator('details.agent-steps summary'),'Steps (2)');
    await message.locator('details.agent-steps > summary').click();
    for (const text of ['Run tests','Deploy','Done','All green','Ship it']) await contains(message,text);
  });
  await run('agent_streaming_test.rb','working presence shows in the member panel','david',async page => {
    await joinRoom(page, labels['rooms.watercooler']);
    await contains(page.locator(`#channel-members [data-member-id='${labels['users.bender']}']`),'Running tests…',20000);
  });
  await run('sudo_mode_test.rb','one prompt, then the action continues automatically','david',async page => {
    await visit(page,`${base}/account/edit`);
    const before = await page.locator('#invite_url').inputValue(); if (!before) throw new Error('empty invite URL');
    await page.getByRole('button',{name:'Regenerate join link',exact:true}).click();
    await contains(page.locator('h1'),"Confirm it's you",10000);
    await page.locator('input[name=password]').fill('secret123456'); await page.locator('form:has(input[name=password])').getByRole('button',{name:'Confirm',exact:true}).click();
    await page.locator('#invite_url').waitFor({state:'visible',timeout:10000});
    const after = await page.locator('#invite_url').inputValue(); if (!after || after===before) throw new Error('sudo did not replay the mutation');
  });
  await run('sudo_mode_test.rb','a wrong password keeps the action gated','david',async page => {
    await visit(page,`${base}/account/edit`); const before = await page.locator('#invite_url').inputValue();
    await page.getByRole('button',{name:'Regenerate join link',exact:true}).click(); await contains(page.locator('h1'),"Confirm it's you",10000);
    await page.locator('input[name=password]').fill('wrong');await page.locator('form:has(input[name=password])').getByRole('button',{name:'Confirm',exact:true}).click();
    await contains(page.locator('.flash'),'Confirmation failed',10000);await visit(page,`${base}/account/edit`);
    if (await page.locator('#invite_url').inputValue() !== before) throw new Error('wrong password replayed the mutation');
  });
  for (const [file,[ok,bad]] of groups) console.log(`${file}: ${ok} passed; ${bad} failed`);
  console.log('agent_streaming_test.rb: 2 deferred (live message mutation API; budget inbox page)');
  console.log('agent_work_assignment_test.rb: 1 deferred (work mutation API)');
  console.log(`Agent system behavior: ${passed} passed; ${failed} failed; 3 deferred`);
} finally {await browser.close();}
process.exitCode = failed ? 1 : 0;
