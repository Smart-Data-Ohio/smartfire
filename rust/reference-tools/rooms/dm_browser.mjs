// Interact with real Stimulus controllers and submit real CSRF-protected forms.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
const require = createRequire(new URL('../../parity/package.json', import.meta.url));
const { chromium } = require('playwright');
const sessions = JSON.parse(readFileSync(new URL('../../vectors/campfire_sessions.json', import.meta.url))).sessions;
const browser = await chromium.launch({ headless: true });
const inject = process.argv.includes('--inject-selection-drift');
async function acceptance(base, broken = false) {
  const context = await browser.newContext();
  try {
    const cookie = sessions.find(row => row.user_name === 'David').cookie_header;
    const [name, ...value] = cookie.split('=');
    await context.addCookies([{ name, value: value.join('='), url: base }]);
    const page = await context.newPage();
    async function picker() {
      await page.goto(base + '/rooms/directs/new');
      await page.locator('#dm_picker_filter').waitFor();
      await page.waitForFunction(() => document.activeElement?.id === 'dm_picker_filter');
      return page.locator('#direct_rooms_control');
    }
    let control = await picker();
    const rows = control.locator('.dm-picker__row');
    assert.ok(await rows.count() > 2);
    const input = control.locator('#dm_picker_filter');
    await input.fill('No Such Person');
    await control.getByText('No one matches that search.', { exact: true }).waitFor();
    assert.equal(await control.locator('.dm-picker__row:not([hidden])').count(), 0);
    await input.fill('kEvIn');
    assert.equal(await control.locator('.dm-picker__row:not([hidden])').count(), 1);
    await input.press('Enter');
    await control.getByRole('button', { name: 'Message (1)', exact: true }).waitFor();
    await input.fill('Jason');
    await input.press('Enter');
    assert.equal(await control.locator('input:checked').count(), 1);
    await input.fill('');
    assert.equal(await control.locator('input[aria-label="Select Kevin"]').isChecked(), true);
    await control.getByRole('button', { name: 'Clear', exact: true }).click();
    await page.waitForFunction(() => document.querySelector('[data-multi-select-target="bar"]').hidden);
    assert.equal(await control.locator('input:checked').count(), 0);
    assert.equal(await control.locator('[data-multi-select-target="bar"]').getAttribute('hidden'), '');
    await control.locator('input[aria-label="Select Kevin"]').check();
    await control.locator('input[aria-label="Select Jason"]').check();
    await control.getByRole('button', { name: 'Message (2)', exact: true }).waitFor();
    assert.equal(await control.getByRole('button', { name: 'Start huddle (2)', exact: true }).isEnabled(), true);
    if (broken) {
      // A valid, wrong hidden user ID must fail the persisted-membership check below.
      await control.locator('[data-multi-select-target="inputs"] input[value="712064548"]').evaluate(el => { el.value = '773523953'; });
    }
    const post = page.waitForResponse(r => r.request().method() === 'POST' && new URL(r.url()).pathname === '/rooms/directs');
    await control.getByRole('button', { name: 'Message (2)', exact: true }).click();
    const response = await post;
    assert.equal(response.status(), 302);
    const location = new URL(response.headers().location, base);
    assert.match(location.pathname, /^\/rooms\/\d+$/);
    await page.waitForURL(location.href);
    await page.waitForLoadState('networkidle');
    const roomId = location.pathname.split('/').at(-1);
    const settings = `/rooms/directs/${roomId}/edit`;
    await page.goto(base + settings);
    assert.deepEqual((await page.locator('.directs--edit .member strong').allTextContents()).sort(),
      ['Jason', 'Kevin'], 'persisted DM member names');
    await page.locator('input[name="room[name]"]').fill('Browser Weekend <team>');
    const rename = page.waitForResponse(r => r.request().method() === 'POST' && new URL(r.url()).pathname === `/rooms/directs/${roomId}`);
    await page.getByRole('button', { name: 'Save', exact: true }).click();
    assert.equal((await rename).status(), 302);
    await page.waitForLoadState('networkidle');
    await page.reload();
    assert.equal(await page.locator('input[name="room[name]"]').inputValue(), 'Browser Weekend <team>');
    await page.locator('input[name="room[name]"]').evaluate(el => el.removeAttribute('maxlength'));
    await page.locator('input[name="room[name]"]').fill('x'.repeat(101));
    const invalid = page.waitForResponse(r => r.request().method() === 'POST' && new URL(r.url()).pathname === `/rooms/directs/${roomId}`);
    await page.getByRole('button', { name: 'Save', exact: true }).click();
    assert.equal((await invalid).status(), 422);
    await page.locator('.field_with_errors input[name="room[name]"]').waitFor();
    assert.equal(await page.locator('input[name="room[name]"]').inputValue(), 'x'.repeat(101));
    await page.reload();
    assert.equal(await page.locator('input[name="room[name]"]').inputValue(), 'Browser Weekend <team>');
    control = await picker();
    await control.locator('input[aria-label="Select Kevin"]').check();
    await control.locator('input[aria-label="Select Jason"]').check();
    const reuse = page.waitForResponse(r => r.request().method() === 'POST' && new URL(r.url()).pathname === '/rooms/directs');
    await control.getByRole('button', { name: 'Start huddle (2)', exact: true }).click();
    const reused = await reuse;
    assert.equal(reused.status(), 302);
    assert.equal(new URL(reused.headers().location, base).pathname, location.pathname);
    assert.equal(new URL(reused.headers().location, base).search, '?huddle=start');
    await page.waitForLoadState('networkidle');
    await picker();
    await page.locator('#dm_picker_filter').press('Escape');
    await page.waitForFunction(() => !document.querySelector('#dm_picker_filter'));
    assert.equal(await page.locator('#dm_picker_filter').count(), 0);
    return { create: 302, rename: 302, invalidRename: 422, memberNames: ['Jason', 'Kevin'], reuse: true, huddle: '?huddle=start', escape: true };
  } finally { await context.close(); }
}
try {
  const rails = await acceptance(process.argv[2]);
  if (inject) {
    await assert.rejects(() => acceptance(process.argv[3], true), /persisted DM member names/);
    console.log('DM browser discrimination: wrong submitted member ID rejected by persisted settings acceptance');
  } else {
    assert.deepEqual(await acceptance(process.argv[3]), rails);
    console.log('DM browser acceptance: 2 targets passed; filter, Enter, hidden selection, clear, real create, rename, invalid name, reload, reuse, huddle redirect and Escape match');
  }
} finally { await browser.close(); }
