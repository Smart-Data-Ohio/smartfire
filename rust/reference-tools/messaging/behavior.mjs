// Observable behaviour from the pinned system cases, through real browser controls.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {execFileSync} from 'node:child_process';
const require=createRequire(new URL('../../parity/package.json',import.meta.url));
const {chromium}=require('playwright');
const sessions=JSON.parse(readFileSync(new URL('../../vectors/campfire_sessions.json',import.meta.url))).sessions;
const [rails,rust,file,caseName]=process.argv.slice(2);
assert.ok(['sending_messages','workspace_markdown','threads'].includes(file));
const browser=await chromium.launch({headless:true});
async function acceptance(base) {
  const contexts=[];
  try {
    async function viewer(name) {
      const context=await browser.newContext({viewport:{width:1440,height:1000}});
      contexts.push(context);
      const [cookie,...value]=sessions.find(s=>s.user_name===name).cookie_header.split('=');
      await context.addCookies([{name:cookie,value:value.join('='),url:base}]);
      const page=await context.newPage();
      page.on('pageerror',error=>console.error('WS8bm browser JavaScript:',error.message));
      const response=await page.goto(base+'/rooms/654632876');
      assert.equal(response.status(),200);
      assert.equal(new URL(page.url()).pathname,'/rooms/654632876');
      assert.equal(await page.locator('#composer').count(),1,'real room composer is present');
      try {await page.waitForFunction(()=>window.Stimulus?.getControllerForElementAndIdentifier(document.getElementById('composer'),'composer'));}
      catch(error) {console.error('WS8bm browser startup:',await page.evaluate(()=>({url:location.href,title:document.title,stimulus:!!window.Stimulus,controllers:document.getElementById('composer')?.dataset.controller,scripts:[...document.scripts].map(s=>s.src||s.type)})));throw error;}
      await page.locator('turbo-cable-stream-source[channel="RoomMessagesChannel"][connected]').waitFor({state:'attached'});
      return page;
    }
    const author=await viewer('JZ'),recipient=await viewer('Kevin');
    const messages=page=>page.locator('.message[data-message-id]');
    async function text(page,value,count=1) {
      await page.waitForFunction(({value,count})=>[...document.querySelectorAll('.message[data-message-id] [data-reply-target="body"]')].filter(body=>body.textContent.trim()===value).length===count,{value,count});
    }
    async function send(page,value) {
      await page.getByRole('combobox',{name:'Write a message',exact:true}).fill(value);
      await page.getByRole('button',{name:'Send Message',exact:true}).click();
      await text(page,value);
    }
    async function submit(page,value) {
      await page.getByRole('combobox',{name:'Write a message',exact:true}).fill(value);
      await page.getByRole('button',{name:'Send Message',exact:true}).click();
    }
    async function openEdit(page,message) {
      await message.click({button:'right'});
      await page.locator('#message-actions-menu:not([hidden])').waitFor();
      await page.getByRole('menuitem',{name:'Edit message',exact:true}).click();
      await page.locator('#composer').filter({hasText:'Editing Message'}).waitFor();
    }
    async function field(page,value) {
      await page.waitForFunction(value=>document.querySelector('#composer textarea[name="message[markdown_source]"]')?.value===value,value);
    }
    if (file==='threads') {
      const panel=author.locator('#thread-panel');
      async function create(name,first,parent=true) {
        if(parent) {
          await author.locator('.message[data-message-id="607264868"]').click({button:'right'});
          await author.getByRole('menuitem',{name:'Create thread',exact:true}).click();
        } else {
          await author.locator('[data-thread-panel-target="browserToggle"]:visible').click();
          await panel.getByRole('button',{name:'New thread',exact:true}).click();
        }
        await panel.locator('[data-thread-panel-target="create"]').waitFor();
        const nameField=panel.locator('[data-thread-panel-target="createName"]');
        assert.match(await nameField.evaluate(input=>input.closest('label')?.textContent||''),/Thread name/);
        await nameField.fill(name);
        const firstField=panel.locator('[data-thread-panel-target="createMessage"]');
        assert.match(await firstField.evaluate(input=>input.closest('label')?.textContent||''),/First message/);
        await firstField.fill(first);
      }
      async function finishCreate(name) {
        await panel.locator('[data-thread-panel-target="createSubmit"]').click();
        await panel.locator('[data-thread-panel-target="conversationTitle"]').filter({hasText:name}).waitFor();
        await panel.locator('turbo-cable-stream-source[channel="RoomMessagesChannel"][connected]').waitFor({state:'attached'});
      }
      const threadMessage=value=>panel.locator('.message[data-message-id]').filter({has:author.locator('[data-reply-target="body"]').filter({hasText:value})});
      async function reply(value) {
        await panel.getByRole('combobox',{name:'Write a thread reply',exact:true}).fill(value);
        await panel.getByRole('button',{name:'Send Reply',exact:true}).click();
        await threadMessage(value).waitFor();
      }
      async function threadMenu(value) {
        // The pinned within_thread_message helper waits for the menu controller
        // to register each newly delivered/replaced row before right-clicking.
        const message=threadMessage(value);
        const id=await message.getAttribute('id');
        await author.locator(`[id="${id}"][aria-haspopup="menu"]`).waitFor();
        await message.locator('[data-message-edit-format], [data-reply-target="body"]').first().click({button:'right'});
        await author.locator('#message-actions-menu:not([hidden])').waitFor();
      }
      if(caseName==='a stray create re-entry does not wipe the half-filled thread name') {
        await create('Survives a stray reset','The name survives the re-entry.',false);
        await author.evaluate(()=>window.dispatchEvent(new CustomEvent('message:thread',{detail:{}})));
        await finishCreate('Survives a stray reset');
        await threadMessage('The name survives the re-entry.').waitFor();
      } else if(caseName==='the thread root counts its replies live and hides the count when none remain') {
        await create('Indicator thread','The only reply.');await finishCreate('Indicator thread');
        const root=author.locator('.message[data-message-id="607264868"]');
        const indicatorId='thread_indicator_'+await root.getAttribute('id');
        for(const page of [author,recipient]) {
          await page.locator(`[id="${indicatorId}"][aria-label="Open thread, 1 reply"]`).waitFor();
          assert.equal((await page.locator(`[id="${indicatorId}"]`).textContent()).trim(),'1 reply');
          await page.locator(`[id="${indicatorId}"] img.colorize--black`).waitFor();
        }
        await reply('A second reply.');
        for(const page of [author,recipient]) await page.locator(`[id="${indicatorId}"][aria-label="Open thread, 2 replies"]`).waitFor();
        for(const value of ['A second reply.','The only reply.']) {
          await threadMenu(value);author.once('dialog',dialog=>dialog.accept());
          await author.getByRole('menuitem',{name:'Delete message',exact:true}).click();
          await threadMessage(value).waitFor({state:'detached'});
        }
        for(const page of [author,recipient]) await page.locator(`[id="${indicatorId}"][hidden]`).waitFor({state:'attached'});
      } else if(caseName==='creates a thread from a channel message and keeps the channel draft separate') {
        const first='Let’s keep the design review focused here.';
        await create('Design review thread',first);await finishCreate('Design review thread');
        await panel.locator('[data-thread-panel-target="parent"]').filter({hasText:"Third time's a charm."}).waitFor();
        await threadMessage(first).waitFor();
        await panel.locator('[data-thread-panel-target="preferences"] summary').click();
        await panel.locator('[data-thread-panel-target="involvement"]').selectOption('nothing');
        await panel.locator('[data-thread-panel-target="manage"] summary').click();
        await panel.locator('[data-thread-panel-target="autoArchive"]').selectOption('1440');
        await author.getByRole('combobox',{name:'Write a message',exact:true}).fill('A channel draft stays here.');
        await reply('A reply from the thread drawer.');await field(author,'A channel draft stays here.');
        await threadMenu('A reply from the thread drawer.');
        await author.getByRole('menuitem',{name:'Reply',exact:true}).click();
        await panel.locator('[data-composer-target="contextLabel"]').filter({hasText:'Replying to'}).waitFor();
        await reply('A reply to the drawer message.');
        await panel.locator('.message__reply-preview').filter({hasText:'A reply from the thread drawer.'}).waitFor();
        await threadMenu(first);await author.getByRole('menuitem',{name:'Edit message',exact:true}).click();
        await panel.locator('[data-composer-target="contextLabel"]').filter({hasText:'Editing Message'}).waitFor();
        await reply('The edited thread starter.');
        await panel.locator('[data-thread-panel-target="parent"]').filter({hasText:"Third time's a charm."}).waitFor();
        await threadMenu('A reply to the drawer message.');
        await author.locator('.message__quick-reaction[title="Thumbs up"]').click();
        await panel.locator('.boosts__reactions').filter({hasText:'👍'}).waitFor();
        await field(author,'A channel draft stays here.');
      } else {throw new Error(`unimplemented case ${caseName}`);}
      return;
    }
    if (file==='workspace_markdown') {
      const source=execFileSync('git',['show','d7c7de92:test/system/workspace_markdown_test.rb'],{encoding:'utf8'});
      const heredoc=(start,indent)=>source.split(start)[1].split(`${' '.repeat(indent)}MARKDOWN`)[0].split('\n').map(line=>line.slice(indent+2)).join('\n');
      if (caseName==='Markdown messages reach other users and editing preserves the original source') {
        const markdown=heredoc("MARKDOWN = <<~'MARKDOWN'.freeze\n",2);
        await submit(author,markdown);
        async function content(page,id) {
          const message=page.locator(`.message[data-message-id="${id}"]`);
          for(const [selector,value] of [['strong','Ready for review'],['em','clear ownership'],['del','old assumptions'],['blockquote','Keep the conversation close to the work.'],['ul li','Check the channel layout'],['table td','Markdown']]) {
            await message.locator(selector).filter({hasText:value}).waitFor();
          }
          assert.equal(await message.locator('input[type="checkbox"][disabled]').count(),2);
          await message.locator('pre code.language-javascript[data-highlighted="yes"] .code-token').filter({hasText:'const'}).waitFor();
          assert.equal(await message.locator('.markdown-code-copy').count(),1);
          assert.equal(await message.getByRole('link',{name:'Project notes',exact:true}).getAttribute('href'),'https://example.com/notes');
          assert.ok((await message.locator('pre code').textContent()).includes('const message = "<script>literal code</script>";'));
        }
        const message=messages(author).filter({has:author.locator('h2').filter({hasText:/^Design review$/})});
        await message.waitFor();
        const id=await message.getAttribute('data-message-id');
        await content(author,id);await content(recipient,id);
        await openEdit(author,message);await field(author,markdown);
        const edited=markdown.replace('Design review','Review complete');
        await submit(author,edited);
        for(const page of [author,recipient]) {
          await page.locator(`.message[data-message-id="${id}"] h2`).filter({hasText:/^Review complete$/}).waitFor();
          await content(page,id);
        }
        await author.reload();
        await author.waitForFunction(()=>window.Stimulus?.getControllerForElementAndIdentifier(document.getElementById('composer'),'composer'));
        await openEdit(author,author.locator(`.message[data-message-id="${id}"]`));
        await field(author,edited);
      } else if (caseName==='desktop keyboard composition keeps line breaks and sends once after composition ends') {
        const editor=author.getByRole('combobox',{name:'Write a message',exact:true});
        await editor.fill('First line');await editor.press('Shift+Enter');await editor.pressSequentially('Second line');
        await field(author,'First line\nSecond line');
        const before=await messages(author).count();
        await editor.evaluate(editor=>editor.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',code:'Enter',keyCode:13,isComposing:true,bubbles:true,cancelable:true})));
        await field(author,'First line\nSecond line');assert.equal(await messages(author).count(),before);
        await editor.press('Enter');
        for(const page of [author,recipient]) await text(page,'First line\nSecond line');
        await field(author,'');
        await editor.press('ArrowUp');
        await author.locator('#composer').filter({hasText:'Editing Message'}).waitFor();
        await field(author,'First line\nSecond line');
      } else if (caseName==='untrusted markup stays inert in the delivered message') {
        const payload=heredoc("payload = <<~'MARKDOWN'\n",4);
        await submit(author,payload);
        for(const page of [author,recipient]) {
          const message=messages(page).filter({has:page.locator('p').filter({hasText:/^Safety check$/})});
          await message.waitFor();
          await message.locator('pre code').filter({hasText:'<img onerror="literal code">'}).waitFor();
          assert.equal(await message.locator('script, img[onerror], a[href^="javascript:"]').count(),0);
          assert.equal(await page.evaluate(()=>window.markdownPayloadExecuted===true),false);
        }
      } else {throw new Error(`unimplemented case ${caseName}`);}
      return;
    }
    if (caseName==='sending messages between two users') {
      await send(author,'Is this thing on?');
      await text(recipient,'Is this thing on?');
      await send(recipient,'👍👍');
      await text(author,'👍👍');
    } else {
      const original=author.locator('.message[data-message-id="607264868"]');
      await text(recipient,"Third time's a charm.");
      await original.click({button:'right'});
      await author.locator('#message-actions-menu:not([hidden])').waitFor();
      if (caseName==='editing messages') {
        await author.getByRole('menuitem',{name:'Edit message',exact:true}).click();
        await author.locator('#composer').filter({hasText:'Editing Message'}).waitFor();
        await send(author,'Redacted!');
        await text(recipient,'Redacted!');
        await text(recipient,"Third time's a charm.",0);
        await recipient.reload();
        await text(recipient,'Redacted!');
      } else if (caseName==='deleting messages') {
        author.once('dialog',dialog=>dialog.accept());
        await author.getByRole('menuitem',{name:'Delete message',exact:true}).click();
        await text(recipient,"Third time's a charm.",0);
        assert.equal(await messages(author).filter({hasText:"Third time's a charm."}).count(),0);
      } else {throw new Error(`unimplemented case ${caseName}`);}
    }
  } finally {for(const context of contexts) await context.close();}
}
try {await acceptance(rails);await acceptance(rust);}
finally {await browser.close();}
