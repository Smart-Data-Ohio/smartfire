// Diagnostics preserve the event order; they never satisfy an assertion.
import {waitForCondition} from './behavior-visibility.mjs';
export const deferredBrowserCases = new Set([
  'a release click landing on the just-opened menu does not activate it',
  'mobile drawer keeps the room list scroll position across close and reopen',
  'mobile drawer reopens on the current room when it is already in view',
]);
export async function traceBrowserSetup(page) {
  await page.addInitScript(({focusDelay}) => {
    const nodes = new WeakMap(); let next = 0;
    const identity = node => node ? (nodes.has(node) ? nodes.get(node) : (nodes.set(node, ++next), next)) : null;
    window.__ws8bmSetupTrace = [];
    const record = (event, target) => {
      const sidebar = document.querySelector('#sidebar');
      const scroller = sidebar?.querySelector('.sidebar__scroll');
      const current = sidebar?.querySelector('a[aria-current="page"]');
      window.__ws8bmSetupTrace.push({event, time: performance.now(), target: target?.id || target?.getAttribute?.('aria-label') || target?.tagName,
        frame: identity(document.querySelector('#user_sidebar')), complete: document.querySelector('#user_sidebar')?.hasAttribute('complete'), scroller: identity(scroller), scroll: scroller?.scrollTop,
        current: current?.dataset.roomId, active: document.activeElement?.id || document.activeElement?.getAttribute('aria-label')});
    };
    for (const type of ['focusin', 'scroll', 'turbo:frame-load', 'workspace-navigation:opening', 'message-actions:open'])
      document.addEventListener(type, event => record(type, event.target), true);
    window.addEventListener('workspace-navigation:opening', event => record(event.type, event.target));
    window.addEventListener('message-actions:open', event => record(event.type, event.target));
    if(focusDelay) {
      // Causal scheduling probe only: delay the client's existing enter-focus
      // callback. It still runs once, with the original code and arguments.
      const request=window.requestAnimationFrame.bind(window);let entering=false;
      window.addEventListener('workspace-navigation:opening',()=>entering=true);
      window.requestAnimationFrame=callback=>{
        if(!entering)return request(callback);
        entering=false;record('enter-focus:scheduled');
        return request(time=>setTimeout(()=>{record('enter-focus:delivered');callback(time);},focusDelay));
      };
    }
  },{focusDelay:Number(process.env.WS8BM_OPEN_FOCUS_DELAY||0)});
  const delay = Number(process.env.WS8BM_SETUP_DELAY || 0);
  if (delay) await page.route(/\/users\/(?:[^/]+\/)?sidebar(?:\?|$)|\/messages\/\d+\/actions(?:\?|$)/, async route => {
    const response = await route.fetch();
    await new Promise(resolve => setTimeout(resolve, delay));
    await route.fulfill({response});
  });
}
export async function readySidebar(page) {
  // Cable is in the layout, so connected streams do not imply that the lazy
  // frame has rendered or synchronized its current link. Wait on the real
  // Turbo completion marker, not a sleep or an arbitrary number of samples.
  // Setup uses Capybara's unchanged 2s default, independently of test waits.
  await waitForCondition(() => page.evaluate(() => {
    const frame=document.querySelector('#user_sidebar');
    const current=document.querySelector('meta[name="current-room-id"]')?.content;
    return !!frame?.hasAttribute('complete') && !!frame.querySelector('.sidebar__scroll') &&
      frame.querySelector('a[aria-current="page"]')?.dataset.roomId===current;
  }));
}
export async function reportBrowserSetup(page) {
  try {
    console.log('WS8bm browser setup trace: ' + JSON.stringify({base: page.url(),
      events: await page.evaluate(() => window.__ws8bmSetupTrace || []),
      geometry: await page.evaluate(() => ({inner: [innerWidth, innerHeight], outer: [outerWidth, outerHeight], motion: document.documentElement.dataset.testMotion,
        menu: (() => {const node = document.querySelector('#message-actions-menu'); const rect = node?.getBoundingClientRect(); return rect && {top: rect.top, height: rect.height, hidden: node.hidden};})()}))}));
  } catch (error) { console.error('WS8bm setup diagnostic failed:', error.message); }
}
