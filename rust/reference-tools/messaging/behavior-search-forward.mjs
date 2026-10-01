// Source: test/system/search_forward_edit_test.rb at d7c7de92.
import assert from 'node:assert/strict';
export async function searchForward({author:page,recipient,base,caseName}) {
  if(caseName.startsWith('search tolerates')) {
    const search=page.locator('#global-search-input');
    await search.fill('nonsense zebra tuxedo xyzzy');await search.press('Enter');
    await page.getByText('No messages match',{exact:false}).waitFor();
    assert.equal(await page.locator('#global-search-input').inputValue(),'nonsense zebra tuxedo xyzzy');
    assert.equal((await page.goto(base+'/searches?q=NOT')).status(),200);
    await page.locator('#message-area').waitFor();
    assert.equal((await page.goto(base+'/searches?q=system%20paging')).status(),200);
    await page.waitForFunction(()=>document.querySelectorAll('#search-results .message').length===40);
    assert.equal(await page.locator('#search-results .message').filter({hasText:'system paging alpha'}).count(),0);
    await page.getByRole('link',{name:'Load older results',exact:true}).click();
    await page.locator('#search-results .message').filter({hasText:'system paging alpha'}).waitFor();
    await page.waitForFunction(()=>document.querySelectorAll('#search-results .message').length===42);
    const ids=await page.locator('#search-results .message').evaluateAll(rows=>rows.map(row=>row.dataset.messageId));
    assert.equal(new Set(ids).size,42,'older search results append without duplicate messages');
    assert.equal(await page.locator('#global-search-input').inputValue(),'system paging');
  } else if(caseName==='forwarded Markdown keeps tables and code blocks') {
    const source=page.locator('.message[data-message-id]').filter({has:page.locator('pre code').filter({hasText:'puts :forwarded'})});
    await source.waitFor();const id=await source.getAttribute('data-message-id');
    await source.locator('[data-message-edit-format], [data-reply-target="body"]').first().click({button:'right'});
    await page.getByRole('menuitem',{name:'Forward',exact:true}).click();
    const dialog=page.locator('dialog[open]');await dialog.waitFor();
    await dialog.locator('.message-forward-dialog__destination:not(.message-forward-dialog__destination--thread)').filter({hasText:'Designers'}).click();
    await dialog.getByRole('button',{name:'Forward',exact:true}).click();
    await page.locator('[data-message-actions-target="forwardStatus"]').filter({hasText:/Forwarded to 1 destination/}).waitFor();
    for(const viewer of [page,recipient]) {
      const forwarded=viewer.locator(`.message[data-message-id]:not([data-message-id="${id}"])`).filter({has:viewer.locator('pre code.language-ruby').filter({hasText:'puts :forwarded'})});
      await forwarded.locator('.markdown-body table').waitFor();
      assert.equal(await forwarded.locator('pre code.language-ruby').textContent(),'puts :forwarded\n');
      assert.equal(await forwarded.count(),1);
    }
  } else throw new Error(`unimplemented search/forward case ${caseName}`);
}
