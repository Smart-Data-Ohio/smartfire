// Capybara 3.40 / Selenium semantics for the Rust-only ports of the pinned Drive and sudo
// system tests (drive_browser_tests.rs). Visibility and visible text use the pinned Selenium
// atoms (reference-tools/messaging/behavior-visibility.mjs); every assertion carries its
// Rails source line, so a failure (and each mutation check) names the original assertion.
import assert from 'node:assert/strict';
import {after,before} from 'node:test';
import {chromium} from 'playwright';
import {startProxy} from '../capture/proxy.ts';
import {installVisibility,visibleCount,waitForVisibility,waitForVisibleCount,waitForVisibleText,filterVisibleText,visibleMatch,waitForCondition,waitForDomCount} from '../../reference-tools/messaging/behavior-visibility.mjs';

// Loopback is a secure context, like Capybara's local server.
export const origin='http://127.0.0.1';
const CAPYBARA_WAIT=2000; // Capybara.default_max_wait_time in the pinned harness
// The ids Ruby's users(:name)/rooms(:name) resolve to in the Rails fixtures.
export const users={david:127326141,jason:149087659,jz:773523953,kevin:712064548};
export const rooms={designers:654632876,hq:201306877};
const mutation=process.env.DRIVE_BROWSER_MUTATION?JSON.parse(process.env.DRIVE_BROWSER_MUTATION):null;
let browser,proxy,api;
before(async()=>{
  proxy=await startProxy(process.env.DRIVE_BROWSER_TARGET);
  browser=await chromium.launch({headless:true});
  api=await browser.newContext({proxy:{server:proxy.server,bypass:'<-loopback>'}});
});
after(async()=>{await api?.close();await browser?.close();await proxy?.close();});

// The Ruby bodies' direct model calls go to the test module's control routes.
export async function control(path,data) {
  const response=data===undefined?await api.request.get(origin+path):await api.request.post(origin+path,{data});
  assert.equal(response.status(),200,`control ${path}: ${await response.text()}`);
  return response.json();
}

// WebMock stubs answered by the in-process fake Google API (google_calendar_test_helper.rb).
export const GOOGLE_DRIVE_FILES_URL='https://www.googleapis.com/drive/v3/files';
export const DRIVE_SCOPES='openid email https://www.googleapis.com/auth/calendar.events https://www.googleapis.com/auth/drive.file';
export const connectGoogle=(user,scopes)=>control('/__drive_browser__/google/accounts',{user,scopes});
export function driveListPayload() {
  return {files:[
    {id:'1AbcDefGhIjKlMnOpQrSt',name:'Q3 Planning',mimeType:'application/vnd.google-apps.document',modifiedTime:'2026-09-16T10:30:00.000Z',
      owners:[{displayName:'Riel'}],webViewLink:'https://docs.google.com/document/d/1AbcDefGhIjKlMnOpQrSt/edit'},
    {id:'2BcdEfgHiJkLmNoPqRsTu',name:'Budget 2026',mimeType:'application/vnd.google-apps.spreadsheet',modifiedTime:'2026-09-15T09:00:00.000Z',
      owners:[{displayName:'Jon'}],webViewLink:'https://docs.google.com/spreadsheets/d/2BcdEfgHiJkLmNoPqRsTu/edit'},
  ]};
}
export function driveFilePayload({name='Q3 Planning',mimeType='application/vnd.google-apps.document'}={}) {
  return {id:'1AbcDefGhIjKlMnOpQrSt',name,mimeType,modifiedTime:'2026-09-16T10:30:00.000Z',owners:[{displayName:'Riel'}],
    webViewLink:'https://docs.google.com/document/d/1AbcDefGhIjKlMnOpQrSt/edit',iconLink:'https://drive-thirdparty.googleusercontent.com/16/type/document'};
}
export const stubGoogleDriveFile=(fileId,{status=200,body=driveFilePayload()}={})=>
  control('/__drive_browser__/google/stubs',{method:'get',url:`${GOOGLE_DRIVE_FILES_URL}/${fileId}`,query:{supportsAllDrives:'true'},status,body});
export const stubGoogleDriveList=({status=200,body=driveListPayload()}={})=>
  control('/__drive_browser__/google/stubs',{method:'get',url:GOOGLE_DRIVE_FILES_URL,query:{pageSize:'10'},status,body});
export const lastMessage=(thread)=>control('/__drive_browser__/messages/last'+(thread?`?thread=${thread}`:''));
export const createThread=(room,creator,name)=>control('/__drive_browser__/threads',{room,creator,name}).then(row=>row.id);

const BUTTONS='button, input[type=submit], input[type=reset], input[type=image], input[type=button]';
const FIELDS='textarea, select, input:not([type=submit]):not([type=image]):not([type=radio]):not([type=checkbox]):not([type=hidden]):not([type=file]):not([type=button]):not([type=reset])';
// Capybara's :button, :link, :link_or_button and :field locators with enable_aria_label and
// match: :smart (exact candidates first, then partial). Text is XPath string(.), not visible text.
function named(elements,{kind,name,disabled}) {
  const norm=value=>(value??'').replace(/\s+/g,' ').trim();
  const candidates=element=>{
    const aria=element.getAttribute('aria-label'),text=norm(element.textContent);
    if(kind==='field') {
      const labels=[...(element.labels??[])].map(label=>norm(label.textContent));
      return {exact:[element.id,element.getAttribute('name'),element.getAttribute('placeholder'),aria,...labels],partial:[element.getAttribute('placeholder'),...labels]};
    }
    const alt=[...element.querySelectorAll('img[alt]')].map(image=>image.alt);
    if(element.tagName==='A') return {exact:[element.id,element.title,text,aria,...alt],partial:[element.title,text,...alt]};
    return {exact:[element.id,element.getAttribute('name'),element.getAttribute('value'),element.title,text,aria],partial:[element.getAttribute('value'),element.title,text]};
  };
  const shown=elements.map((element,index)=>[element,index]).filter(([element])=>window.__ws8bmSeleniumVisible(element)&&(disabled===undefined||element.tagName==='A'||element.disabled===disabled));
  const exact=shown.filter(([element])=>candidates(element).exact.some(value=>value===name));
  const chosen=exact.length?exact:shown.filter(([element])=>candidates(element).partial.some(value=>value&&value.includes(name)));
  return chosen.map(([,index])=>index);
}

export async function session(t,{viewport={width:1400,height:1400}}={}) {
  const probe={applied:0};
  // Like the native harness's proxy, a mutation run bypasses the service worker.
  const context=await browser.newContext({proxy:{server:proxy.server,bypass:'<-loopback>'},timezoneId:'UTC',locale:'en-US',viewport,
    ...(mutation?{serviceWorkers:'block'}:{})});
  t.after(async()=>{
    if(mutation) console.log(`DRIVE_BROWSER_MUTATION applied=${probe.applied}`);
    await context.close();
  });
  await installVisibility(context);
  await context.route('**/*',async route=>{
    const url=new URL(route.request().url());
    if(url.origin!==origin) return route.abort();
    if(!mutation||!url.pathname.includes('/assets/'+mutation[0])||!/\.(css|js)$/.test(url.pathname)) return route.continue();
    const response=await route.fetch();
    const body=await response.text();
    assert.ok(body.includes(mutation[1]),`mutation needle in ${url.pathname}`);
    probe.applied++;
    const headers={...response.headers()};
    delete headers['content-length'];delete headers['content-encoding'];
    await route.fulfill({status:response.status(),headers,body:body.replace(mutation[1],mutation[2])});
  });
  const page=await context.newPage();
  page.setDefaultNavigationTimeout(30000);
  return page;
}

// One Capybara session: `file` names the Rails source cited by failures.
export class Capybara {
  constructor(page,file) {this.page=page;this.file=file;this.scopes=[];}
  at(line) {return `${this.file}:${line}`;}
  get scope() {return this.scopes.at(-1)??this.page.locator('html');}
  // Like Capybara's synchronize, a query interrupted by a navigation starts over.
  async cite(line,body) {
    for(let attempt=0;;attempt++) {
      try {return await body();}
      catch(error) {
        if(attempt<20&&/Execution context was destroyed|Frame was detached|Cannot find context/.test(error.message)) continue;
        error.message=`${line===undefined?this.file:typeof line==='string'?line:this.at(line)}: ${error.message}`;throw error;
      }
    }
  }
  // A plain Minitest assertion at `line`.
  verify(line,body) {
    try {body();}
    catch(error) {error.message=`${typeof line==='string'?line:this.at(line)}: ${error.message}`;throw error;}
  }
  async visit(path) {await this.page.goto(path.startsWith('http')||path==='about:blank'?path:origin+path);}
  async within(target,body) {
    const element=typeof target==='string'?await this.find(target):target;
    this.scopes.push(element);
    try {return await body();} finally {this.scopes.pop();}
  }
  locator(css,{text,exactText}={}) {
    let locator=this.scope.locator(css);
    if(text!==undefined) locator=filterVisibleText(locator,text);
    if(exactText!==undefined) locator=filterVisibleText(locator,exactText,{exact:true});
    return locator;
  }
  // Capybara's find (and within) wait for exactly one match: several are Ambiguous.
  async find(css,{text,wait=CAPYBARA_WAIT}={}) {
    const locator=this.locator(css,{text});
    let count=0;
    await waitForCondition(async()=>(count=await visibleCount(locator))===1,{timeout:wait})
      .catch(error=>{error.message=`${count?'ambiguous':'no visible'} match for ${JSON.stringify(css)}${text===undefined?'':` with text ${JSON.stringify(text)}`} (${count} found): ${error.message}`;throw error;});
    return visibleMatch(locator,{timeout:wait});
  }
  async assertSelector(css,{text,exactText,count,visible=true,wait=CAPYBARA_WAIT,at}={}) {
    await this.cite(at,async()=>{
      const locator=this.locator(css,{text,exactText});
      if(visible===false) return count===undefined?waitForVisibility(locator,{state:'attached',timeout:wait}):waitForDomCount(locator,count,{timeout:wait});
      if(count!==undefined) return waitForVisibleCount(locator,count,{timeout:wait});
      return waitForVisibility(locator,{timeout:wait});
    });
  }
  async assertNoSelector(css,{text,visible=true,wait=CAPYBARA_WAIT,at}={}) {
    await this.cite(at,async()=>{
      const locator=this.locator(css,{text});
      return visible===false?waitForDomCount(locator,0,{timeout:wait}):waitForVisibleCount(locator,0,{timeout:wait});
    });
  }
  async assertText(text,{wait=CAPYBARA_WAIT,at}={}) {
    await this.cite(at,()=>waitForVisibleText(this.scopes.length?this.scope:this.page.locator('body'),text,{timeout:wait}));
  }
  async assertNoText(text,{wait=CAPYBARA_WAIT,at}={}) {
    await this.cite(at,()=>waitForVisibleCount(filterVisibleText(this.scopes.length?this.scope:this.page.locator('body'),text),0,{timeout:wait}));
  }
  // `single` is find's rule (one match, else Ambiguous); assert_button accepts any match.
  async named(kind,name,{disabled,wait=CAPYBARA_WAIT,single=true}={}) {
    const css=kind==='button'?BUTTONS:kind==='link'?'a[href]':kind==='field'?FIELDS:`${BUTTONS}, a[href]`;
    const locator=this.scope.locator(css);
    let indices=[];
    const found=async()=>{
      try {indices=await locator.evaluateAll(named,{kind,name,disabled});}
      catch(error) {if(/Execution context was destroyed|Frame was detached|Cannot find context/.test(error.message)) return false;throw error;}
      return single?indices.length===1:indices.length>0;
    };
    await waitForCondition(found,{timeout:wait})
      .catch(error=>{error.message=`${indices.length>1?'ambiguous':'no visible'} ${kind} ${JSON.stringify(name)}${disabled===undefined?'':` (disabled: ${disabled})`}: ${error.message}`;throw error;});
    return locator.nth(indices[0]);
  }
  async assertButton(name,{disabled=false,wait=CAPYBARA_WAIT,at}={}) {await this.cite(at,()=>this.named('button',name,{disabled,wait,single:false}));}
  async clickOn(name,options={}) {await (await this.named('link_or_button',name,{disabled:false,...options})).click();}
  async clickButton(name,{wait}={}) {await (await this.named('button',name,{disabled:false,wait})).click();}
  async clickLink(name) {await (await this.named('link',name)).click();}
  async findField(name,options={}) {return this.named('field',name,{disabled:false,...options});}
  // Selenium's set on a text field: select the current value, then type over it. On date and
  // time inputs a Rails string is timeable, so Capybara sets the value by script instead.
  async fillIn(name,value) {
    const field=await this.findField(name);
    if(['date','time','datetime-local'].includes(await field.evaluate(element=>element.type))) return field.fill(value);
    await field.focus();
    await field.evaluate(element=>element.select());
    await field.pressSequentially(value);
  }
  // Selenium execute_script/evaluate_script: a function body called with `arguments`.
  // Evaluating it as one expression keeps Ruby's in-page JavaScript byte-identical.
  async execute(body,...args) {
    return this.page.evaluate(`(function(){${body}\n}).apply(null, ${JSON.stringify(args)})`);
  }
  async evaluate(expression) {return this.execute(`return (${expression}\n);`);}
  async synchronize(body,{wait=CAPYBARA_WAIT,at}={}) {await this.cite(at,()=>waitForCondition(body,{timeout:wait}));}

  // test_helpers/system_test_helper.rb
  async signIn(email,password='secret123456') {
    await this.visit(`/test_session?email_address=${encodeURIComponent(email)}&password=${encodeURIComponent(password)}`);
    await this.assertSelector('a.btn',{text:'Designers',wait:10000,at:'system_test_helper.rb:65'});
  }
  async waitForCableConnection(wait=15000) {
    await this.synchronize(()=>this.page.evaluate(()=>{
      const sources=document.querySelectorAll('turbo-cable-stream-source');
      return sources.length>=3&&[...sources].every(source=>source.hasAttribute('connected'));
    }),{wait,at:'system_test_helper.rb:72'});
  }
  async joinRoom(room) {
    await this.visit(`/rooms/${room}`);
    await this.waitForCableConnection();
    await this.dismissPwaInstallPrompt();
  }
  async dismissPwaInstallPrompt() {
    if(await this.locator("[data-pwa-install-target~='dialog']").evaluateAll(nodes=>nodes.some(node=>window.__ws8bmSeleniumVisible(node)))) await this.clickOn('Close');
  }
  async sendMessage(message) {
    const markdown=await this.locator('[id="message_markdown_source"]').evaluateAll(nodes=>nodes.some(node=>window.__ws8bmSeleniumVisible(node)));
    assert.ok(markdown,'system_test_helper.rb:85: the composer renders the markdown editor');
    await this.fillInMarkdown('message_markdown_source',message);
    await this.clickOn('Send Message');
  }
  async fillInMarkdown(name,value) {
    const editor=await this.findField(name);
    await editor.click();
    await editor.evaluate((editor,source)=>{
      editor.value=source;
      editor.dispatchEvent(new InputEvent('input',{bubbles:true,inputType:'insertFromPaste',data:source}));
    },value);
  }
  async withinMessage(message,body) {return this.within(`[id="message_${message.client_message_id}"]`,body);}
  async rightClickMessage() {
    await (await visibleMatch(this.scope.locator("[data-message-edit-format], [data-reply-target='body']"),{timeout:CAPYBARA_WAIT})).click({button:'right'});
  }
  async assertMessageMenuOpen() {
    await this.assertSelector("[data-message-actions-target='menu']",{wait:10000,at:'system_test_helper.rb:141'});
    await this.assertSelector('.message[data-message-actions-open]',{at:'system_test_helper.rb:142'});
  }
}

// WebMockSystemTestHelper#before_teardown leaves the page before stubs reset.
export async function finish(c,key) {
  await c.visit('about:blank');
  console.log(`DRIVE_BROWSER_RECEIPT ${key}`);
}
