// Shared real-browser fixture; signed Rust sessions and live Cable.
import assert from 'node:assert/strict';
import { before, after } from 'node:test';
import { chromium } from 'playwright';
import { startProxy } from '../capture/proxy.ts';
let browser, proxy, api;
// Loopback is a secure browser context, like Capybara's local Rails server.
const origin = 'http://127.0.0.1';
before(async () => {
  proxy = await startProxy(process.env.WS13_SYSTEM_TARGET);
  browser = await chromium.launch({headless:true,args:['--use-fake-device-for-media-stream','--use-fake-ui-for-media-stream','--mute-audio']});
  api = await browser.newContext({proxy:{server:proxy.server,bypass:'<-loopback>'}});
});
after(async () => { await api?.close(); await browser?.close(); await proxy?.close(); });
async function fixture(t, names, options={}) {
  const response = await api.request.post(`${origin}/__ws13__/fixture`,{data:options});
  assert.equal(response.status(),200);
  const data = await response.json();
  const pages = {};
  for (const name of names) {
    // Each browser context owns its forwarding agent. Close both together so
    // cancelled page requests cannot occupy the shared agent across declarations.
    const pageProxy = await startProxy(process.env.WS13_SYSTEM_TARGET);
    const context = await browser.newContext({proxy:{server:pageProxy.server,bypass:'<-loopback>'},timezoneId:'UTC',locale:'en-US',viewport:{width:1400,height:1000}});
    t.after(async () => { await context.close(); await pageProxy.close(); });
    await context.addCookies([{name:'session_token',value:data.people[name].cookie,url:origin}]);
    const page = await context.newPage();
    page.setDefaultTimeout(2000); // Capybara selector wait; individual Rails waits below are unchanged.
    // Capybara's selector wait does not bound Selenium page loads. Retain
    // Playwright's normal navigation budget rather than inheriting that 2 s wait.
    page.setDefaultNavigationTimeout(30000);
    await visitRoom(page,data.room);
    pages[name] = page;
  }
  const state = async () => (await api.request.get(`${origin}/__ws13__/rooms/${data.room}`)).json();
  return {...data,pages,state};
}
async function visitRoom(page,room) {
  const response = await page.goto(`${origin}/rooms/${room}`,{waitUntil:'domcontentloaded'});
  assert.equal(response.status(),200);
  await page.waitForFunction(() => {
    const sources = [...document.querySelectorAll('turbo-cable-stream-source')];
    return sources.length>=3 && sources.every(source=>source.hasAttribute('connected'));
  },null,{timeout:15000});
  // Cable and Stimulus connect independently. The Rails helper assumes the
  // huddle controller exists before installing its SDK fixture.
  await page.waitForFunction(() => !!window.Stimulus?.getControllerForElementAndIdentifier(document.getElementById('channel-huddle'),'huddle'),null,{timeout:2000});
}

async function text(page, selector, value, timeout=2000) {
  await page.locator(selector).filter({hasText:value}).first().waitFor({state:'visible',timeout});
}
async function absent(page,selector,timeout=2000) {
  await page.locator(selector).waitFor({state:'hidden',timeout});
}
// Selenium/Capybara considers this empty, whitespace-bearing inline span displayed.
// Playwright requires a nonzero box. Preserve the original display/visibility assertion
// without manufacturing dimensions absent from the byte-identical Rails/CSS oracle.
async function emptyStackShown(page,selector) {
  await page.waitForFunction(selector=>{
    let element=document.querySelector(selector);if(!element) return false;
    for (;element;element=element.parentElement) {
      const style=getComputedStyle(element);
      if(element.hidden || style.display==='none' || ['hidden','collapse'].includes(style.visibility)) return false;
    }
    return true;
  },selector,{timeout:2000});
}
async function panel(page) { await page.getByRole('button',{name:'Show stage',exact:true}).click(); }
const row = id => `#stage_row_membership_${id}`;
const controls = room => `#stage_controls_rooms_stage_${room}`;


export { fixture, visitRoom, text, absent, emptyStackShown, panel, row, controls };
