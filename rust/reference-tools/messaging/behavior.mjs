// Observable behaviour from the pinned system cases, through real browser controls.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {execFileSync} from 'node:child_process';
import {messageList} from './behavior-message-list.mjs';
import {searchForward} from './behavior-search-forward.mjs';
import {installMutation} from './behavior-mutations.mjs';
import {unreadDivider} from './behavior-unread.mjs';
import {destinationCases,messageDestinations} from './behavior-message-destinations.mjs';
import {composer} from './behavior-composer.mjs';
import {attachMenu} from './behavior-attach-menu.mjs';
import {boosts} from './behavior-boosts.mjs';
const require=createRequire(new URL('../../parity/package.json',import.meta.url));
const {chromium}=require('playwright');
const sessions=JSON.parse(readFileSync(new URL('../../vectors/campfire_sessions.json',import.meta.url))).sessions;
const [rails,rust,file,caseNames,fixtureJson='{}']=process.argv.slice(2);
const cases=JSON.parse(caseNames);
const fixture=JSON.parse(fixtureJson);
assert.ok(['sending_messages','workspace_markdown','threads','message_list_a11y','search_forward_edit','unread_divider','composer','composer_attach_menu','boosting_messages'].includes(file));
const browser=await chromium.launch({headless:true});
const negative=process.env.WS8BM_NEGATIVE==='1';
const keepGoing=process.env.WS8BM_KEEP_GOING==='1';
async function acceptance(base,caseName,probe={}) {
  const contexts=[],threadResponses=[];
  try {
    async function viewer(name) {
      const height=file==='unread_divider'&&caseName.startsWith('many unread')?700:1000;
      const context=await browser.newContext({viewport:{width:1440,height}});
      contexts.push(context);
      const [cookie,...value]=sessions.find(s=>s.user_name===name).cookie_header.split('=');
      await context.addCookies([{name:cookie,value:value.join('='),url:base}]);
      const page=await context.newPage();
      if(file==='threads') page.on('response',async response=>{
        const url=new URL(response.url());
        if(!/^\/rooms\/654632876\/threads(?:\.json)?(?:\/|$)/.test(url.pathname)) return;
        const entry={method:response.request().method(),path:url.pathname,status:response.status()};
        threadResponses.push(entry);
        if((response.headers()['content-type']||'').includes('json')) {
          try {const body=await response.json();entry.threadName=(body.thread||body).name;entry.error=body.error||body.message;} catch {}
        }
      });
      if(negative) await installMutation(page,caseName,probe);
      page.on('pageerror',error=>console.error('WS8bm browser JavaScript:',base,error.stack));
      page.on('requestfailed',request=>{
        const failure=request.failure()?.errorText;
        // Navigation cancels background fetches; diagnose actual network failures.
        if(failure!=='net::ERR_ABORTED') {
          console.error('WS8bm browser failed request:',request.url(),failure);
          if(negative) (probe.networkFailures??=[]).push(failure);
        }
      });
      const response=await page.goto(base+'/rooms/654632876');
      assert.equal(response.status(),200);
      assert.equal(new URL(page.url()).pathname,'/rooms/654632876');
      assert.equal(await page.locator('#composer').count(),1,'real room composer is present');
      try {await page.waitForFunction(()=>window.Stimulus?.getControllerForElementAndIdentifier(document.getElementById('composer'),'composer'));}
      catch(error) {console.error('WS8bm browser startup:',await page.evaluate(()=>({url:location.href,title:document.title,stimulus:!!window.Stimulus,controllers:document.getElementById('composer')?.dataset.controller,scripts:[...document.scripts].map(s=>s.src||s.type)})));throw error;}
      await page.locator('turbo-cable-stream-source[channel="RoomMessagesChannel"][connected]').waitFor({state:'attached'});
      return page;
    }
    const profileActors=file==='message_list_a11y'&&caseName==='profile message and ban buttons have accessible names';
    const author=await viewer('JZ'),recipient=await viewer(profileActors?'David':'Kevin');
    // Startup errors are never accepted as proof of assertion discrimination.
    probe.ready=true;
    if(negative) {
      // Shorter failure-only waits for deliberate mutants; acceptance keeps
      // the original 30-second waits and unchanged concurrency/thresholds.
      author.setDefaultTimeout(3000);recipient.setDefaultTimeout(3000);
    }
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
    if(file==='message_list_a11y') {
      if(destinationCases.includes(caseName)) await messageDestinations({author,recipient,base,caseName,fixture,viewer});
      else await messageList({author,recipient,caseName,send,text,openEdit,field});
      return;
    }
    if(file==='search_forward_edit') {
      await searchForward({author,recipient,base,caseName,fixture,openEdit,send});
      return;
    }
    if(file==='unread_divider') {
      await unreadDivider({author,base,caseName,fixture});
      return;
    }
    if(file==='composer') {
      await composer({author,recipient,base,caseName,fixture,viewer,send,text,field});
      return;
    }
    if(file==='composer_attach_menu') {
      await attachMenu({author,recipient,caseName});
      return;
    }
    if(file==='boosting_messages') {
      await boosts({author,recipient,caseName,viewer,openEdit,send,text});
      return;
    }
    if (file==='threads') {
      const panel=author.locator('#thread-panel');
      async function create(name,first,parent=true,page=author) {
        const panel=page.locator('#thread-panel');
        if(parent) {
          const root=page.locator('.message[data-message-id="607264868"]');
          await page.locator('.message[data-message-id="607264868"][aria-haspopup="menu"]').waitFor();
          await root.locator('[data-message-edit-format], [data-reply-target="body"]').first().click({button:'right'});
          await page.getByRole('menuitem',{name:'Create thread',exact:true}).click();
        } else {
          await page.locator('[data-thread-panel-target="browserToggle"]:visible').click();
          // Match the pinned open_threads helper's actual open-state assertion.
          await panel.locator(':scope[aria-hidden="false"]').waitFor();
          await panel.getByRole('button',{name:'New thread',exact:true}).click();
        }
        await panel.locator('[data-thread-panel-target="create"]').waitFor();
        // beginCreate hands focus to First message on the next animation
        // frame. Wait for that observable open-state transition before
        // typing the name, so insertText cannot land in the other field.
        await page.waitForFunction(()=>document.activeElement===document.querySelector('[data-thread-panel-target="createMessage"]'));
        const nameField=panel.locator('[data-thread-panel-target="createName"]');
        assert.match(await nameField.evaluate(input=>input.closest('label')?.textContent||''),/Thread name/);
        await nameField.fill(name);
        const firstField=panel.locator('[data-thread-panel-target="createMessage"]');
        assert.match(await firstField.evaluate(input=>input.closest('label')?.textContent||''),/First message/);
        await firstField.fill(first);
      }
      async function finishCreate(name,page=author) {
        const panel=page.locator('#thread-panel');
        assert.equal(await panel.locator('[data-thread-panel-target="createName"]').inputValue(),name,`${base}: ${caseName}: the completed name must survive until submission`);
        await panel.locator('[data-thread-panel-target="createSubmit"]').click();
        try {await panel.locator('[data-thread-panel-target="conversationTitle"]').filter({hasText:name}).waitFor();}
        catch(error) {
          console.error('WS8bm thread create diagnostics:',base,caseName,
            await page.evaluate(()=>Object.fromEntries(['create','conversation','conversationTitle','threadStatus','createStatus','createName','createMessage'].map(target=>{
              const node=document.querySelector(`[data-thread-panel-target="${target}"]`);
              return [target,node?{hidden:node.hidden,text:node.value??node.textContent.trim()}:null];
            }))),threadResponses);
          throw error;
        }
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
      async function close(page) {
        await page.getByRole('button',{name:'Close threads',exact:true}).click();
        await page.waitForFunction(()=>!document.body.classList.contains('thread-panel-open'));
      }
      if(caseName==='rejects an external thread deep link before fetching it') {
        const external='https://attacker.invalid/rooms/1/threads/999';
        const requests=[];
        author.on('request',request=>requests.push(request.url()));
        const response=await author.goto(base+'/rooms/654632876?thread='+encodeURIComponent(external));
        assert.equal(response.status(),200);
        await author.locator('#thread-panel[aria-hidden="false"]').waitFor();
        await panel.locator('[data-thread-panel-target="threadStatus"]').filter({hasText:'This thread link is invalid.'}).waitFor();
        assert.equal(await panel.locator('.thread-panel__thread-content .message').count(),0);
        assert.equal(requests.some(url=>url.startsWith('https://attacker.invalid/')),false);
        assert.equal(await author.evaluate(()=>performance.getEntriesByType('resource').some(entry=>entry.name.startsWith('https://attacker.invalid/'))),false);
      } else if(caseName==='renders untrusted thread metadata as text') {
        const malicious='<img src=x onerror="window.__threadXss = true">';
        await create(malicious,'A safe thread body.',false);await finishCreate(malicious);
        const title=panel.locator('[data-thread-panel-target="conversationTitle"]');
        assert.equal((await title.textContent()).trim(),malicious);
        assert.equal(await title.locator('img').count(),0);
        assert.equal(await author.evaluate(()=>window.__threadXss),undefined);
        await threadMessage('A safe thread body.').waitFor();
      } else if(caseName==='browses active and closed threads and can join or leave a closed one') {
        await create('Active planning thread','The active planning conversation.',false);await finishCreate('Active planning thread');await close(author);
        const david=await viewer('David');
        await create('Closed planning thread','The closed planning conversation.',false,david);await finishCreate('Closed planning thread',david);
        const other=david.locator('#thread-panel');
        await other.locator('[data-thread-panel-target="manage"] summary').click();
        await other.locator('[data-thread-panel-target="closeThread"]').click();
        await other.locator('[data-thread-panel-target="threadStatus"]').filter({hasText:/Closed thread/}).waitFor();
        await close(david);
        await author.locator('[data-thread-panel-target="browserToggle"]:visible').click();
        await panel.locator('[data-thread-panel-target="browser"]').waitFor();
        const items=panel.locator('[data-thread-panel-target="browserList"] .thread-panel__thread-item');
        await items.filter({hasText:'Active planning thread'}).waitFor();
        assert.equal(await items.filter({hasText:'Closed planning thread'}).count(),0);
        await panel.locator('[data-thread-panel-target="filter"]').selectOption('closed');
        await items.filter({hasText:'Closed planning thread'}).click();
        await panel.locator('[data-thread-panel-target="conversation"]').waitFor();
        await panel.locator('[data-thread-panel-target="join"]').waitFor();
        assert.equal(await panel.locator('[data-thread-panel-target="leave"]').isVisible(),false);
        await panel.getByRole('button',{name:'Join',exact:true}).click();
        await panel.locator('[data-thread-panel-target="leave"]').waitFor();
        assert.equal(await panel.locator('[data-thread-panel-target="join"]').isVisible(),false);
        await panel.getByRole('button',{name:'Leave',exact:true}).click();
        await panel.locator('[data-thread-panel-target="join"]').waitFor();
        assert.equal(await panel.locator('[data-thread-panel-target="leave"]').isVisible(),false);
      } else if(caseName==='a stray create re-entry does not wipe the half-filled thread name') {
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
      } else if(caseName==='Markdown replies and file attachments remain usable') {
        const uploadResponses=[];
        author.on('response',response=>{
          if(response.request().method()==='POST'&&new URL(response.url()).pathname==='/rooms/654632876/messages') uploadResponses.push({status:response.status(),type:response.headers()['content-type']});
        });
        const source='**A useful point** with `inline code`.';
        await submit(author,source);
        const parent=messages(author).filter({has:author.locator('strong').filter({hasText:'A useful point'})});
        await parent.waitFor();
        await parent.locator('[data-message-edit-format], [data-reply-target="body"]').first().click({button:'right'});
        await author.getByRole('menuitem',{name:'Reply',exact:true}).click();
        await author.locator('#composer [data-composer-target="contextLabel"]').filter({hasText:'Replying to JZ'}).waitFor();
        await author.locator('#composer [data-composer-target="contextPreview"]').filter({hasText:'A useful point'}).waitFor();
        await field(author,'');await author.getByLabel('Notify author',{exact:true}).uncheck();
        await author.locator('#composer input[type="file"]').setInputFiles({name:'markdown-workspace-attachment.txt',mimeType:'text/plain',buffer:Buffer.from('An attachment sent from the Markdown composer.\n')});
        await author.locator('#composer').filter({hasText:'markdown-workspace-attachment'}).waitFor();
        await author.getByRole('button',{name:'Send Message',exact:true}).click();
        for(const page of [author,recipient]) {
          const attachment=messages(page).filter({has:page.locator('.message__reply-preview').filter({hasText:'A useful point'})});
          try {await attachment.waitFor();}
          catch(error) {
            console.error('WS8bm attachment preview diagnostics:',base,uploadResponses,
              await page.locator('.message[data-message-id]').evaluateAll(rows=>rows.map(row=>({id:row.dataset.messageId,preview:row.querySelector('.message__reply-preview')?.textContent,body:row.querySelector('[data-reply-target="body"]')?.textContent}))),
              await author.locator('#composer').evaluate(node=>({busy:node.getAttribute('aria-busy'),reply:node.querySelector('[data-composer-target="replyTo"]')?.value,feedback:node.querySelector('[data-composer-target="feedback"]')?.textContent})));
            throw error;
          }
          try {await attachment.getByRole('link',{name:'Download markdown-workspace-attachment.txt',exact:true}).waitFor();}
          catch(error) {console.error('WS8bm attachment delivery:',base,await attachment.textContent());throw error;}
        }
        await author.locator('#composer [data-composer-target="context"][hidden]').waitFor({state:'attached'});
      } else if(caseName==='mention suggestions select a room member without sending the unfinished message') {
        const editor=author.getByRole('combobox',{name:'Write a message',exact:true});
        const before=await messages(author).count();
        await editor.fill('@Kev');await author.locator('suggestion-option').filter({hasText:'Kevin'}).waitFor();
        await editor.press('Enter');await field(author,'@[Kevin] ');
        assert.equal(await messages(author).count(),before);
        await submit(author,'@[Kevin] please review **the layout**.');
        for(const page of [author,recipient]) {
          const message=messages(page).filter({has:page.locator('strong').filter({hasText:'the layout'})});
          await message.locator('.mention').filter({hasText:'Kevin'}).waitFor();
          assert.equal(await message.locator('.mention').getAttribute('data-user-id'),String(sessions.find(session=>session.user_name==='Kevin').user_id));
          if(page===recipient) await message.locator(':scope.message--mentioned').waitFor();
        }
      } else if(caseName==='a rejected message can be recovered corrected and sent') {
        // SOURCE_LIMIT is read from the pin, rather than the candidate's input.
        const model=execFileSync('git',['show','d7c7de92:app/models/message/markdown.rb'],{encoding:'utf8'});
        const limit=Number(model.match(/SOURCE_LIMIT = ([\d_]+)/)[1].replaceAll('_',''));
        const invalid='A'.repeat(limit+1);
        await author.locator('#composer textarea').evaluate((editor,value)=>{
          editor.removeAttribute('maxlength');editor.value=value;editor.dispatchEvent(new Event('input',{bubbles:true}));
        },invalid);
        await author.getByRole('button',{name:'Send Message',exact:true}).click();
        await author.locator('.message--failed').waitFor();
        // Clearing the still-present failed input proves Restore draft reads
        // the saved submission, rather than passing on an unchanged editor.
        await author.getByRole('combobox',{name:'Write a message',exact:true}).fill('');
        await author.getByRole('button',{name:'Restore draft',exact:true}).click();await field(author,invalid);
        await submit(author,'**Recovered** after correcting the draft.');
        for(const page of [author,recipient]) await messages(page).locator('strong').filter({hasText:'Recovered'}).waitFor();
      } else if(caseName==='sending preserves the submitted source and a newer draft') {
        const first='**First message** stays exact.',second='A newer draft is still here.';
        await author.getByRole('combobox',{name:'Write a message',exact:true}).fill(first);
        await author.evaluate(second=>{
          document.querySelector('#composer button[name="send"]').click();
          const editor=document.querySelector('#composer textarea[name="message[markdown_source]"]');
          editor.value=second;editor.dispatchEvent(new InputEvent('input',{bubbles:true,inputType:'insertFromPaste'}));
        },second);
        for(const page of [author,recipient]) await messages(page).locator('strong').filter({hasText:'First message'}).waitFor();
        await field(author,second);
        assert.equal(await messages(recipient).filter({hasText:second}).count(),0);
        await author.getByRole('button',{name:'Send Message',exact:true}).click();
        for(const page of [author,recipient]) await text(page,second);
        await field(author,'');
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
  } catch(error) {
    console.error('WS8bm failed application:',base,caseName);
    throw error;
  } finally {for(const context of contexts) await context.close();}
}
try {
  let failures=0;
  for(const caseName of cases) {
    try {
    if(negative) {
      const probe={ready:false,applied:0};let failure;
      try {await acceptance(rust,caseName,probe);} catch(error) {failure=error;}
      if(!probe.ready || !probe.applied || !failure) console.error('WS8bm invalid discrimination run:',caseName,probe,failure);
      assert.ok(probe.ready,'mutant must reach the actual named case, not fail startup');
      assert.equal(probe.networkFailures?.length||0,0,'network failures cannot count as mutant rejection');
      assert.ok(probe.applied>0,'a deliberate served mutation must actually apply');
      assert.ok(failure,'named behaviour check must reject the served mutant');
      assert.ok(failure.code==='ERR_ASSERTION'||failure.name==='TimeoutError',`unexpected infrastructure/adapter failure: ${failure}`);
      console.log(`WS8bm discrimination: ${file}: ${caseName}: served mutant REJECTED (${failure.code||failure.name})`);
    } else {
      await acceptance(rails,caseName);await acceptance(rust,caseName);
      console.log(`WS8bm browser flow: ${file}: ${caseName}: Rails PASS; Rust PASS`);
    }
    } catch(error) {
      console.error(`WS8bm browser flow FAILED: ${file}: ${caseName}:`,error.stack);
      failures++;
      if(!keepGoing) throw error;
    }
  }
  if(failures) process.exitCode=1;
}
finally {await browser.close();}
