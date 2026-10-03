// Observable behaviour from the pinned system cases, through real browser controls.
import assert from 'node:assert/strict';
import {waitForVisibility,installVisibility,setVisibilityTimeout,waitForVisibleCount,visibleCount,waitForVisibleProperty,waitForVisibleAttribute,waitForVisibleContentCount,actOnVisible,filterVisibleText,visibleText,waitForDomCount} from './behavior-visibility.mjs';
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {execFileSync} from 'node:child_process';
import {messageList} from './behavior-message-list.mjs';
import {searchForward} from './behavior-search-forward.mjs';
import {rejectionEvidence} from './behavior-discrimination.mjs';
import {installMutation,mutationVariants} from './behavior-mutations.mjs';
import {unreadDivider} from './behavior-unread.mjs';
import {destinationCases,messageDestinations} from './behavior-message-destinations.mjs';
import {composer} from './behavior-composer.mjs';
import {attachMenu} from './behavior-attach-menu.mjs';
import {boosts} from './behavior-boosts.mjs';
import {interactions,mobileActions,assertMenuOpen} from './behavior-actions.mjs';
import {toolbar} from './behavior-toolbar.mjs';
import {codeHighlighting} from './behavior-code.mjs';
import {threadContinuation,continuationCases} from './behavior-thread-continuation.mjs';
import {workControllers} from './behavior-work-controllers.mjs';
import {mobileContinuation} from './behavior-mobile-continuation.mjs';
import {workspace,WORKSPACE_CASE} from './behavior-workspace.mjs';
import {motion,motionCases} from './behavior-motion.mjs';
import {nativePhone,PHONE_CASE} from './behavior-native-phone.mjs';
import {CAPYBARA_DEFAULT,DELIVERY_WAIT,CABLE_WAIT} from './behavior-deadlines.mjs';
const require=createRequire(new URL('../../parity/package.json',import.meta.url));
const {chromium}=require('playwright');
const sessions=JSON.parse(readFileSync(new URL('../../vectors/campfire_sessions.json',import.meta.url))).sessions;
const [rails,rust,file,caseNames,fixtureJson='{}']=process.argv.slice(2);
const cases=JSON.parse(caseNames);
const fixture=JSON.parse(fixtureJson);
assert.ok(['motion','mobile_layout','channel_threads_controller','sending_messages','workspace_markdown','threads','message_list_a11y','search_forward_edit','unread_divider','composer','composer_attach_menu','boosting_messages','message_interactions','message_actions_mobile','message_toolbar','code_highlighting'].includes(file));
const browser=await chromium.launch({headless:true});
const negative=process.env.WS8BM_NEGATIVE==='1';
const keepGoing=process.env.WS8BM_KEEP_GOING==='1';
const selectedMutant=process.env.WS8BM_MUTANT;
async function acceptance(base,caseName,probe={},variant='default') {
  const contexts=[],threadResponses=[];
  try {
    // The positive phone control is the pinned Selenium sequence itself.
    // Served negatives retain their translated initial creation checkpoint.
    if(caseName===PHONE_CASE&&!negative&&!selectedMutant) {
      await nativePhone(base);return;
    }
    async function viewer(name) {
      const height=file==='unread_divider'&&caseName.startsWith('many unread')?700:1000;
      // Mutation routes must remain observable across navigations; a service
      // worker can otherwise fetch/cache the original asset outside page.route.
      // Positive acceptance retains the app's actual service worker.
      const context=await browser.newContext({viewport:{width:1440,height},...(negative||selectedMutant?{serviceWorkers:'block'}:{})});
      contexts.push(context);
      await installVisibility(context);
      // These pinned system cases run with Rails.env.test? and the layout's
      // data-test-motion="off" input (application.html.erb:2). Our servers use
      // the production reference image. Supply that test-only input before
      // parsing either app; this does not claim the server emits the attribute.
      const pinnedTestMotion = caseName===WORKSPACE_CASE || caseName==='Markdown replies and file attachments remain usable' || continuationCases.includes(caseName) || file==='mobile_layout' || caseName.startsWith('text fields') || caseName==='thread code stays readable in both themes and scrolls within a narrow screen';
      if(pinnedTestMotion) await context.addInitScript(()=>{
        const apply=()=>document.documentElement?.setAttribute('data-test-motion','off');
        apply();new MutationObserver(apply).observe(document,{childList:true,subtree:true});
      });
      const [cookie,...value]=sessions.find(s=>s.user_name===name).cookie_header.split('=');
      await context.addCookies([{name:cookie,value:value.join('='),url:base}]);
      const page=await context.newPage();
      page.setDefaultTimeout(CAPYBARA_DEFAULT);
      setVisibilityTimeout(page,CAPYBARA_DEFAULT);
      // Capybara visit/page-load is separate from selector assertions.
      page.setDefaultNavigationTimeout(30000);
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
      try {await page.waitForFunction(()=>window.Stimulus?.getControllerForElementAndIdentifier(document.getElementById('composer'),'composer'),null,{timeout:CABLE_WAIT});}
      catch(error) {console.error('WS8bm browser startup:',await page.evaluate(()=>({url:location.href,title:document.title,stimulus:!!window.Stimulus,controllers:document.getElementById('composer')?.dataset.controller,scripts:[...document.scripts].map(s=>s.src||s.type)})));throw error;}
      await waitForVisibility(page.locator('turbo-cable-stream-source[channel="RoomMessagesChannel"][connected]'),{state:'attached',timeout:CABLE_WAIT});
      return page;
    }
    const profileActors=file==='message_list_a11y'&&caseName==='profile message and ban buttons have accessible names';
    const author=await viewer('JZ'),recipient=await viewer(profileActors||file==='message_interactions'?'David':'Kevin');
    // Startup errors are never accepted as proof of assertion discrimination.
    probe.ready=true;
    const messages=page=>page.locator('.message[data-message-id]');
    async function text(page,value,count=1,options={}) {
      await waitForVisibleContentCount(page.locator('.message[data-message-id] .message__body'),'[data-reply-target="body"]',value,count,options);
    }
    async function send(page,value,options={}) {
      await actOnVisible(page.getByRole('combobox',{name:'Write a message',exact:true}),'fill',{},[value]);
      await actOnVisible(page.getByRole('button',{name:'Send Message',exact:true}),'click',{});
      await text(page,value,1,options);
    }
    async function submit(page,value) {
      await actOnVisible(page.getByRole('combobox',{name:'Write a message',exact:true}),'fill',{},[value]);
      await actOnVisible(page.getByRole('button',{name:'Send Message',exact:true}),'click',{});
    }
    async function openEdit(page,message,{contextTimeout}={}) {
      await actOnVisible(message.locator('[data-message-edit-format], [data-reply-target="body"]').first(),'click',{button:'right'});
      await assertMenuOpen(page);
      await actOnVisible(page.getByRole('menuitem',{name:'Edit message',exact:true}),'click',{});
      if(contextTimeout) await waitForVisibility(filterVisibleText(page.locator('#composer [data-composer-target="contextLabel"]'),'Editing Message'),{timeout:contextTimeout});
      else if(file==='sending_messages'||file==='boosting_messages'||file==='workspace_markdown') await waitForVisibility(filterVisibleText(page.locator('#composer'),'Editing Message'));
    }
    async function field(page,value,options={}) {
      await waitForVisibleProperty(page.locator('#composer textarea[name="message[markdown_source]"]'),'value',value,options);
    }
    if(file==='motion') {await motion({author,base,caseName,fixture});return;}
    if(file==='mobile_layout'||caseName.startsWith('text fields')) {await mobileContinuation({author,base,caseName,fixture});return;}
    if(file==='channel_threads_controller') {await workControllers({author,recipient,base,caseName,fixture});return;}
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
      const card=()=>author.locator('.github-pr-card').filter({has:filterVisibleText(author.locator('.github-pr-card__title'),'Fix login')});
      await waitForVisibility(card(),{timeout:DELIVERY_WAIT});
      await actOnVisible(card().getByRole('button',{name:'Discuss',exact:true}),'click',{});
      await waitForVisibility(filterVisibleText(author.locator('.github-pr-thread-header .github-pr-card__title'),'Fix login'),{timeout:DELIVERY_WAIT});
      await waitForVisibility(filterVisibleText(author.locator('.github-pr-files__heading'),'Files changed'));
      await waitForVisibility(filterVisibleText(author.locator('.github-pr-files__path'),'app/models/user.rb'));
      const threadPath=new URL(author.url()).pathname;assert.match(threadPath,/^\/rooms\/654632876\/threads\/\d+$/);
      await author.goto(base+'/rooms/654632876');await waitForVisibility(card(),{timeout:DELIVERY_WAIT});
      await actOnVisible(card().getByRole('link',{name:'Discuss',exact:true}),'click',{});
      await waitForVisibility(filterVisibleText(author.locator('.github-pr-thread-header .github-pr-card__title'),'Fix login'),{timeout:DELIVERY_WAIT});
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
          await actOnVisible(root.locator('[data-message-edit-format], [data-reply-target="body"]').first(),'click',{button:'right'});
          await actOnVisible(page.getByRole('menuitem',{name:'Create thread',exact:true,includeHidden:true}),'click',{});
        } else {
          if(await visibleCount(page.locator('[data-thread-panel-target="browserToggle"]'))) await actOnVisible(page.locator('[data-thread-panel-target="browserToggle"]'),'click');
          else {await actOnVisible(page.getByRole('button',{name:'More actions',exact:true,includeHidden:true}),'click');await actOnVisible(page.locator('#header-overflow-menu [data-thread-panel-target="browserToggle"]'),'click');}
          // Match the pinned open_threads helper's actual open-state assertion.
          await waitForVisibility(panel.locator(':scope[aria-hidden="false"]'),{timeout:DELIVERY_WAIT});
          await actOnVisible(panel.getByRole('button',{name:'New thread',exact:true,includeHidden:true}),'click',{});
        }
        await waitForVisibility(panel.locator('[data-thread-panel-target="create"]'),{timeout:DELIVERY_WAIT});
        // beginCreate hands focus to First message on the next animation
        // frame. Wait for that observable open-state transition before
        // typing the name, so insertText cannot land in the other field.
        await page.waitForFunction(()=>document.activeElement===document.querySelector('[data-thread-panel-target="createMessage"]'));
        const nameField=panel.locator('[data-thread-panel-target="createName"]');
        assert.match(await nameField.evaluate(input=>input.closest('label')?.textContent||''),/Thread name/);
        await actOnVisible(nameField,'fill',{},[name]);
        const firstField=panel.locator('[data-thread-panel-target="createMessage"]');
        assert.match(await firstField.evaluate(input=>input.closest('label')?.textContent||''),/First message/);
        await actOnVisible(firstField,'fill',{},[first]);
      }
      async function finishCreate(name,page=author) {
        const panel=page.locator('#thread-panel');
        assert.equal(await panel.locator('[data-thread-panel-target="createName"]').inputValue(),name,`${base}: ${caseName}: the completed name must survive until submission`);
        await actOnVisible(panel.locator('[data-thread-panel-target="createSubmit"]'),'click',{});
        try {
          await waitForVisibility(panel.locator('[data-thread-panel-target="conversation"]'),{timeout:DELIVERY_WAIT});
          await waitForVisibility(filterVisibleText(panel.locator('[data-thread-panel-target="conversationTitle"]'),name),{timeout:DELIVERY_WAIT});
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
      const threadMessage=value=>panel.locator('.message[data-message-id]').filter({has:filterVisibleText(author.locator('[data-reply-target="body"]'),value)});
      async function assertThreadMessage(value) {
        await waitForVisibility(filterVisibleText(panel.locator('.thread-panel__thread-content .message__body'),value),{timeout:DELIVERY_WAIT});
      }
      async function reply(value) {
        await actOnVisible(panel.getByRole('combobox',{name:'Write a thread reply',exact:true,includeHidden:true}),'fill',{},[value]);
        await actOnVisible(panel.getByRole('button',{name:'Send Reply',exact:true,includeHidden:true}),'click',{});
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
        await actOnVisible(message.locator('[data-message-edit-format], [data-reply-target="body"]').first(),'click',{button:'right'});
        await waitForVisibility(author.locator('#message-actions-menu:not([hidden])'),{timeout:10000});
      }
      async function close(page) {
        await actOnVisible(page.getByRole('button',{name:'Close threads',exact:true,includeHidden:true}),'click',{});
        await page.waitForFunction(()=>!document.body.classList.contains('thread-panel-open'),null,{timeout:DELIVERY_WAIT});
      }
      if(continuationCases.includes(caseName)) {
        try {await threadContinuation({author,recipient,base,caseName,fixture,create,finishCreate,assertThreadMessage,reply,threadMenu,close});}
        catch(error) {console.error('WS8bm continuation diagnostics:',base,caseName,await author.evaluate(()=>({
          body:document.body.className,panel:document.querySelector('#thread-panel')?.outerHTML.slice(0,1600),
          views:[...document.querySelectorAll('#thread-panel section,#thread-panel button,#header-overflow-button')].map(node=>({tag:node.tagName,text:node.textContent.trim(),hidden:node.hidden,aria:node.getAttribute('aria-hidden'),opacity:getComputedStyle(node).opacity,visibility:getComputedStyle(node).visibility,display:getComputedStyle(node).display,rect:node.getBoundingClientRect().toJSON()}))
        })));throw error;}
      } else if(caseName==='rejects an external thread deep link before fetching it') {
        const external='https://attacker.invalid/rooms/1/threads/999';
        const requests=[];
        author.on('request',request=>requests.push(request.url()));
        const response=await author.goto(base+'/rooms/654632876?thread='+encodeURIComponent(external));
        assert.equal(response.status(),200);
        await waitForVisibility(author.locator('#thread-panel[aria-hidden="false"]'),{timeout:DELIVERY_WAIT});
        await waitForVisibility(filterVisibleText(panel.locator('[data-thread-panel-target="threadStatus"]'),'This thread link is invalid.'));
        await waitForVisibleCount(panel.locator('.thread-panel__thread-content .message'),0);
        assert.equal(requests.some(url=>url.startsWith('https://attacker.invalid/')),false);
        assert.equal(await author.evaluate(()=>performance.getEntriesByType('resource').some(entry=>entry.name.startsWith('https://attacker.invalid/'))),false);
      } else if(caseName==='renders untrusted thread metadata as text') {
        const malicious='<img src=x onerror="window.__threadXss = true">';
        await create(malicious,'A safe thread body.',false);await finishCreate(malicious);
        const title=panel.locator('[data-thread-panel-target="conversationTitle"]');
        assert.equal((await visibleText(title)).trim(),malicious);
        await waitForVisibleCount(title.locator('img'),0);
        assert.equal(await author.evaluate(()=>window.__threadXss),undefined);
        await assertThreadMessage('A safe thread body.');
      } else if(caseName==='browses active and closed threads and can join or leave a closed one') {
        await create('Active planning thread','The active planning conversation.',false);await finishCreate('Active planning thread');await close(author);
        const david=await viewer('David');
        await create('Closed planning thread','The closed planning conversation.',false,david);await finishCreate('Closed planning thread',david);
        const other=david.locator('#thread-panel');
        await actOnVisible(other.locator('[data-thread-panel-target="manage"] summary'),'click',{});
        await waitForVisibility(other.locator('[data-thread-panel-target="closeThread"]'),{timeout:DELIVERY_WAIT});
        await actOnVisible(other.locator('[data-thread-panel-target="closeThread"]'),'click',{});
        await waitForVisibility(filterVisibleText(other.locator('[data-thread-panel-target="threadStatus"]'),/Closed thread/),{timeout:DELIVERY_WAIT});
        await close(david);
        await actOnVisible(author.locator('[data-thread-panel-target="browserToggle"]'),'click');
        await waitForVisibility(panel.locator('[data-thread-panel-target="browser"]'));
        const items=panel.locator('[data-thread-panel-target="browserList"] .thread-panel__thread-item');
        await waitForVisibility(filterVisibleText(items,'Active planning thread'),{timeout:DELIVERY_WAIT});
        await waitForVisibleCount(filterVisibleText(items,'Closed planning thread'),0);
        await actOnVisible(panel.locator('[data-thread-panel-target="filter"]'),'selectOption',{},['closed']);
        await waitForVisibility(filterVisibleText(items,'Closed planning thread'),{timeout:DELIVERY_WAIT});
        await actOnVisible(filterVisibleText(items,'Closed planning thread'),'click',{});
        await waitForVisibility(panel.locator('[data-thread-panel-target="conversation"]'),{timeout:DELIVERY_WAIT});
        await waitForVisibility(panel.locator('[data-thread-panel-target="join"]'),{timeout:DELIVERY_WAIT});
        await waitForVisibility(panel.locator('[data-thread-panel-target="leave"]'),{state:'hidden'});
        await actOnVisible(panel.getByRole('button',{name:'Join',exact:true,includeHidden:true}),'click',{});
        await waitForVisibility(panel.locator('[data-thread-panel-target="leave"]'),{timeout:DELIVERY_WAIT});
        await waitForVisibility(panel.locator('[data-thread-panel-target="join"]'),{state:'hidden'});
        await actOnVisible(panel.getByRole('button',{name:'Leave',exact:true,includeHidden:true}),'click',{});
        await waitForVisibility(panel.locator('[data-thread-panel-target="join"]'),{timeout:DELIVERY_WAIT});
        await waitForVisibility(panel.locator('[data-thread-panel-target="leave"]'),{state:'hidden'});
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
          await waitForVisibility(page.locator(`[id="${indicatorId}"][aria-label="Open thread, 1 reply"]`),{timeout:CABLE_WAIT});
          assert.equal((await visibleText(page.locator(`[id="${indicatorId}"]`))).trim(),'1 reply');
          await waitForVisibility(page.locator(`[id="${indicatorId}"] img.colorize--black`));
        }
        await reply('A second reply.');
        for(const page of [author,recipient]) await waitForVisibility(page.locator(`[id="${indicatorId}"][aria-label="Open thread, 2 replies"]`),{timeout:CABLE_WAIT});
        for(const value of ['A second reply.','The only reply.']) {
          await threadMenu(value);author.once('dialog',dialog=>dialog.accept());
          await actOnVisible(author.getByRole('menuitem',{name:'Delete message',exact:true,includeHidden:true}),'click',{});
          await waitForVisibility(threadMessage(value),{state:'hidden',timeout:DELIVERY_WAIT});
        }
        for(const page of [author,recipient]) await waitForVisibility(page.locator(`[id="${indicatorId}"]`),{state:'hidden',timeout:CABLE_WAIT});
        for(const page of [author,recipient]) await waitForVisibility(page.locator(`[id="${indicatorId}"][hidden]`),{state:'attached'});
      } else if(caseName==='creates a thread from a channel message and keeps the channel draft separate') {
        const first='Let’s keep the design review focused here.';
        await create('Design review thread',first);await finishCreate('Design review thread');
        await waitForVisibility(filterVisibleText(panel.locator('[data-thread-panel-target="parent"]'),"Third time's a charm."),{timeout:DELIVERY_WAIT});
        await assertThreadMessage(first);
        await actOnVisible(panel.locator('[data-thread-panel-target="preferences"] summary'),'click',{});
        await actOnVisible(panel.locator('[data-thread-panel-target="involvement"]'),'selectOption',{},['nothing']);
        await actOnVisible(panel.locator('[data-thread-panel-target="manage"] summary'),'click',{});
        await actOnVisible(panel.locator('[data-thread-panel-target="autoArchive"]'),'selectOption',{},['1440']);
        await actOnVisible(author.getByRole('combobox',{name:'Write a message',exact:true,includeHidden:true}),'fill',{},['A channel draft stays here.']);
        await reply('A reply from the thread drawer.');await field(author,'A channel draft stays here.');
        await threadMenu('A reply from the thread drawer.');
        await actOnVisible(author.getByRole('menuitem',{name:'Reply',exact:true,includeHidden:true}),'click',{});
        await waitForVisibility(filterVisibleText(panel.locator('[data-composer-target="contextLabel"]'),'Replying to'),{timeout:DELIVERY_WAIT});
        await reply('A reply to the drawer message.');
        await waitForVisibility(filterVisibleText(panel.locator('.message__reply-preview'),'A reply from the thread drawer.'),{timeout:DELIVERY_WAIT});
        await threadMenu(first);await actOnVisible(author.getByRole('menuitem',{name:'Edit message',exact:true,includeHidden:true}),'click',{});
        await waitForVisibility(filterVisibleText(panel.locator('[data-composer-target="contextLabel"]'),'Editing Message'),{timeout:DELIVERY_WAIT});
        await reply('The edited thread starter.');
        await waitForVisibility(filterVisibleText(panel.locator('[data-thread-panel-target="parent"]'),"Third time's a charm."),{timeout:DELIVERY_WAIT});
        await threadMenu('A reply to the drawer message.');
        await actOnVisible(author.locator('.message__quick-reaction[title="Thumbs up"]'),'click',{});
        await waitForVisibility(filterVisibleText(panel.locator('.boosts__reactions'),'👍'),{timeout:DELIVERY_WAIT});
        await field(author,'A channel draft stays here.');
      } else {throw new Error(`unimplemented case ${caseName}`);}
      return;
    }
    if (file==='workspace_markdown') {
      const source=execFileSync('git',['show','d7c7de92:test/system/workspace_markdown_test.rb'],{encoding:'utf8'});
      const heredoc=(start,indent)=>source.split(start)[1].split(`${' '.repeat(indent)}MARKDOWN`)[0].split('\n').map(line=>line.slice(indent+2)).join('\n');
      if(caseName===WORKSPACE_CASE) {await workspace({author,source,submit});return;}
      if (caseName==='Markdown messages reach other users and editing preserves the original source') {
        const markdown=heredoc("MARKDOWN = <<~'MARKDOWN'.freeze\n",2);
        await submit(author,markdown);
        async function content(page,id,initial=false) {
          const message=page.locator(`.message[data-message-id="${id}"]`);
          if(initial) await waitForVisibility(filterVisibleText(message.locator('h2'),/^Design review$/),{timeout:CAPYBARA_DEFAULT});
          for(const [selector,value] of [['strong','Ready for review'],['em','clear ownership'],['del','old assumptions'],['blockquote','Keep the conversation close to the work.'],['ul li','Check the channel layout'],['table td','Markdown']]) {
            await waitForVisibility(filterVisibleText(message.locator(selector),value));
          }
          await waitForVisibleCount(message.locator('input[type="checkbox"][disabled]'),2);
          const code=filterVisibleText(message.locator('pre code'),'const message = "<script>literal code</script>";');
          await waitForVisibility(code,{timeout:CAPYBARA_DEFAULT});
          assert.ok((await visibleText(code)).includes('const message = "<script>literal code</script>";'));
          await waitForVisibility(filterVisibleText(message.locator('pre code.language-javascript[data-highlighted="yes"] .code-token'),'const'),{timeout:20000});
          await waitForVisibleCount(message.locator('.markdown-code-copy'),1);
          await waitForVisibleAttribute(message.getByRole('link',{name:'Project notes',exact:true}),'href','https://example.com/notes');
        }
        const message=messages(author).filter({has:filterVisibleText(author.locator('h2'),/^Design review$/)});
        await waitForVisibility(filterVisibleText(message.locator('h2'),/^Design review$/),{timeout:CAPYBARA_DEFAULT});
        const id=await message.getAttribute('data-message-id');
        await content(author,id,true);await content(recipient,id,true);
        await openEdit(author,message);await field(author,markdown);
        const edited=markdown.replace('Design review','Review complete');
        await submit(author,edited);
        for(const page of [author,recipient]) {
          await waitForVisibility(filterVisibleText(page.locator(`.message[data-message-id="${id}"] h2`),/^Review complete$/));
          await content(page,id);
        }
        await author.reload();
        await author.waitForFunction(()=>window.Stimulus?.getControllerForElementAndIdentifier(document.getElementById('composer'),'composer'));
        await openEdit(author,author.locator(`.message[data-message-id="${id}"]`));
        await field(author,edited);
      } else if (caseName==='desktop keyboard composition keeps line breaks and sends once after composition ends') {
        const editor=author.getByRole('combobox',{name:'Write a message',exact:true});
        await actOnVisible(editor,'fill',{},['First line']);await actOnVisible(editor,'press',{},['Shift+Enter']);await actOnVisible(editor,'pressSequentially',{},['Second line']);
        await field(author,'First line\nSecond line');
        const before=await visibleCount(messages(author));
        await editor.evaluate(editor=>editor.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',code:'Enter',keyCode:13,isComposing:true,bubbles:true,cancelable:true})));
        await field(author,'First line\nSecond line');assert.equal(await visibleCount(messages(author)),before);
        await actOnVisible(editor,'press',{},['Enter']);
        for(const page of [author,recipient]) {
          // workspace_markdown :107-108 asserts each visible line separately;
          // the exact two-line saved source remains checked in SQLite.
          await text(page,'First line');await text(page,'Second line');
        }
        await field(author,'');
        await actOnVisible(editor,'press',{},['ArrowUp']);
        await waitForVisibility(filterVisibleText(author.locator('#composer'),'Editing Message'));
        await field(author,'First line\nSecond line');
      } else if (caseName==='untrusted markup stays inert in the delivered message') {
        const payload=heredoc("payload = <<~'MARKDOWN'\n",4);
        await submit(author,payload);
        for(const page of [author,recipient]) {
          const message=messages(page).filter({has:filterVisibleText(page.locator('p'),/^Safety check$/)});
          await waitForVisibility(filterVisibleText(message.locator('.message__body'),'Safety check'),{timeout:CAPYBARA_DEFAULT});
          await waitForVisibility(filterVisibleText(message.locator('pre code'),'<img onerror="literal code">'));
          await waitForDomCount(message.locator('script, img[onerror], a[href^="javascript:"]'),0);
          assert.equal(await page.evaluate(()=>window.markdownPayloadExecuted===true),false);
        }
      } else if(caseName==='Markdown replies and file attachments remain usable') {
        const uploadResponses=[];
        author.on('response',response=>{
          if(response.request().method()==='POST'&&new URL(response.url()).pathname==='/rooms/654632876/messages') uploadResponses.push({status:response.status(),type:response.headers()['content-type']});
        });
        const source='**A useful point** with `inline code`.';
        await submit(author,source);
        const parent=messages(author).filter({has:filterVisibleText(author.locator('strong'),'A useful point')});
        await waitForVisibility(filterVisibleText(parent.locator('.message__body'),'A useful point'),{timeout:CAPYBARA_DEFAULT});
        await actOnVisible(parent.locator('[data-message-edit-format], [data-reply-target="body"]').first(),'click',{button:'right'});
        await actOnVisible(author.getByRole('menuitem',{name:'Reply',exact:true}),'click',{});
        await waitForVisibility(filterVisibleText(author.locator('#composer [data-composer-target="contextLabel"]'),'Replying to JZ'));
        await waitForVisibility(filterVisibleText(author.locator('#composer [data-composer-target="contextPreview"]'),'A useful point'));
        await field(author,'');await actOnVisible(author.getByLabel('Notify author',{exact:true}),'uncheck',{});
        await author.locator('#composer input[type="file"]').setInputFiles({name:'markdown-workspace-attachment.txt',mimeType:'text/plain',buffer:Buffer.from('An attachment sent from the Markdown composer.\n')});
        await waitForVisibility(filterVisibleText(author.locator('#composer'),'markdown-workspace-attachment'));
        await actOnVisible(author.getByRole('button',{name:'Send Message',exact:true}),'click',{});
        for(const page of [author,recipient]) {
          const attachment=messages(page).filter({has:filterVisibleText(page.locator('.message__reply-preview'),'A useful point')});
          try {await waitForVisibility(filterVisibleText(attachment.locator('.message__reply-preview'),'A useful point'),{timeout:DELIVERY_WAIT});}
          catch(error) {
            console.error('WS8bm attachment preview diagnostics:',base,uploadResponses,
              await page.locator('.message[data-message-id]').evaluateAll(rows=>rows.map(row=>({id:row.dataset.messageId,preview:row.querySelector('.message__reply-preview')?.textContent,body:row.querySelector('[data-reply-target="body"]')?.textContent}))),
              await author.locator('#composer').evaluate(node=>({busy:node.getAttribute('aria-busy'),reply:node.querySelector('[data-composer-target="replyTo"]')?.value,feedback:node.querySelector('[data-composer-target="feedback"]')?.textContent})));
            throw error;
          }
          // Rails workspace_markdown:166 uses assert_message_text (:115),
          // not an accessible download name. Check the actual visible body.
          await waitForVisibility(filterVisibleText(attachment.locator('.message__body'),'markdown-workspace-attachment.txt'));
          try {await waitForVisibility(attachment.getByRole('link',{name:'Download markdown-workspace-attachment.txt',exact:true}));}
          catch(error) {console.error('WS8bm attachment delivery:',base,await attachment.textContent());throw error;}
        }
        await waitForVisibility(author.locator('#composer [data-composer-target="context"][hidden]'),{state:'attached'});
      } else if(caseName==='mention suggestions select a room member without sending the unfinished message') {
        const editor=author.getByRole('combobox',{name:'Write a message',exact:true});
        const before=await visibleCount(messages(author));
        await actOnVisible(editor,'fill',{},['@Kev']);await waitForVisibility(filterVisibleText(author.locator('suggestion-option'),'Kevin'));
        await actOnVisible(editor,'press',{},['Enter']);await field(author,'@[Kevin] ');
        assert.equal(await visibleCount(messages(author)),before);
        await submit(author,'@[Kevin] please review **the layout**.');
        for(const page of [author,recipient]) {
          const message=messages(page).filter({has:filterVisibleText(page.locator('strong'),'the layout')});
          await waitForVisibility(filterVisibleText(message.locator('.mention'),'Kevin'));
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
        await actOnVisible(author.getByRole('button',{name:'Send Message',exact:true}),'click',{});
        await waitForVisibility(author.locator('.message--failed'));
        // Clearing the still-present failed input proves Restore draft reads
        // the saved submission, rather than passing on an unchanged editor.
        await actOnVisible(author.getByRole('combobox',{name:'Write a message',exact:true}),'fill',{},['']);
        await actOnVisible(author.getByRole('button',{name:'Restore draft',exact:true}),'click',{});await field(author,invalid);
        await submit(author,'**Recovered** after correcting the draft.');
        for(const page of [author,recipient]) await waitForVisibility(filterVisibleText(messages(page).locator('strong'),'Recovered'));
      } else if(caseName==='sending preserves the submitted source and a newer draft') {
        const first='**First message** stays exact.',second='A newer draft is still here.';
        await actOnVisible(author.getByRole('combobox',{name:'Write a message',exact:true}),'fill',{},[first]);
        await author.evaluate(second=>{
          document.querySelector('#composer button[name="send"]').click();
          const editor=document.querySelector('#composer textarea[name="message[markdown_source]"]');
          editor.value=second;editor.dispatchEvent(new InputEvent('input',{bubbles:true,inputType:'insertFromPaste'}));
        },second);
        for(const page of [author,recipient]) await text(page,'First message stays exact.');
        await field(author,second);
        await waitForVisibleCount(filterVisibleText(messages(recipient),second),0);
        await actOnVisible(author.getByRole('button',{name:'Send Message',exact:true}),'click',{});
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
      await actOnVisible(original.locator('[data-message-edit-format], [data-reply-target="body"]').first(),'click',{button:'right'});
      await waitForVisibility(author.locator('#message-actions-menu:not([hidden])'),{timeout:10000});
      if (caseName==='editing messages') {
        await actOnVisible(author.getByRole('menuitem',{name:'Edit message',exact:true}),'click',{});
        await waitForVisibility(filterVisibleText(author.locator('#composer'),'Editing Message'));
        await send(author,'Redacted!');
        await text(recipient,'Redacted!');
        await text(recipient,"Third time's a charm.",0);
        await recipient.reload();
        await text(recipient,'Redacted!');
      } else if (caseName==='deleting messages') {
        author.once('dialog',dialog=>dialog.accept());
        await actOnVisible(author.getByRole('menuitem',{name:'Delete message',exact:true}),'click',{});
        await text(recipient,"Third time's a charm.",0);
        await waitForVisibleCount(filterVisibleText(messages(author),"Third time's a charm."),0);
      } else {throw new Error(`unimplemented case ${caseName}`);}
    }
  } catch(error) {
    console.error('WS8bm failed application:',base,caseName);
    probe.observed=(await Promise.all((probe.observers||[]).map(observe=>observe()))).flat();
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
      const invalidApps=[];
      for(const [app,base] of [['Rails',rails],['Rust',rust]]) {
      const probe={ready:false,applied:0};let failure;
      try {await acceptance(base,caseName,probe,variant);} catch(error) {failure=error;}
      const evidence=rejectionEvidence(caseName,variant,probe,failure);
      if(!evidence.valid) {
        const escaped=!failure&&probe.ready&&probe.applied;
        console.error(`WS8bm ${escaped?'escaped':'invalid'} discrimination run:`,JSON.stringify({caseName,variant,app,...evidence}),failure?.stack);
        const error=new Error(`No rejection credit: ${evidence.reasons.join('; ')}`);
        error.code=escaped?'WS8BM_MUTANT_ESCAPED':'WS8BM_INVALID_DISCRIMINATION';
        invalidApps.push(error);
        continue;
      }
      console.log(`WS8bm intended assertion: ${app}: ${caseName}: ${variant}: ${JSON.stringify(evidence)}`);
      if(probe.delayedWriteStarted) console.log(`WS8bm delayed-write probe: ${app}: ${Date.now()-probe.delayedWriteStarted} ms observed; actual write completed: ${!!probe.delayedWriteCompleted}`);
      console.log(`WS8bm discrimination: ${file}: ${caseName}: ${variant}: ${app} served mutant REJECTED (${failure.code||failure.name})`);
      }
      if(invalidApps.length) throw new AggregateError(invalidApps,'Uncredited discrimination attempts');
      }
    } else {
      const failedApps=[];
      for(const [app,base] of [['Rails',rails],['Rust',rust]]) {
        const probe={ready:false,applied:0};
        try {await acceptance(base,caseName,probe,selectedMutant||'default');}
        catch(error) {console.error(`WS8bm positive application FAILED: ${app}: ${caseName}:`,error.stack);failedApps.push(error);continue;}
        if(selectedMutant) {
          assert.ok(probe.ready&&probe.applied>0,'probe must actually apply after valid startup');
          assert.equal(probe.networkFailures?.length||0,0);
          console.log(`WS8bm review escape: ${file}: ${caseName}: ${selectedMutant}: ${app} ACCEPTED`);
        }
      }
      if(failedApps.length) throw new AggregateError(failedApps,'Unpaired positive attempts');
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
