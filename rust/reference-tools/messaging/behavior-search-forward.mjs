// Source: test/system/search_forward_edit_test.rb at d7c7de92.
import assert from 'node:assert/strict';
import {waitForVisibility,waitForVisibleCount,waitForVisibleProperty} from './behavior-visibility.mjs';
export async function searchForward({author:page,recipient,base,caseName,fixture,openEdit,submit}) {
  if(caseName.startsWith('search tolerates')) {
    const search=page.locator('#global-search-input');
    await search.fill('nonsense zebra tuxedo xyzzy');await search.press('Enter');
    await waitForVisibility(page.getByText('No messages match',{exact:false}));
    await waitForVisibleProperty(page.locator('#global-search-input'),'value','nonsense zebra tuxedo xyzzy');
    assert.equal((await page.goto(base+'/searches?q=NOT')).status(),200);
    await waitForVisibility(page.locator('#message-area'));
    assert.equal((await page.goto(base+'/searches?q=system%20paging')).status(),200);
    await waitForVisibleCount(page.locator('#search-results .message'),40);
    await waitForVisibleCount(page.locator('#search-results .message').filter({hasText:'system paging alpha'}),0);
    await page.getByRole('link',{name:'Load older results',exact:true}).click();
    await waitForVisibility(page.locator('#search-results').getByText('system paging alpha',{exact:true}),{timeout:10000});
    await waitForVisibleCount(page.locator('#search-results .message'),42);
    const ids=await page.locator('#search-results .message').evaluateAll(rows=>rows.map(row=>row.dataset.messageId));
    assert.equal(new Set(ids).size,42,'older search results append without duplicate messages');
    assert.equal(await page.locator('#global-search-input').inputValue(),'system paging');
  } else if(caseName==='forwarded Markdown keeps tables and code blocks') {
    const source=page.locator('.message[data-message-id]').filter({has:page.locator('pre code').filter({hasText:'puts :forwarded'})});
    await waitForVisibility(source.locator('pre code').filter({hasText:'puts :forwarded'}),{timeout:10000});const id=await source.getAttribute('data-message-id');
    await source.locator('[data-message-edit-format], [data-reply-target="body"]').first().click({button:'right'});
    await page.getByRole('menuitem',{name:'Forward',exact:true}).click();
    const dialog=page.locator('dialog[open]');await waitForVisibility(dialog);
    await dialog.locator('.message-forward-dialog__destination:not(.message-forward-dialog__destination--thread)').filter({hasText:'Designers'}).click();
    await dialog.getByRole('button',{name:'Forward',exact:true}).click();
    await waitForVisibility(page.locator('[data-message-actions-target="forwardStatus"]').filter({hasText:/Forwarded to 1 destination/}));
    for(const viewer of [page,recipient]) {
      const forwarded=viewer.locator(`.message[data-message-id]:not([data-message-id="${id}"])`).filter({has:viewer.locator('pre code.language-ruby').filter({hasText:'puts :forwarded'})});
      await waitForVisibility(forwarded.locator('.markdown-body table'));
      await waitForVisibleProperty(forwarded.locator('pre code.language-ruby'),'textContent','puts :forwarded\n');
      await waitForVisibleCount(forwarded,1);
    }
  } else if(caseName==='editing to add a URL renders its card live and the edited marker on load') {
    const message=page.locator(`.message[data-message-id="${fixture.edit_card_id}"]`);
    await waitForVisibility(message.getByText('nothing linked yet',{exact:true}),{timeout:10000});await openEdit(page,message,{contextTimeout:10000});
    // Rails :74-78 clicks Send, then immediately asserts Loading post on the
    // author. Waiting for message text first, or for a second viewer's transient
    // card afterward, can miss a loading state the original assertion observes.
    await submit(page,'now with https://x.com/jack/status/424242');
    await waitForVisibility(message.locator('.x-post-card').filter({hasText:'Loading post'}),{timeout:15000});
    assert.equal((await page.goto(base+'/rooms/654632876')).status(),200);
    await waitForVisibility(message.locator('.x-post-card'),{timeout:10000});await waitForVisibility(message.locator('.message__edited').filter({hasText:'(edited)'}));
  } else throw new Error(`unimplemented search/forward case ${caseName}`);
}
