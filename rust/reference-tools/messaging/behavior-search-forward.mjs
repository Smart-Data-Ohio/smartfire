// Source: test/system/search_forward_edit_test.rb at d7c7de92.
import assert from 'node:assert/strict';
import {waitForVisibility,waitForVisibleCount,waitForVisibleProperty,waitForVisibleText,actOnVisible,filterVisibleText,byVisibleText} from './behavior-visibility.mjs';
export async function searchForward({author:page,recipient,base,caseName,fixture,openEdit,submit}) {
  if(caseName.startsWith('search tolerates')) {
    const search=page.locator('#global-search-input');
    await actOnVisible(search,'fill',{},['nonsense zebra tuxedo xyzzy']);await actOnVisible(search,'press',{},['Enter']);
    await waitForVisibility(byVisibleText(page,'No messages match',{exact:false}),{timeout:10000});
    await waitForVisibleProperty(page.locator('#global-search-input'),'value','nonsense zebra tuxedo xyzzy');
    assert.equal((await page.goto(base+'/searches?q=NOT')).status(),200);
    await waitForVisibility(page.locator('#message-area'),{timeout:10000});
    assert.equal((await page.goto(base+'/searches?q=system%20paging')).status(),200);
    await waitForVisibleCount(page.locator('#search-results .message'),40,{timeout:10000});
    await waitForVisibleCount(filterVisibleText(page.locator('#search-results .message'),'system paging alpha'),0);
    await actOnVisible(page.getByRole('link',{name:'Load older results',exact:true}),'click',{});
    await waitForVisibility(byVisibleText(page.locator('#search-results'),'system paging alpha',{exact:true}),{timeout:10000});
    await waitForVisibleCount(page.locator('#search-results .message'),42);
    const ids=await page.locator('#search-results .message').evaluateAll(rows=>rows.map(row=>row.dataset.messageId));
    assert.equal(new Set(ids).size,42,'older search results append without duplicate messages');
    await waitForVisibleProperty(page.locator('#global-search-input'),'value','system paging');
  } else if(caseName==='forwarded Markdown keeps tables and code blocks') {
    const source=page.locator('.message[data-message-id]').filter({has:filterVisibleText(page.locator('pre code'),'puts :forwarded')});
    await waitForVisibility(filterVisibleText(source.locator('pre code'),'puts :forwarded'),{timeout:10000});
    await actOnVisible(source.locator('[data-message-edit-format], [data-reply-target="body"]').first(),'click',{button:'right'});
    await actOnVisible(page.getByRole('menuitem',{name:'Forward',exact:true}),'click',{});
    const dialog=page.locator('dialog[open]');await waitForVisibility(dialog,{timeout:10000});
    await actOnVisible(filterVisibleText(dialog.locator('.message-forward-dialog__destination:not(.message-forward-dialog__destination--thread)'),'Designers'),'click',{timeout:10000});
    const forwardedResponses=[];
    page.on('response',response=>{if(response.request().method()==='POST'&&/\/forwards(?:\.json)?$/.test(new URL(response.url()).pathname)) forwardedResponses.push(response);});
    await actOnVisible(dialog.getByRole('button',{name:'Forward',exact:true}),'click',{});
    await waitForVisibility(filterVisibleText(page.locator('[data-message-actions-target="forwardStatus"]'),/Forwarded to 1 destination/),{timeout:10000});
    // The original status assertion already observes the completed request.
    // Read its identity without inserting another wait before that assertion.
    assert.equal(forwardedResponses.length,1);const response=forwardedResponses[0];assert.equal(response.status(),201);
    const payload=await response.json(),forwardedId=payload.forwards[0].message.id;
    for(const viewer of [page,recipient]) {
      // Rails :57-60 scopes to the persisted copy's identity. Its code's
      // visibility must not decide whether the preceding table lookup succeeds.
      const forwarded=viewer.locator(`.message[data-message-id="${forwardedId}"]`);
      await waitForVisibility(forwarded.locator('.markdown-body table'),{timeout:10000});
      await waitForVisibleText(forwarded.locator('pre code.language-ruby'),'puts :forwarded');
      await waitForVisibleCount(forwarded,1);
    }
  } else if(caseName==='editing to add a URL renders its card live and the edited marker on load') {
    const message=page.locator(`.message[data-message-id="${fixture.edit_card_id}"]`);
    await waitForVisibility(byVisibleText(message,'nothing linked yet',{exact:true}),{timeout:10000});await openEdit(page,message,{contextTimeout:10000});
    // Rails :74-78 clicks Send, then immediately asserts Loading post on the
    // author. Waiting for message text first, or for a second viewer's transient
    // card afterward, can miss a loading state the original assertion observes.
    await submit(page,'now with https://x.com/jack/status/424242');
    await waitForVisibility(filterVisibleText(message.locator('.x-post-card'),'Loading post'),{timeout:15000});
    assert.equal((await page.goto(base+'/rooms/654632876')).status(),200);
    await waitForVisibility(message.locator('.x-post-card'),{timeout:10000});
    await waitForVisibility(filterVisibleText(message.locator('.message__edited'),'(edited)'));
  } else throw new Error(`unimplemented search/forward case ${caseName}`);
}
