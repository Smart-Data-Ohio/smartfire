// Review regression: Rails' Board navigation deliberately has no call controls.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { fixture, observeStreams, waitIssuance } from './ws13-support.mjs';

test('Board navigation remains unchanged when its actual Cable presence target is replayed', async t => {
  const f = await fixture(t, ['david'], {kind:'board'});
  const page = f.pages.david;
  const nav = page.locator('#nav .room-header__actions');
  const before = await nav.innerHTML();
  await observeStreams(page);
  const grant = await f.issue('david', f.room, f.people.david.session);
  await f.mutate({op:'seen_presence', grant:grant.id});
  // This observes Turbo consuming the production presence broadcast sent over
  // the real socket, rather than manufacturing a replacement or renaming it.
  await waitIssuance(page, f.room, 'board');
  assert.equal(await nav.innerHTML(), before);
  assert.equal(await nav.locator('[data-controller="huddle-launcher"]').count(), 0);
  assert.equal(await nav.locator('[id^="header_voice_participants_"]').count(), 0);
  assert.equal(await nav.getByRole('link', {name:'New post', exact:true}).count(), 1);
  assert.equal(await page.locator(`#header_rooms_board_${f.room}`).count(), 1);
});
