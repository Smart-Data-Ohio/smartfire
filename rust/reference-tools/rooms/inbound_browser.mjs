// Browser acceptance on the actual pinned Rails and candidate servers, each on its own seed.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
const require = createRequire(new URL('../../parity/package.json', import.meta.url));
const { chromium } = require('playwright');
const vectors = JSON.parse(readFileSync(new URL('../../vectors/campfire_sessions.json', import.meta.url)));
const cookie = vectors.sessions.find(row => row.user_name === 'David').cookie_header;
const [cookieName, ...cookieParts] = cookie.split('=');
const browser = await chromium.launch({ headless: true });
async function acceptance(base) {
  const context = await browser.newContext();
  try {
    await context.addCookies([{ name: cookieName, value: cookieParts.join('='), url: base }]);
    const page = await context.newPage();
    const edit = '/rooms/opens/201306877/edit';
    await page.goto(base + edit);
    const section = page.locator('#inbound-email');
    await section.getByRole('heading', { name: 'Email to room', exact: true }).waitFor();
    assert.match(await section.innerText(), /No email address yet\./);
    const results = [];
    let previous;
    for (const label of ['Create email address', 'Rotate address']) {
      if (label === 'Rotate address') page.once('dialog', dialog => dialog.accept());
      const post = page.waitForResponse(response => response.request().method() === 'POST' && response.url().endsWith('/inbound_email_address'));
      // The pinned controller redirects to the undeclared RoomsController#edit, a 404.
      const redirect = page.waitForResponse(response => response.request().method() === 'GET' && response.url().endsWith('/rooms/201306877/edit'));
      await section.getByRole('button', { name: label, exact: true }).click();
      const response = await post;
      assert.equal(response.status(), 302);
      const destination = await redirect;
      assert.equal(destination.status(), 404);
      await destination.finished();
      await page.waitForLoadState('networkidle');
      await page.goto(base + edit);
      await section.locator('code').waitFor();
      await page.waitForFunction(old => {
        const address = document.querySelector('#inbound-email code')?.textContent;
        return address && address !== old;
      }, previous);
      const address = await section.locator('code').textContent();
      assert.match(address, /^room-[0-9a-f]{32}@mail\.campfire\.test$/);
      assert.notEqual(address, previous);
      await section.getByRole('button', { name: 'Rotate address', exact: true }).waitFor();
      results.push({ postStatus: response.status(), redirectStatus: destination.status(), redirectPath: new URL(destination.url()).pathname,
        notice: (await page.locator('body').innerText()).includes('Room email address rotated.'),
        rotates: true, addressDomain: address.split('@')[1], tokenLength: 32 });
      previous = address;
    }
    await page.reload();
    assert.equal(await section.locator('code').textContent(), previous);
    await page.goto(base + '/rooms/directs/186869642/edit');
    assert.equal(await page.locator('#inbound-email').count(), 0);
    return results;
  } finally { await context.close(); }
}
try {
  const rails = await acceptance(process.argv[2]);
  const rust = await acceptance(process.argv[3]);
  assert.deepEqual(rust, rails);
  console.log('Inbound-email browser acceptance: 2 targets passed; create, confirm rotation, Rails 302-to-404 redirect, flash, reload and direct-room exclusion match');
} finally { await browser.close(); }
