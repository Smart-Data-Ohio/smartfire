// Source: test/system/search_forward_edit_test.rb at d7c7de92.
import assert from 'node:assert/strict';
import {waitForVisibility,waitForVisibleCount} from './behavior-visibility.mjs';
export async function searchForward({author:page,recipient,base,caseName,fixture,openEdit,send}) {
  if(caseName.startsWith('search tolerates')) {
    const search=page.locator('#global-search-input');
    await search.fill('nonsense zebra tuxedo xyzzy');await search.press('Enter');
    await waitForVisibility(page.getByText('No messages match',{exact:false}));
    assert.equal(await page.locator('#global-search-input').inputValue(),'nonsense zebra tuxedo xyzzy');
    assert.equal((await page.goto(base+'/searches?q=NOT')).status(),200);
    await waitForVisibility(page.locator('#message-area'));
    assert.equal((await page.goto(base+'/searches?q=system%20paging')).status(),200);
    await waitForVisibleCount(page.locator('#search-results .message'),40);
    await waitForVisibleCount(page.locator('#search-results .message').filter({hasText:'system paging alpha'}),0);
    await page.getByRole('link',{name:'Load older results',exact:true}).click();
    await waitForVisibility(page.locator('#search-results .message').filter({hasText:'system paging alpha'}));
    await waitForVisibleCount(page.locator('#search-results .message'),42);
    const ids=await page.locator('#search-results .message').evaluateAll(rows=>rows.map(row=>row.dataset.messageId));
    assert.equal(new Set(ids).size,42,'older search results append without duplicate messages');
    assert.equal(await page.locator('#global-search-input').inputValue(),'system paging');
  } else if(caseName==='forwarded Markdown keeps tables and code blocks') {
    const source=page.locator('.message[data-message-id]').filter({has:page.locator('pre code').filter({hasText:'puts :forwarded'})});
    await waitForVisibility(source);const id=await source.getAttribute('data-message-id');
    await source.locator('[data-message-edit-format], [data-reply-target="body"]').first().click({button:'right'});
    await page.getByRole('menuitem',{name:'Forward',exact:true}).click();
    const dialog=page.locator('dialog[open]');await waitForVisibility(dialog);
    await dialog.locator('.message-forward-dialog__destination:not(.message-forward-dialog__destination--thread)').filter({hasText:'Designers'}).click();
    await dialog.getByRole('button',{name:'Forward',exact:true}).click();
    await waitForVisibility(page.locator('[data-message-actions-target="forwardStatus"]').filter({hasText:/Forwarded to 1 destination/}));
    for(const viewer of [page,recipient]) {
      const forwarded=viewer.locator(`.message[data-message-id]:not([data-message-id="${id}"])`).filter({has:viewer.locator('pre code.language-ruby').filter({hasText:'puts :forwarded'})});
      await waitForVisibility(forwarded.locator('.markdown-body table'));
      assert.equal(await forwarded.locator('pre code.language-ruby').textContent(),'puts :forwarded\n');
      await waitForVisibleCount(forwarded,1);
    }
  } else if(caseName==='editing to add a URL renders its card live and the edited marker on load') {
    const message=page.locator(`.message[data-message-id="${fixture.edit_card_id}"]`);
    await waitForVisibility(message.filter({hasText:'nothing linked yet'}));await openEdit(page,message);
    await send(page,'now with https://x.com/jack/status/424242');
    for(const viewer of [page,recipient]) {
      // Original Rails BROADCAST_WAIT=15; no increased delivery threshold.
      await waitForVisibility(viewer.locator(`.message[data-message-id="${fixture.edit_card_id}"] .x-post-card`).filter({hasText:'Loading post'}),{timeout:15000});
    }
    assert.equal((await page.goto(base+'/rooms/654632876')).status(),200);
    await waitForVisibility(message.locator('.x-post-card'));await waitForVisibility(message.locator('.message__edited').filter({hasText:'(edited)'}));
  } else throw new Error(`unimplemented search/forward case ${caseName}`);
}
