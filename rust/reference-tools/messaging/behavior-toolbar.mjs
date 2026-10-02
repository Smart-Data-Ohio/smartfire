// All thirteen message_toolbar_test.rb declarations, pinned d7c7de92.
import assert from 'node:assert/strict';
import {waitForVisibility,waitForVisibleCount,actOnVisible,waitForVisibleAttribute} from './behavior-visibility.mjs';
import {originalMessage,closedMenu,assertMenuOpen} from './behavior-actions.mjs';
import {DELIVERY_WAIT} from './behavior-deadlines.mjs';
export async function toolbar({author:page,recipient,caseName}) {
  const row=originalMessage(page),panel=page.locator('#emoji-picker-panel');
  const search=page.getByLabel('Search emoji and icons',{exact:true});
  const option=name=>panel.getByRole('button',{name,exact:true});
  async function hover() {await waitForVisibility(row.locator('.message__toolbar'),{state:'attached',timeout:2000});await actOnVisible(row.locator('[data-reply-target="body"]'),'hover');await waitForVisibility(row.locator('.message__toolbar'),{timeout:DELIVERY_WAIT});}
  async function picker(waitForPanel=true) {
    await hover();await row.getByRole('button',{name:'Add reaction',exact:true}).click();
    if(waitForPanel) await waitForVisibility(panel,{timeout:DELIVERY_WAIT});
  }
  async function tab(id) {await waitForVisibility(page.locator(`#emoji-picker-tab-${id}[aria-selected="true"]`));}
  async function focused(label) {assert.equal(await page.evaluate(()=>document.activeElement?.getAttribute('aria-label')),label);}
  async function reacted(content) {
    await waitForVisibility(row.locator(`.reaction-chip[data-reaction="${content}"] .reaction-chip__count`).filter({hasText:/^1$/}),{timeout:DELIVERY_WAIT});
    await waitForVisibility(row.locator(`.reaction-chip[data-reaction="${content}"].reaction-chip--active`));
    // The extra peer delivery check cannot lengthen the author's two-second
    // active-state assertion after the original ten-second count wait.
    await waitForVisibility(originalMessage(recipient).locator(`.reaction-chip[data-reaction="${content}"] .reaction-chip__count`).filter({hasText:/^1$/}),{timeout:DELIVERY_WAIT});
  }
  if(caseName.startsWith('the toolbar stays')) {
    await waitForVisibleCount(page.locator('.message__toolbar'),0);
    await hover();
    for(const name of ['React with thumbs up','Add reaction','Reply to message','Open thread','More message actions']) await waitForVisibility(row.getByRole('button',{name,exact:true}));
    assert.equal(await row.getByRole('button',{name:'More message actions',exact:true}).getAttribute('aria-haspopup'),'menu');
  } else if(caseName.startsWith('quick-react')) {
    await hover();await row.getByRole('button',{name:'React with thumbs up',exact:true}).click();await reacted('👍');
  } else if(caseName.startsWith('reply and thread')) {
    await hover();await row.getByRole('button',{name:'Reply to message',exact:true}).click();
    await waitForVisibility(page.locator('#composer [data-composer-target="contextLabel"]').filter({hasText:'Replying to JZ'}),{timeout:DELIVERY_WAIT});
    await page.getByRole('button',{name:'Cancel message context',exact:true}).click();await hover();
    await row.getByRole('button',{name:'Open thread',exact:true}).click();await waitForVisibility(page.locator('#thread-panel [data-thread-panel-target="create"]'),{timeout:DELIVERY_WAIT});
  } else if(caseName.startsWith('the more button')) {
    await hover();await row.getByRole('button',{name:'More message actions',exact:true}).click();
    await assertMenuOpen(page);
    await waitForVisibleAttribute(row,'data-message-actions-open','');
    // message_toolbar_test.rb:49 uses visible: false after opening the menu.
    await waitForVisibility(row.locator('button[aria-label="More message actions"][aria-expanded="true"]'),{state:'attached',timeout:2000});
    await page.keyboard.press('Escape');await closedMenu(page);
  } else if(caseName.startsWith('keyboard users')) {
    await row.focus();await page.keyboard.press('Tab');await page.keyboard.press('Tab');await focused('React with thumbs up');
    await page.keyboard.press('Enter');await reacted('👍');
  } else if(caseName.startsWith('the emoji picker searches')) {
    await picker();await tab('smileys');await waitForVisibility(option('Grinning face'),{timeout:DELIVERY_WAIT});await focused('Search emoji and icons');
    await search.fill('fire');await waitForVisibility(option('Fire'),{timeout:DELIVERY_WAIT});await option('Fire').click();await waitForVisibility(panel,{state:'hidden'});await reacted('🔥');
  } else if(caseName.startsWith('the picker shows category')) {
    await picker();await waitForVisibleCount(panel.getByRole('tab'),11);
    assert.notEqual(await page.locator('#emoji-picker-tab-recent').getAttribute('aria-selected'),'true');
    await page.locator('#emoji-picker-tab-people').click();await tab('people');await waitForVisibility(option('Waving hand'));
    await waitForVisibleCount(option('Grinning face'),0);await actOnVisible(page.locator('#emoji-picker-tab-flags'),'click');await tab('flags');await waitForVisibility(option('Chequered flag'),{timeout:DELIVERY_WAIT});
  } else if(caseName.startsWith('the picker loads')) {
    const resources=()=>page.evaluate(()=>performance.getEntriesByType('resource').map(entry=>entry.name));
    assert.equal((await resources()).some(url=>url.endsWith('.json')&&url.includes('emoji')),false);
    await page.evaluate(()=>performance.clearResourceTimings());await picker(false);await waitForVisibility(option('Grinning face'),{timeout:DELIVERY_WAIT});
    assert.equal((await resources()).some(url=>url.endsWith('.json')&&url.includes('emoji')),true);
    await page.keyboard.press('Escape');await picker(false);await waitForVisibility(option('Grinning face'),{timeout:DELIVERY_WAIT});
    assert.equal((await resources()).filter(url=>url.endsWith('.json')&&url.includes('emoji')).length,1,'emoji data remains lazy and cached');
  } else if(caseName.startsWith('the picker remembers')) {
    await picker(false);await waitForVisibility(option('Grinning face'),{timeout:DELIVERY_WAIT});await option('Grinning face').click();await reacted('😀');await picker();
    await page.locator('#emoji-picker-tab-recent').click();await tab('recent');await waitForVisibility(option('Grinning face'));
  } else if(caseName.startsWith('the picker Custom')) {
    await picker();await page.locator('#emoji-picker-tab-custom').click();await tab('custom');
    await waitForVisibility(option('Acme Corp').locator('img'),{timeout:DELIVERY_WAIT});await option('Acme Corp').click();await waitForVisibility(panel,{state:'hidden'});await reacted(':acme:');
  } else if(caseName.startsWith('the picker reacts with a brand')) {
    await picker();await search.fill('openai');await waitForVisibility(option('OpenAI').locator('img'),{timeout:DELIVERY_WAIT});
    await option('OpenAI').click();await waitForVisibility(panel,{state:'hidden'});await reacted(':openai:');
  } else if(caseName.startsWith('picker arrows')) {
    await picker();await waitForVisibility(option('Grinning face'),{timeout:DELIVERY_WAIT});await actOnVisible(search,'press',{},['ArrowDown']);await focused('Grinning face');
    await page.keyboard.press('ArrowRight');await focused('Grinning face with big eyes');await page.keyboard.press('Escape');await waitForVisibility(panel,{state:'hidden'});await focused('Add reaction');
    await picker(false);await waitForVisibility(option('Grinning face'),{timeout:DELIVERY_WAIT});await actOnVisible(search,'press',{},['ArrowDown']);await page.keyboard.press('Enter');await waitForVisibility(panel,{state:'hidden'});await reacted('😀');
  } else if(caseName.startsWith('picker tabs move')) {
    await picker(false);await waitForVisibility(option('Grinning face'),{timeout:DELIVERY_WAIT});await page.locator('#emoji-picker-tab-smileys').click();
    await page.keyboard.press('ArrowRight');await tab('people');assert.equal(await page.evaluate(()=>document.activeElement?.id),'emoji-picker-tab-people');await waitForVisibility(option('Waving hand'));
    await page.keyboard.press('ArrowLeft');await tab('smileys');assert.equal(await page.evaluate(()=>document.activeElement?.id),'emoji-picker-tab-smileys');
  } else throw new Error(`unimplemented toolbar case ${caseName}`);
}
