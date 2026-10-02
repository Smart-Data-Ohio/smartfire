// Observable behaviour from the pinned system cases, through real browser controls.
import assert from 'node:assert/strict';
import {waitForVisibility,isSeleniumVisible,installVisibility,setVisibilityTimeout,waitForVisibleCount,visibleCount,visibleMatch,waitForVisibleProperty,waitForVisibleAttribute,waitForVisibleContentCount} from './behavior-visibility.mjs';
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {execFileSync} from 'node:child_process';
import {messageList} from './behavior-message-list.mjs';
import {searchForward} from './behavior-search-forward.mjs';
import {installMutation,mutationVariants} from './behavior-mutations.mjs';
import {unreadDivider} from './behavior-unread.mjs';
import {destinationCases,messageDestinations} from './behavior-message-destinations.mjs';
import {composer} from './behavior-composer.mjs';
import {attachMenu} from './behavior-attach-menu.mjs';
import {boosts} from './behavior-boosts.mjs';
import {interactions,mobileActions,assertMenuOpen} from './behavior-actions.mjs';
import {toolbar} from './behavior-toolbar.mjs';
import {codeHighlighting} from './behavior-code.mjs';
import {CAPYBARA_DEFAULT,DELIVERY_WAIT,CABLE_WAIT,REVIEW_GROUPS} from './behavior-deadlines.mjs';
const require=createRequire(new URL('../../parity/package.json',import.meta.url));
const {chromium}=require('playwright');
const sessions=JSON.parse(readFileSync(new URL('../../vectors/campfire_sessions.json',import.meta.url))).sessions;
const [rails,rust,file,caseNames,fixtureJson='{}']=process.argv.slice(2);
const cases=JSON.parse(caseNames);
const fixture=JSON.parse(fixtureJson);
assert.ok(['sending_messages','workspace_markdown','threads','message_list_a11y','search_forward_edit','unread_divider','composer','composer_attach_menu','boosting_messages','message_interactions','message_actions_mobile','message_toolbar','code_highlighting'].includes(file));
const browser=await chromium.launch({headless:true});
const negative=process.env.WS8BM_NEGATIVE==='1';
const keepGoing=process.env.WS8BM_KEEP_GOING==='1';
const selectedMutant=process.env.WS8BM_MUTANT;
async function acceptance(base,caseName,probe={},variant='default') {
  const contexts=[],threadResponses=[];
  try {
    async function viewer(name) {
      const height=file==='unread_divider'&&caseName.startsWith('many unread')?700:1000;
      // Mutation routes must remain observable across navigations; a service
      // worker can otherwise fetch/cache the original asset outside page.route.
      // Positive acceptance retains the app's actual service worker.
      const context=await browser.newContext({viewport:{width:1440,height},...(negative||selectedMutant?{serviceWorkers:'block'}:{})});
      contexts.push(context);
      await installVisibility(context);
      const [cookie,...value]=sessions.find(s=>s.user_name===name).cookie_header.split('=');
      await context.addCookies([{name:cookie,value:value.join('='),url:base}]);
      const page=await context.newPage();
      if(REVIEW_GROUPS.has(file)) {
        page.setDefaultTimeout(CAPYBARA_DEFAULT);
        setVisibilityTimeout(page,CAPYBARA_DEFAULT);
        // Capybara visit/page-load is separate from selector assertions.
        page.setDefaultNavigationTimeout(30000);
      }
      if(file==='threads') page.on('response',async response=>{
        const url=new URL(response.url());
        if(!/^\/rooms\/654632876\/threads(?:\.json)?(?:\/|$)/.test(url.pathname)) return;
        const entry={method:response.request().method(),path:url.pathname,status:response.status()};
        threadResponses.push(entry);
        if((response.headers()['content-type']||'').includes('json')) {
          try {const body=await response.json();entry.threadName=(body.thread||body).name;entry.error=body.error||body.message;} catch {}
        }
      });
      if(negative||selectedMutant) await installMutation(page,caseName,probe,variant);
      page.on('pageerror',error=>console.error('WS8bm browser JavaScript:',base,error.stack));
      page.on('requestfailed',request=>{
        const failure=request.failure()?.errorText;
        // Navigation cancels background fetches; diagnose actual network failures.
        if(failure!=='net::ERR_ABORTED') {
          console.error('WS8bm browser failed request:',request.url(),failure);
          if(negative||selectedMutant) (probe.networkFailures??=[]).push(failure);
        }
      });
      const response=await page.goto(base+'/rooms/654632876');
      assert.equal(response.status(),200);
      assert.equal(new URL(page.url()).pathname,'/rooms/654632876');
      assert.equal(await page.locator('#composer').count(),1,'real room composer is present');
      try {await page.waitForFunction(()=>window.Stimulus?.getControllerForElementAndIdentifier(document.getElementById('composer'),'composer'));}
      catch(error) {console.error('WS8bm browser startup:',await page.evaluate(()=>({url:location.href,title:document.title,stimulus:!!window.Stimulus,controllers:document.getElementById('composer')?.dataset.controller,scripts:[...document.scripts].map(s=>s.src||s.type)})));throw error;}
      await waitForVisibility(page.locator('turbo-cable-stream-source[channel="RoomMessagesChannel"][connected]'),{state:'attached',...(REVIEW_GROUPS.has(file)?{timeout:CABLE_WAIT}:{})});
      return page;
    }
    const profileActors=file==='message_list_a11y'&&caseName==='profile message and ban buttons have accessible names';
    const author=await viewer('JZ'),recipient=await viewer(profileActors||file==='message_interactions'?'David':'Kevin');
    // Startup errors are never accepted as proof of assertion discrimination.
    probe.ready=true;
    if(negative&&!REVIEW_GROUPS.has(file)) {
      // Older groups retain their existing failure-only probes. The audited
      // thirty use the original Rails budgets identically in both modes.
      author.setDefaultTimeout(3000);recipient.setDefaultTimeout(3000);
      setVisibilityTimeout(author,3000);setVisibilityTimeout(recipient,3000);
    }
    const messages=page=>page.locator('.message[data-message-id]');
    async function text(page,value,count=1,options={}) {
      await waitForVisibleContentCount(page.locator('.message[data-message-id] .message__body'),'[data-reply-target="body"]',value,count,options);
    }
    async function send(page,value) {
      await page.getByRole('combobox',{name:'Write a message',exact:true}).fill(value);
      await page.getByRole('button',{name:'Send Message',exact:true}).click();
      await text(page,value,1,REVIEW_GROUPS.has(file)?{timeout:DELIVERY_WAIT}:{});
    }
    async function submit(page,value) {
      await page.getByRole('combobox',{name:'Write a message',exact:true}).fill(value);
      await page.getByRole('button',{name:'Send Message',exact:true}).click();
    }
    async function openEdit(page,message,{contextTimeout}={}) {
      await message.click({button:'right'});
      await assertMenuOpen(page);
      await page.getByRole('menuitem',{name:'Edit message',exact:true}).click();
      if(!REVIEW_GROUPS.has(file)) await waitForVisibility(page.locator('#composer').filter({hasText:'Editing Message'}));
      else if(contextTimeout) await waitForVisibility(page.locator('#composer [data-composer-target="contextLabel"]').filter({hasText:'Editing Message'}),{timeout:contextTimeout});
    }
    async function field(page,value,options={}) {
      await waitForVisibleProperty(page.locator('#composer textarea[name="message[markdown_source]"]'),'value',value,options);
    }
    if(file==='message_list_a11y') {
      if(destinationCases.includes(caseName)) await messageDestinations({author,recipient,base,caseName,fixture,viewer});
      else await messageList({author,recipient,caseName,send,text,openEdit,field});
      return;
    }
    if(file==='search_forward_edit') {
      await searchForward({author,recipient,base,caseName,fixture,openEdit,submit});
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
    if(file==='message_interactions') {
      await interactions({author,recipient,caseName,fixture,openEdit,send,text,field});
      return;
    }
    if(file==='message_actions_mobile') {
      await mobileActions({author,caseName});
      return;
    }
    if(file==='message_toolbar') {
      await toolbar({author,recipient,caseName});
      return;
    }
    if(file==='code_highlighting') {
      await codeHighlighting({author,recipient,caseName,fixture,base,openEdit,field});
      return;
    }
    if(file==='threads'&&caseName==='discusses a pull request from its card') {
      const card=()=>author.locator('.github-pr-card').filter({has:author.locator('.github-pr-card__title').filter({hasText:'Fix login'})});
      await card().getByRole('button',{name:'Discuss',exact:true}).click();
      await waitForVisibility(author.locator('.github-pr-thread-header .github-pr-card__title').filter({hasText:'Fix login'}));
      await waitForVisibility(author.locator('.github-pr-files__heading').filter({hasText:'Files changed'}));
      await waitForVisibility(author.locator('.github-pr-files__path').filter({hasText:'app/models/user.rb'}));
      const threadPath=new URL(author.url()).pathname;assert.match(threadPath,/^\/rooms\/654632876\/threads\/\d+$/);
      await author.goto(base+'/rooms/654632876');await card().getByRole('link',{name:'Discuss',exact:true}).click();
      await waitForVisibility(author.locator('.github-pr-thread-header .github-pr-card__title').filter({hasText:'Fix login'}));
      assert.equal(new URL(author.url()).pathname,threadPath,'the second discussion opens the existing thread');
      return;
    }
    if (file==='threads') {
      const panel=author.locator('#thread-panel');
      async function create(name,first,parent=true,page=author) {
        const panel=page.locator('#thread-panel');
        if(parent) {
          const root=page.locator('.message[data-message-id="607264868"]');
          await waitForVisibility(page.locator('.message[data-message-id="607264868"][aria-haspopup="menu"]'));
          await root.locator('[data-message-edit-format], [data-reply-target="body"]').first().click({button:'right'});
          await page.getByRole('menuitem',{name:'Create thread',exact:true}).click();
        } else {
          await (await visibleMatch(page.locator('[data-thread-panel-target="browserToggle"]'))).click();
          // Match the pinned open_threads helper's actual open-state assertion.
          await waitForVisibility(panel.locator(':scope[aria-hidden="false"]'));
          await panel.getByRole('button',{name:'New thread',exact:true}).click();
        }
        await waitForVisibility(panel.locator('[data-thread-panel-target="create"]'));
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
        try {
          await waitForVisibility(panel.locator('[data-thread-panel-target="conversation"]'),{timeout:DELIVERY_WAIT});
          await waitForVisibility(panel.locator('[data-thread-panel-target="conversationTitle"]').filter({hasText:name}),{timeout:DELIVERY_WAIT});
        }
        catch(error) {
          console.error('WS8bm thread create diagnostics:',base,caseName,
            await page.evaluate(()=>Object.fromEntries(['create','conversation','conversationTitle','threadStatus','createStatus','createName','createMessage'].map(target=>{
              const node=document.querySelector(`[data-thread-panel-target="${target}"]`);
              return [target,node?{hidden:node.hidden,text:node.value??node.textContent.trim()}:null];
            }))),threadResponses);
          throw error;
        }
        await waitForVisibility(panel.locator('turbo-cable-stream-source[channel="RoomMessagesChannel"][connected]'),{state:'attached'});
      }
      const threadMessage=value=>panel.locator('.message[data-message-id]').filter({has:author.locator('[data-reply-target="body"]').filter({hasText:value})});
      async function assertThreadMessage(value) {
        await waitForVisibility(panel.locator('.thread-panel__thread-content .message__body').filter({hasText:value}),{timeout:DELIVERY_WAIT});
      }
      async function reply(value) {
        await panel.getByRole('combobox',{name:'Write a thread reply',exact:true}).fill(value);
        await panel.getByRole('button',{name:'Send Reply',exact:true}).click();
        await assertThreadMessage(value);
      }
      async function threadMenu(value) {
        // The pinned within_thread_message helper waits for the menu controller
        // to register each newly delivered/replaced row before right-clicking.
        const message=threadMessage(value);
        // threads_test.rb:581 finds a visible row, but :583 then checks
        // the controller's attribute with visible: false, wait: 10.
        await waitForVisibility(message,{timeout:DELIVERY_WAIT});
        const id=await message.getAttribute('id');
        await waitForVisibility(author.locator(`[id="${id}"][aria-haspopup="menu"]`),{state:'attached',timeout:DELIVERY_WAIT});
        await message.locator('[data-message-edit-format], [data-reply-target="body"]').first().click({button:'right'});
        await waitForVisibility(author.locator('#message-actions-menu:not([hidden])'));
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
        await waitForVisibility(author.locator('#thread-panel[aria-hidden="false"]'));
        await waitForVisibility(panel.locator('[data-thread-panel-target="threadStatus"]').filter({hasText:'This thread link is invalid.'}));
        await waitForVisibleCount(panel.locator('.thread-panel__thread-content .message'),0);
        assert.equal(requests.some(url=>url.startsWith('https://attacker.invalid/')),false);
        assert.equal(await author.evaluate(()=>performance.getEntriesByType('resource').some(entry=>entry.name.startsWith('https://attacker.invalid/'))),false);
      } else if(caseName==='renders untrusted thread metadata as text') {
        const malicious='<img src=x onerror="window.__threadXss = true">';
        await create(malicious,'A safe thread body.',false);await finishCreate(malicious);
        const title=panel.locator('[data-thread-panel-target="conversationTitle"]');
        assert.equal((await title.textContent()).trim(),malicious);
        await waitForVisibleCount(title.locator('img'),0);
        assert.equal(await author.evaluate(()=>window.__threadXss),undefined);
        await assertThreadMessage('A safe thread body.');
      } else if(caseName==='browses active and closed threads and can join or leave a closed one') {
        await create('Active planning thread','The active planning conversation.',false);await finishCreate('Active planning thread');await close(author);
        const david=await viewer('David');
        await create('Closed planning thread','The closed planning conversation.',false,david);await finishCreate('Closed planning thread',david);
        const other=david.locator('#thread-panel');
        await other.locator('[data-thread-panel-target="manage"] summary').click();
        await other.locator('[data-thread-panel-target="closeThread"]').click();
        await waitForVisibility(other.locator('[data-thread-panel-target="threadStatus"]').filter({hasText:/Closed thread/}));
        await close(david);
        await (await visibleMatch(author.locator('[data-thread-panel-target="browserToggle"]'))).click();
        await waitForVisibility(panel.locator('[data-thread-panel-target="browser"]'));
        const items=panel.locator('[data-thread-panel-target="browserList"] .thread-panel__thread-item');
        await waitForVisibility(items.filter({hasText:'Active planning thread'}));
        await waitForVisibleCount(items.filter({hasText:'Closed planning thread'}),0);
        await panel.locator('[data-thread-panel-target="filter"]').selectOption('closed');
        await items.filter({hasText:'Closed planning thread'}).click();
        await waitForVisibility(panel.locator('[data-thread-panel-target="conversation"]'));
        await waitForVisibility(panel.locator('[data-thread-panel-target="join"]'));
        assert.equal(await isSeleniumVisible(panel.locator('[data-thread-panel-target="leave"]')),false);
        await panel.getByRole('button',{name:'Join',exact:true}).click();
        await waitForVisibility(panel.locator('[data-thread-panel-target="leave"]'));
        assert.equal(await isSeleniumVisible(panel.locator('[data-thread-panel-target="join"]')),false);
        await panel.getByRole('button',{name:'Leave',exact:true}).click();
        await waitForVisibility(panel.locator('[data-thread-panel-target="join"]'));
        assert.equal(await isSeleniumVisible(panel.locator('[data-thread-panel-target="leave"]')),false);
      } else if(caseName==='a stray create re-entry does not wipe the half-filled thread name') {
        await create('Survives a stray reset','The name survives the re-entry.',false);
        await author.evaluate(()=>window.dispatchEvent(new CustomEvent('message:thread',{detail:{}})));
        await finishCreate('Survives a stray reset');
        await assertThreadMessage('The name survives the re-entry.');
      } else if(caseName==='the thread root counts its replies live and hides the count when none remain') {
        await create('Indicator thread','The only reply.');await finishCreate('Indicator thread');
        const root=author.locator('.message[data-message-id="607264868"]');
        const indicatorId='thread_indicator_'+await root.getAttribute('id');
        for(const page of [author,recipient]) {
          await waitForVisibility(page.locator(`[id="${indicatorId}"][aria-label="Open thread, 1 reply"]`));
          assert.equal((await page.locator(`[id="${indicatorId}"]`).textContent()).trim(),'1 reply');
          await waitForVisibility(page.locator(`[id="${indicatorId}"] img.colorize--black`));
        }
        await reply('A second reply.');
        for(const page of [author,recipient]) await waitForVisibility(page.locator(`[id="${indicatorId}"][aria-label="Open thread, 2 replies"]`));
        for(const value of ['A second reply.','The only reply.']) {
          await threadMenu(value);author.once('dialog',dialog=>dialog.accept());
          await author.getByRole('menuitem',{name:'Delete message',exact:true}).click();
          await waitForVisibility(threadMessage(value),{state:'detached'});
        }
        for(const page of [author,recipient]) await waitForVisibility(page.locator(`[id="${indicatorId}"][hidden]`),{state:'attached'});
      } else if(caseName==='creates a thread from a channel message and keeps the channel draft separate') {
        const first='Let’s keep the design review focused here.';
        await create('Design review thread',first);await finishCreate('Design review thread');
        await waitForVisibility(panel.locator('[data-thread-panel-target="parent"]').filter({hasText:"Third time's a charm."}));
        await assertThreadMessage(first);
        await panel.locator('[data-thread-panel-target="preferences"] summary').click();
        await panel.locator('[data-thread-panel-target="involvement"]').selectOption('nothing');
        await panel.locator('[data-thread-panel-target="manage"] summary').click();
        await panel.locator('[data-thread-panel-target="autoArchive"]').selectOption('1440');
        await author.getByRole('combobox',{name:'Write a message',exact:true}).fill('A channel draft stays here.');
        await reply('A reply from the thread drawer.');await field(author,'A channel draft stays here.');
        await threadMenu('A reply from the thread drawer.');
        await author.getByRole('menuitem',{name:'Reply',exact:true}).click();
        await waitForVisibility(panel.locator('[data-composer-target="contextLabel"]').filter({hasText:'Replying to'}));
        await reply('A reply to the drawer message.');
        await waitForVisibility(panel.locator('.message__reply-preview').filter({hasText:'A reply from the thread drawer.'}));
        await threadMenu(first);await author.getByRole('menuitem',{name:'Edit message',exact:true}).click();
        await waitForVisibility(panel.locator('[data-composer-target="contextLabel"]').filter({hasText:'Editing Message'}));
        await reply('The edited thread starter.');
        await waitForVisibility(panel.locator('[data-thread-panel-target="parent"]').filter({hasText:"Third time's a charm."}));
        await threadMenu('A reply to the drawer message.');
        await author.locator('.message__quick-reaction[title="Thumbs up"]').click();
        await waitForVisibility(panel.locator('.boosts__reactions').filter({hasText:'👍'}));
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
        async function content(page,id,initial=false) {
          const message=page.locator(`.message[data-message-id="${id}"]`);
          if(initial) await waitForVisibility(message.locator('h2').filter({hasText:/^Design review$/}),{timeout:CAPYBARA_DEFAULT});
          for(const [selector,value] of [['strong','Ready for review'],['em','clear ownership'],['del','old assumptions'],['blockquote','Keep the conversation close to the work.'],['ul li','Check the channel layout'],['table td','Markdown']]) {
            await waitForVisibility(message.locator(selector).filter({hasText:value}));
          }
          await waitForVisibleCount(message.locator('input[type="checkbox"][disabled]'),2);
          const code=message.locator('pre code').filter({hasText:'const message = "<script>literal code</script>";'});
          await waitForVisibility(code,{timeout:CAPYBARA_DEFAULT});
          assert.ok((await code.textContent()).includes('const message = "<script>literal code</script>";'));
          await waitForVisibility(message.locator('pre code.language-javascript[data-highlighted="yes"] .code-token').filter({hasText:'const'}),{timeout:20000});
          await waitForVisibleCount(message.locator('.markdown-code-copy'),1);
          await waitForVisibleAttribute(message.getByRole('link',{name:'Project notes',exact:true}),'href','https://example.com/notes');
        }
        const message=messages(author).filter({has:author.locator('h2').filter({hasText:/^Design review$/})});
        await waitForVisibility(message.locator('h2').filter({hasText:/^Design review$/}),{timeout:CAPYBARA_DEFAULT});
        const id=await message.getAttribute('data-message-id');
        await content(author,id,true);await content(recipient,id,true);
        await openEdit(author,message);await field(author,markdown);
        const edited=markdown.replace('Design review','Review complete');
        await submit(author,edited);
        for(const page of [author,recipient]) {
          await waitForVisibility(page.locator(`.message[data-message-id="${id}"] h2`).filter({hasText:/^Review complete$/}));
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
        const before=await visibleCount(messages(author));
        await editor.evaluate(editor=>editor.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',code:'Enter',keyCode:13,isComposing:true,bubbles:true,cancelable:true})));
        await field(author,'First line\nSecond line');assert.equal(await visibleCount(messages(author)),before);
        await editor.press('Enter');
        for(const page of [author,recipient]) await text(page,'First line\nSecond line');
        await field(author,'');
        await editor.press('ArrowUp');
        await waitForVisibility(author.locator('#composer').filter({hasText:'Editing Message'}));
        await field(author,'First line\nSecond line');
      } else if (caseName==='untrusted markup stays inert in the delivered message') {
        const payload=heredoc("payload = <<~'MARKDOWN'\n",4);
        await submit(author,payload);
        for(const page of [author,recipient]) {
          const message=messages(page).filter({has:page.locator('p').filter({hasText:/^Safety check$/})});
          await waitForVisibility(message.locator('.message__body').filter({hasText:'Safety check'}),{timeout:CAPYBARA_DEFAULT});
          await waitForVisibility(message.locator('pre code').filter({hasText:'<img onerror="literal code">'}));
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
        await waitForVisibility(parent.locator('.message__body').filter({hasText:'A useful point'}),{timeout:CAPYBARA_DEFAULT});
        await parent.locator('[data-message-edit-format], [data-reply-target="body"]').first().click({button:'right'});
        await author.getByRole('menuitem',{name:'Reply',exact:true}).click();
        await waitForVisibility(author.locator('#composer [data-composer-target="contextLabel"]').filter({hasText:'Replying to JZ'}));
        await waitForVisibility(author.locator('#composer [data-composer-target="contextPreview"]').filter({hasText:'A useful point'}));
        await field(author,'');await author.getByLabel('Notify author',{exact:true}).uncheck();
        await author.locator('#composer input[type="file"]').setInputFiles({name:'markdown-workspace-attachment.txt',mimeType:'text/plain',buffer:Buffer.from('An attachment sent from the Markdown composer.\n')});
        await waitForVisibility(author.locator('#composer').filter({hasText:'markdown-workspace-attachment'}));
        await author.getByRole('button',{name:'Send Message',exact:true}).click();
        for(const page of [author,recipient]) {
          const attachment=messages(page).filter({has:page.locator('.message__reply-preview').filter({hasText:'A useful point'})});
          try {await waitForVisibility(attachment.locator('.message__reply-preview').filter({hasText:'A useful point'}),{timeout:DELIVERY_WAIT});}
          catch(error) {
            console.error('WS8bm attachment preview diagnostics:',base,uploadResponses,
              await page.locator('.message[data-message-id]').evaluateAll(rows=>rows.map(row=>({id:row.dataset.messageId,preview:row.querySelector('.message__reply-preview')?.textContent,body:row.querySelector('[data-reply-target="body"]')?.textContent}))),
              await author.locator('#composer').evaluate(node=>({busy:node.getAttribute('aria-busy'),reply:node.querySelector('[data-composer-target="replyTo"]')?.value,feedback:node.querySelector('[data-composer-target="feedback"]')?.textContent})));
            throw error;
          }
          try {await waitForVisibility(attachment.getByRole('link',{name:'Download markdown-workspace-attachment.txt',exact:true}));}
          catch(error) {console.error('WS8bm attachment delivery:',base,await attachment.textContent());throw error;}
        }
        await waitForVisibility(author.locator('#composer [data-composer-target="context"][hidden]'),{state:'attached'});
      } else if(caseName==='mention suggestions select a room member without sending the unfinished message') {
        const editor=author.getByRole('combobox',{name:'Write a message',exact:true});
        const before=await visibleCount(messages(author));
        await editor.fill('@Kev');await waitForVisibility(author.locator('suggestion-option').filter({hasText:'Kevin'}));
        await editor.press('Enter');await field(author,'@[Kevin] ');
        assert.equal(await visibleCount(messages(author)),before);
        await submit(author,'@[Kevin] please review **the layout**.');
        for(const page of [author,recipient]) {
          const message=messages(page).filter({has:page.locator('strong').filter({hasText:'the layout'})});
          await waitForVisibility(message.locator('.mention').filter({hasText:'Kevin'}));
          assert.equal(await message.locator('.mention').getAttribute('data-user-id'),String(sessions.find(session=>session.user_name==='Kevin').user_id));
          if(page===recipient) await waitForVisibility(message.locator(':scope.message--mentioned'));
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
        await waitForVisibility(author.locator('.message--failed'));
        // Clearing the still-present failed input proves Restore draft reads
        // the saved submission, rather than passing on an unchanged editor.
        await author.getByRole('combobox',{name:'Write a message',exact:true}).fill('');
        await author.getByRole('button',{name:'Restore draft',exact:true}).click();await field(author,invalid);
        await submit(author,'**Recovered** after correcting the draft.');
        for(const page of [author,recipient]) await waitForVisibility(messages(page).locator('strong').filter({hasText:'Recovered'}));
      } else if(caseName==='sending preserves the submitted source and a newer draft') {
        const first='**First message** stays exact.',second='A newer draft is still here.';
        await author.getByRole('combobox',{name:'Write a message',exact:true}).fill(first);
        await author.evaluate(second=>{
          document.querySelector('#composer button[name="send"]').click();
          const editor=document.querySelector('#composer textarea[name="message[markdown_source]"]');
          editor.value=second;editor.dispatchEvent(new InputEvent('input',{bubbles:true,inputType:'insertFromPaste'}));
        },second);
        for(const page of [author,recipient]) await text(page,'First message stays exact.');
        await field(author,second);
        await waitForVisibleCount(messages(recipient).filter({hasText:second}),0);
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
      await waitForVisibility(author.locator('#message-actions-menu:not([hidden])'));
      if (caseName==='editing messages') {
        await author.getByRole('menuitem',{name:'Edit message',exact:true}).click();
        await waitForVisibility(author.locator('#composer').filter({hasText:'Editing Message'}));
        await send(author,'Redacted!');
        await text(recipient,'Redacted!');
        await text(recipient,"Third time's a charm.",0);
        await recipient.reload();
        await text(recipient,'Redacted!');
      } else if (caseName==='deleting messages') {
        author.once('dialog',dialog=>dialog.accept());
        await author.getByRole('menuitem',{name:'Delete message',exact:true}).click();
        await text(recipient,"Third time's a charm.",0);
        await waitForVisibleCount(messages(author).filter({hasText:"Third time's a charm."}),0);
      } else {throw new Error(`unimplemented case ${caseName}`);}
    }
  } catch(error) {
    console.error('WS8bm failed application:',base,caseName);
    if(caseName==='discusses a pull request from its card') {
      for(const context of contexts) for(const page of context.pages()) {
        console.error('WS8bm discussion diagnostic:',JSON.stringify({url:page.url(),threadResponses,
          cards:await page.locator('.github-pr-card').evaluateAll(cards=>cards.map(card=>card.outerHTML))}));
      }
    }
    throw error;
  } finally {for(const context of contexts) await context.close();}
}
try {
  let failures=0;
  for(const caseName of cases) {
    try {
    if(negative) {
      for(const variant of mutationVariants(caseName)) {
      for(const [app,base] of [['Rails',rails],['Rust',rust]]) {
      const probe={ready:false,applied:0};let failure;
      try {await acceptance(base,caseName,probe,variant);} catch(error) {failure=error;}
      if(!probe.ready || !probe.applied || !failure) console.error('WS8bm invalid discrimination run:',caseName,probe,failure);
      assert.ok(probe.ready,'mutant must reach the actual named case, not fail startup');
      assert.equal(probe.networkFailures?.length||0,0,'network failures cannot count as mutant rejection');
      assert.ok(probe.applied>0,'a deliberate served mutation must actually apply');
      assert.ok(failure,'named behaviour check must reject the served mutant');
      assert.ok(failure.code==='ERR_ASSERTION'||failure.name==='TimeoutError',`unexpected infrastructure/adapter failure: ${failure}`);
      if(probe.delayedWriteStarted) console.log(`WS8bm delayed-write probe: ${app}: ${Date.now()-probe.delayedWriteStarted} ms observed; actual write completed: ${!!probe.delayedWriteCompleted}`);
      console.log(`WS8bm discrimination: ${file}: ${caseName}: ${variant}: ${app} served mutant REJECTED (${failure.code||failure.name})`);
      }
      }
    } else {
      for(const [app,base] of [['Rails',rails],['Rust',rust]]) {
        const probe={ready:false,applied:0};
        await acceptance(base,caseName,probe,selectedMutant||'default');
        if(selectedMutant) {
          assert.ok(probe.ready&&probe.applied>0,'probe must actually apply after valid startup');
          assert.equal(probe.networkFailures?.length||0,0);
          console.log(`WS8bm review escape: ${file}: ${caseName}: ${selectedMutant}: ${app} ACCEPTED`);
        }
      }
      if(!selectedMutant) console.log(`WS8bm browser flow: ${file}: ${caseName}: Rails PASS; Rust PASS`);
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
