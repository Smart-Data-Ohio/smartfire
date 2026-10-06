// Non-pixel code_highlighting_test.rb flows from pinned d7c7de92.
import assert from 'node:assert/strict';
import {waitForVisibility,waitForVisibleCount,actOnVisible,filterVisibleText,waitForDomCount,visibleMatch} from './behavior-visibility.mjs';
import {CAPYBARA_DEFAULT} from './behavior-deadlines.mjs';
export async function codeHighlighting({author:page,recipient,caseName,fixture,base,openEdit,field}) {
  const {samples,literal_code_source,code_source,code_replacement,highlight_wait,code_search_id}=fixture;
  const marked=(row,language)=>row.locator(`pre code[class~="language-${language}"][data-highlighted="yes"]`);
  async function highlight(code,keyword) {
    if(keyword) {
      // The original waits once for the highlighted keyword, not for any
      // token or for two sequential twenty-second readiness conditions.
      await waitForVisibility(filterVisibleText(code.locator('.code-token'),keyword).first(),{timeout:highlight_wait*1000});
    } else {
      await waitForVisibility(code,{timeout:highlight_wait*1000});
      await waitForVisibility(code.locator('.code-token').first(),{timeout:CAPYBARA_DEFAULT});
    }
  }
  async function post(source) {
    await actOnVisible(page.getByRole('combobox',{name:'Write a message',exact:true}),'fill',{},[source]);
    await actOnVisible(page.getByRole('button',{name:'Send Message',exact:true}),'click',{});
    return page.locator('.message[data-message-id]').filter({has:page.locator('pre code')});
  }
  async function copy(row,expected) {
    // Same original clipboard permission-independent boundary. The actual
    // copy handler and its saved source still run.
    await page.evaluate(()=>{navigator.clipboard.writeText=async source=>{window.copiedCode=source;};});
    await actOnVisible(row.getByRole('button',{name:'Copy code',exact:true}),'click',{});await waitForVisibility(row.getByRole('button',{name:'Code copied',exact:true}));
    assert.equal(await page.evaluate(()=>window.copiedCode),expected);
  }
  if(caseName.startsWith('thread code stays')) {
    await page.goto(`${base}/rooms/654632876?thread=${fixture.code_thread_id}&message_id=${fixture.code_thread_message_id}`);
    const row=page.locator(`.message[data-message-id="${fixture.code_thread_message_id}"]`);await highlight(marked(row,'ts'),'const');
    const colors=[];
    for(const colorScheme of ['light','dark']) {
      await page.emulateMedia({colorScheme});
      const code=await visibleMatch(page.locator('#thread-panel pre code.language-ts'));
      const keyword=await visibleMatch(filterVisibleText(code.locator('.code-token'),'const'));
      const color=await keyword.evaluate(node=>getComputedStyle(node).color);assert.notEqual(await code.evaluate(node=>getComputedStyle(node).color),color);colors.push(color);
    }
    assert.deepEqual(colors,['rgb(0, 0, 255)','rgb(86, 156, 214)']);
    await page.setViewportSize({width:390,height:844});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
    const pre=await visibleMatch(page.locator('#thread-panel pre'));assert.ok(await pre.evaluate(node=>node.scrollWidth>node.clientWidth));await waitForVisibleCount(page.locator('#thread-panel .markdown-code-copy'),1);
  } else if(caseName.startsWith('language fences')) {
    const source=samples.map(([language,code])=>`\`\`\`${language}\n${code}\n\`\`\``).join('\n\n');
    const row=await post(source);
    await waitForVisibleCount(page.locator('.message[data-message-id] pre code'),samples.length,{timeout:CAPYBARA_DEFAULT});
    for(const [language,code] of samples) {
      const element=marked(row,language);await highlight(element);assert.equal(await element.textContent(),code+'\n');
    }
    await waitForDomCount(row.locator('pre img,pre script'),0);assert.notEqual(await page.evaluate(()=>window.codeExecuted),true);
    assert.equal(await marked(row,'c#').getAttribute('data-code-language'),'csharp');
    assert.equal(await marked(row,'c++').getAttribute('data-code-language'),'cpp');
    await copy(row.locator('pre').filter({has:page.locator('code.language-ts')}),samples.find(([language])=>language==='ts')[1]+'\n');
    const remote=recipient.locator('.message[data-message-id]').filter({has:recipient.locator('pre code.language-ts')});
    await highlight(marked(remote,'ts'));
  } else if(caseName.startsWith('unlabelled code')) {
    const row=await post(literal_code_source);
    await waitForVisibility(row.locator('pre.code-highlighted code .code-token').first(),{timeout:highlight_wait*1000});
    await waitForVisibility(filterVisibleText(marked(row,'text'),'const plain = "@[JZ] :smile:";'),{timeout:highlight_wait*1000});
    assert.equal((await marked(row,'text').textContent()).trim(),'const plain = "@[JZ] :smile:";');
    await waitForVisibleCount(row.locator('code.language-text span,code.language-unknown-language span,p code span'),0);
    await waitForVisibility(filterVisibleText(row.locator('code.language-unknown-language'),'<script>window.codeExecuted = true</script>'));
    assert.equal((await row.locator('code.language-unknown-language').textContent()).trim(),'<script>window.codeExecuted = true</script>');
    await waitForDomCount(row.locator('pre script'),0);assert.notEqual(await page.evaluate(()=>window.codeExecuted),true);
    await waitForVisibleCount(row.locator('.markdown-code-copy'),3);
  } else if(caseName.startsWith('search results highlight')) {
    await page.goto(base+'/searches?q=HighlightSearchExample');
    const row=page.locator(`.message[data-message-id="${code_search_id}"]`);
    await highlight(marked(row,'javascript'),'const');await waitForVisibleCount(row.locator('.markdown-code-copy'),1);
    await actOnVisible(page.getByRole('link',{name:'Back to Designers',exact:true}),'click');
    await page.waitForURL(base+'/rooms/654632876',{timeout:CAPYBARA_DEFAULT});
    assert.equal(new URL(page.url()).pathname,'/rooms/654632876');await highlight(marked(row,'javascript'),'const');await waitForVisibleCount(row.locator('.markdown-code-copy'),1);
    await page.goBack();assert.equal(new URL(page.url()).pathname,'/searches');assert.equal(new URL(page.url()).searchParams.get('q'),'HighlightSearchExample');
    await copy(row,'const value = true;\n');
  } else if(caseName.startsWith('code and copying')) {
    await page.evaluate(()=>{window.Worker=class {constructor() {window.highlighterWorkerFailed=true;throw new Error('Worker unavailable');}};});
    const row=await post(code_source);
    await waitForVisibility(filterVisibleText(row.locator('pre code.language-ts'),'const value: string = "hello";'));
    assert.equal((await row.locator('pre code.language-ts').textContent()).trim(),'const value: string = "hello";');
    await copy(row,'const value: string = "hello";\n');assert.equal(await page.evaluate(()=>window.highlighterWorkerFailed),true);
    await waitForVisibleCount(row.locator('pre code[data-highlighted],pre .code-token'),0);await field(page,'');
  } else if(caseName.startsWith('editing a code block')) {
    const row=await post(code_source);await highlight(marked(row,'ts'),'const');const id=await row.getAttribute('data-message-id');
    await openEdit(page,row);await actOnVisible(page.getByRole('combobox',{name:'Write a message',exact:true}),'fill',{},[code_replacement]);await actOnVisible(page.getByRole('button',{name:'Send Message',exact:true}),'click',{});
    const replacement=page.locator(`.message[data-message-id="${id}"]`);await highlight(marked(replacement,'python'),'def');
    await waitForVisibleCount(replacement.locator('code.language-ts'),0);await waitForVisibleCount(replacement.locator('.markdown-code-copy'),1);
    await copy(replacement,'def greet(name):\n    return "Hello " + name\n');
    await highlight(marked(recipient.locator(`.message[data-message-id="${id}"]`),'python'),'def');
  } else throw new Error(`unimplemented highlighting case ${caseName}`);
}
