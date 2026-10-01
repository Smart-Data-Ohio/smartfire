// All boosting_messages_test.rb declarations at pinned d7c7de92.
import assert from 'node:assert/strict';
export async function boosts({author,recipient,caseName,viewer,openEdit,send,text}) {
  const page=recipient;  // Kevin is the original test's draft owner.
  const message=browser=>browser.locator('.message[data-message-id="607264868"]');
  async function input(browser,value) {
    await message(browser).getByRole('link',{name:'Add a boost',exact:true}).click();
    await message(browser).locator('input[name="boost[content]"]').fill(value);
  }
  async function submit(browser) {await message(browser).getByRole('button',{name:'Submit',exact:true}).click();}
  async function delivered(browser,value) {await message(browser).locator('.boost[id]').filter({has:browser.locator('[data-boost-delete-target="content"]').filter({hasText:value})}).waitFor();}
  if(caseName==='boosting a message') {
    await input(page,'Good morning');await submit(page);await delivered(page,'Good morning');await delivered(author,'Good morning');
  } else if(caseName==='deleting a boost') {
    const david=await viewer('David'),boost=david.locator('.boost[id]').filter({has:david.locator('[data-boost-delete-target="content"]').filter({hasText:/^Hello$/})});
    await boost.locator('[data-boost-delete-target="content"]').click();await boost.getByRole('button',{name:'Delete this boost',exact:true}).click();
    await boost.waitFor({state:'detached'});
    for(const browser of [page,author]) await browser.locator('.boost[id]').filter({has:browser.locator('[data-boost-delete-target="content"]').filter({hasText:/^Hello$/})}).waitFor({state:'detached'});
  } else if(caseName==='message update preserves the input state') {
    await input(page,'Hey!');await openEdit(author,message(author));await send(author,'Redacted!');await text(page,'Redacted!');
    assert.equal(await message(page).locator('input[name="boost[content]"]').inputValue(),'Hey!');
  } else if(caseName==='boost by another user preserves the input state') {
    await input(page,'Hey!');const david=await viewer('David');await input(david,'Morning');await submit(david);await delivered(david,'Morning');await delivered(page,'Morning');
    assert.equal(await message(page).locator('input[name="boost[content]"]').inputValue(),'Hey!');
  } else throw new Error(`unimplemented boost case ${caseName}`);
}
