// All boosting_messages_test.rb declarations at pinned d7c7de92.
import {waitForVisibility,waitForVisibleProperty,actOnVisible,filterVisibleText} from './behavior-visibility.mjs';
export async function boosts({author,recipient,caseName,viewer,openEdit,send,text}) {
  const page=recipient;  // Kevin is the original test's draft owner.
  const message=browser=>browser.locator('.message[data-message-id="607264868"]');
  async function input(browser,value) {
    await actOnVisible(message(browser).getByRole('link',{name:'Add a boost',exact:true}),'click',{});
    await actOnVisible(message(browser).locator('input[name="boost[content]"]'),'fill',{},[value]);
  }
  async function submit(browser) {await actOnVisible(message(browser).getByRole('button',{name:'Submit',exact:true}),'click',{});}
  async function delivered(browser,value) {await waitForVisibility(message(browser).locator('.boost[id]').filter({has:filterVisibleText(browser.locator('[data-boost-delete-target="content"]'),value)}));}
  if(caseName==='boosting a message') {
    await input(page,'Good morning');await submit(page);await delivered(page,'Good morning');await delivered(author,'Good morning');
  } else if(caseName==='deleting a boost') {
    const david=await viewer('David'),boost=david.locator('.boost[id]').filter({has:filterVisibleText(david.locator('[data-boost-delete-target="content"]'),/^Hello$/)});
    await actOnVisible(boost.locator('[data-boost-delete-target="content"]'),'click',{});await waitForVisibility(filterVisibleText(boost.locator('button'),'Delete this boost'),{timeout:5000});
    await actOnVisible(boost.getByRole('button',{name:'Delete this boost',exact:true}),'click',{});
    await waitForVisibility(boost,{state:'hidden'});
    for(const browser of [page,author]) await waitForVisibility(browser.locator('.boost[id]').filter({has:filterVisibleText(browser.locator('[data-boost-delete-target="content"]'),/^Hello$/)}),{state:'hidden'});
  } else if(caseName==='message update preserves the input state') {
    await input(page,'Hey!');await openEdit(author,message(author));await send(author,'Redacted!');await text(page,'Redacted!');
    await waitForVisibleProperty(message(page).locator('input[name="boost[content]"]'),'value','Hey!');
  } else if(caseName==='boost by another user preserves the input state') {
    await input(page,'Hey!');const david=await viewer('David');await input(david,'Morning');await submit(david);await delivered(david,'Morning');await delivered(page,'Morning');
    await waitForVisibleProperty(message(page).locator('input[name="boost[content]"]'),'value','Hey!');
  } else throw new Error(`unimplemented boost case ${caseName}`);
}
