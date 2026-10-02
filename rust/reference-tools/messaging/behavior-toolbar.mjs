// All thirteen message_toolbar_test.rb declarations, pinned d7c7de92.
import assert from 'node:assert/strict';
import {originalMessage,closedMenu} from './behavior-actions.mjs';
export async function toolbar({author:page,recipient,caseName}) {
  const row=originalMessage(page),panel=page.locator('#emoji-picker-panel');
  const search=page.getByLabel('Search emoji and icons',{exact:true});
  const option=name=>panel.getByRole('button',{name,exact:true});
  async function hover() {await row.locator('[data-reply-target="body"]').hover();await row.locator('.message__toolbar').waitFor();}
  async function picker() {await hover();await row.getByRole('button',{name:'Add reaction',exact:true}).click();await panel.waitFor();}
  async function tab(id) {await page.locator(`#emoji-picker-tab-${id}[aria-selected="true"]`).waitFor();}
  async function focused(label) {await page.waitForFunction(label=>document.activeElement?.getAttribute('aria-label')===label,label);}
  async function reacted(content) {
    for(const browser of [page,recipient]) await originalMessage(browser).locator(`.reaction-chip[data-reaction="${content}"] .reaction-chip__count`).filter({hasText:/^1$/}).waitFor();
    await row.locator(`.reaction-chip[data-reaction="${content}"].reaction-chip--active`).waitFor();
  }
  if(caseName.startsWith('the toolbar stays')) {
    assert.equal(await page.locator('.message__toolbar:visible').count(),0);
    await hover();
    for(const name of ['React with thumbs up','Add reaction','Reply to message','Open thread','More message actions']) await row.getByRole('button',{name,exact:true}).waitFor();
    assert.equal(await row.getByRole('button',{name:'More message actions',exact:true}).getAttribute('aria-haspopup'),'menu');
  } else if(caseName.startsWith('quick-react')) {
    await hover();await row.getByRole('button',{name:'React with thumbs up',exact:true}).click();await reacted('👍');
  } else if(caseName.startsWith('reply and thread')) {
    await hover();await row.getByRole('button',{name:'Reply to message',exact:true}).click();
    await page.locator('#composer [data-composer-target="contextLabel"]').filter({hasText:'Replying to JZ'}).waitFor();
    await page.getByRole('button',{name:'Cancel message context',exact:true}).click();await hover();
    await row.getByRole('button',{name:'Open thread',exact:true}).click();await page.locator('#thread-panel [data-thread-panel-target="create"]').waitFor();
  } else if(caseName.startsWith('the more button')) {
    await hover();await row.getByRole('button',{name:'More message actions',exact:true}).click();
    await page.locator('#message-actions-menu:not([hidden])').waitFor();
    assert.equal(await row.getAttribute('data-message-actions-open'),'');
    assert.equal(await row.getByRole('button',{name:'More message actions',exact:true}).getAttribute('aria-expanded'),'true');
    await page.keyboard.press('Escape');await closedMenu(page);
  } else if(caseName.startsWith('keyboard users')) {
    await row.focus();await page.keyboard.press('Tab');await page.keyboard.press('Tab');await focused('React with thumbs up');
    await page.keyboard.press('Enter');await reacted('👍');
  } else if(caseName.startsWith('the emoji picker searches')) {
    await picker();await tab('smileys');await option('Grinning face').waitFor();await focused('Search emoji and icons');
    await search.fill('fire');await option('Fire').click();await panel.waitFor({state:'hidden'});await reacted('🔥');
  } else if(caseName.startsWith('the picker shows category')) {
    await picker();assert.equal(await panel.getByRole('tab').count(),11);
    assert.notEqual(await page.locator('#emoji-picker-tab-recent').getAttribute('aria-selected'),'true');
    await page.locator('#emoji-picker-tab-people').click();await tab('people');await option('Waving hand').waitFor();
    assert.equal(await option('Grinning face').count(),0);await page.locator('#emoji-picker-tab-flags').click();await tab('flags');await option('Chequered flag').waitFor();
  } else if(caseName.startsWith('the picker loads')) {
    const resources=()=>page.evaluate(()=>performance.getEntriesByType('resource').map(entry=>entry.name));
    assert.equal((await resources()).some(url=>url.endsWith('.json')&&url.includes('emoji')),false);
    await page.evaluate(()=>performance.clearResourceTimings());await picker();await option('Grinning face').waitFor();
    assert.equal((await resources()).some(url=>url.endsWith('.json')&&url.includes('emoji')),true);
    await page.keyboard.press('Escape');await picker();await option('Grinning face').waitFor();
    assert.equal((await resources()).filter(url=>url.endsWith('.json')&&url.includes('emoji')).length,1,'emoji data remains lazy and cached');
  } else if(caseName.startsWith('the picker remembers')) {
    await picker();await option('Grinning face').click();await reacted('😀');await picker();
    await page.locator('#emoji-picker-tab-recent').click();await tab('recent');await option('Grinning face').waitFor();
  } else if(caseName.startsWith('the picker Custom')) {
    await picker();await page.locator('#emoji-picker-tab-custom').click();await tab('custom');
    await option('Acme Corp').locator('img').waitFor();await option('Acme Corp').click();await panel.waitFor({state:'hidden'});await reacted(':acme:');
  } else if(caseName.startsWith('the picker reacts with a brand')) {
    await picker();await search.fill('openai');await option('OpenAI').locator('img').waitFor();
    await option('OpenAI').click();await panel.waitFor({state:'hidden'});await reacted(':openai:');
  } else if(caseName.startsWith('picker arrows')) {
    await picker();await option('Grinning face').waitFor();await search.press('ArrowDown');await focused('Grinning face');
    await page.keyboard.press('ArrowRight');await focused('Grinning face with big eyes');await page.keyboard.press('Escape');await panel.waitFor({state:'hidden'});await focused('Add reaction');
    await picker();await option('Grinning face').waitFor();await search.press('ArrowDown');await page.keyboard.press('Enter');await panel.waitFor({state:'hidden'});await reacted('😀');
  } else if(caseName.startsWith('picker tabs move')) {
    await picker();await option('Grinning face').waitFor();await page.locator('#emoji-picker-tab-smileys').click();
    await page.keyboard.press('ArrowRight');await tab('people');await page.waitForFunction(()=>document.activeElement?.id==='emoji-picker-tab-people');await option('Waving hand').waitFor();
    await page.keyboard.press('ArrowLeft');await tab('smileys');await page.waitForFunction(()=>document.activeElement?.id==='emoji-picker-tab-smileys');
  } else throw new Error(`unimplemented toolbar case ${caseName}`);
}
