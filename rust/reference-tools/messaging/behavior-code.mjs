// Non-pixel code_highlighting_test.rb flows from pinned d7c7de92.
import assert from 'node:assert/strict';
import {CAPYBARA_DEFAULT} from './behavior-deadlines.mjs';
export async function codeHighlighting({author:page,recipient,caseName,fixture,base,openEdit,field}) {
  const {samples,literal_code_source,code_source,code_replacement,highlight_wait,code_search_id}=fixture;
  const marked=(row,language)=>row.locator(`pre code[class~="language-${language}"][data-highlighted="yes"]`);
  async function highlight(code,keyword) {
    if(keyword) {
      // The original waits once for the highlighted keyword, not for any
      // token or for two sequential twenty-second readiness conditions.
      await code.locator('.code-token').filter({hasText:keyword}).first().waitFor({timeout:highlight_wait*1000});
    } else {
      await code.waitFor({timeout:highlight_wait*1000});
      await code.locator('.code-token').first().waitFor({timeout:CAPYBARA_DEFAULT});
    }
  }
  async function post(source) {
    await page.getByRole('combobox',{name:'Write a message',exact:true}).fill(source);
    await page.getByRole('button',{name:'Send Message',exact:true}).click();
    return page.locator('.message[data-message-id]').filter({has:page.locator('pre code')});
  }
  async function copy(row,expected) {
    // Same original clipboard permission-independent boundary. The actual
    // copy handler and its saved source still run.
    await page.evaluate(()=>{navigator.clipboard.writeText=async source=>{window.copiedCode=source;};});
    await row.getByRole('button',{name:'Copy code',exact:true}).click();await row.getByRole('button',{name:'Code copied',exact:true}).waitFor();
    assert.equal(await page.evaluate(()=>window.copiedCode),expected);
  }
  if(caseName.startsWith('language fences')) {
    const source=samples.map(([language,code])=>`\`\`\`${language}\n${code}\n\`\`\``).join('\n\n');
    const row=await post(source);
    await page.waitForFunction(count=>document.querySelectorAll('.message[data-message-id] pre code').length===count,samples.length,{timeout:CAPYBARA_DEFAULT});
    for(const [language,code] of samples) {
      const element=marked(row,language);await highlight(element);assert.equal(await element.textContent(),code+'\n');
    }
    assert.equal(await row.locator('pre img,pre script').count(),0);assert.notEqual(await page.evaluate(()=>window.codeExecuted),true);
    assert.equal(await marked(row,'c#').getAttribute('data-code-language'),'csharp');
    assert.equal(await marked(row,'c++').getAttribute('data-code-language'),'cpp');
    await copy(row.locator('pre').filter({has:page.locator('code.language-ts')}),samples.find(([language])=>language==='ts')[1]+'\n');
    const remote=recipient.locator('.message[data-message-id]').filter({has:recipient.locator('pre code.language-ts')});
    await highlight(marked(remote,'ts'));
  } else if(caseName.startsWith('unlabelled code')) {
    const row=await post(literal_code_source);
    await row.locator('pre.code-highlighted code .code-token').first().waitFor({timeout:highlight_wait*1000});
    await marked(row,'text').waitFor({timeout:highlight_wait*1000});
    assert.equal((await marked(row,'text').textContent()).trim(),'const plain = "@[JZ] :smile:";');
    assert.equal(await row.locator('code.language-text span,code.language-unknown-language span,p code span').count(),0);
    assert.equal((await row.locator('code.language-unknown-language').textContent()).trim(),'<script>window.codeExecuted = true</script>');
    assert.equal(await row.locator('pre script').count(),0);assert.notEqual(await page.evaluate(()=>window.codeExecuted),true);
    assert.equal(await row.locator('.markdown-code-copy').count(),3);
  } else if(caseName.startsWith('search results highlight')) {
    await page.goto(base+'/searches?q=HighlightSearchExample');
    const row=page.locator(`.message[data-message-id="${code_search_id}"]`);
    await highlight(marked(row,'javascript'),'const');assert.equal(await row.locator('.markdown-code-copy').count(),1);
    await page.getByRole('link',{name:'Back to Designers',exact:true}).click();
    await page.waitForURL(base+'/rooms/654632876',{timeout:CAPYBARA_DEFAULT});
    assert.equal(new URL(page.url()).pathname,'/rooms/654632876');await highlight(marked(row,'javascript'),'const');assert.equal(await row.locator('.markdown-code-copy').count(),1);
    await page.goBack();assert.equal(new URL(page.url()).pathname,'/searches');assert.equal(new URL(page.url()).searchParams.get('q'),'HighlightSearchExample');
    await copy(row,'const value = true;\n');
  } else if(caseName.startsWith('code and copying')) {
    await page.evaluate(()=>{window.Worker=class {constructor() {window.highlighterWorkerFailed=true;throw new Error('Worker unavailable');}};});
    const row=await post(code_source);
    await row.locator('pre code.language-ts').filter({hasText:'const value: string = "hello";'}).waitFor();
    assert.equal((await row.locator('pre code.language-ts').textContent()).trim(),'const value: string = "hello";');
    await copy(row,'const value: string = "hello";\n');assert.equal(await page.evaluate(()=>window.highlighterWorkerFailed),true);
    assert.equal(await row.locator('pre code[data-highlighted],pre .code-token').count(),0);await field(page,'');
  } else if(caseName.startsWith('editing a code block')) {
    const row=await post(code_source);await highlight(marked(row,'ts'),'const');const id=await row.getAttribute('data-message-id');
    await openEdit(page,row);await page.getByRole('combobox',{name:'Write a message',exact:true}).fill(code_replacement);await page.getByRole('button',{name:'Send Message',exact:true}).click();
    const replacement=page.locator(`.message[data-message-id="${id}"]`);await highlight(marked(replacement,'python'),'def');
    assert.equal(await replacement.locator('code.language-ts').count(),0);assert.equal(await replacement.locator('.markdown-code-copy').count(),1);
    await copy(replacement,'def greet(name):\n    return "Hello " + name\n');
    await highlight(marked(recipient.locator(`.message[data-message-id="${id}"]`),'python'),'def');
  } else throw new Error(`unimplemented highlighting case ${caseName}`);
}
