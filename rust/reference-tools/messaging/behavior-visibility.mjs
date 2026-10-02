// Use the pinned Selenium implementation, including inherited opacity,
// overflow, closed details, image maps and shadow-tree visibility.
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
const require=createRequire(new URL('../../parity/package.json',import.meta.url));
const {errors}=require('playwright');
const atom=readFileSync(new URL('./selenium/isDisplayed.js',import.meta.url),'utf8');
const defaults=new WeakMap();

export async function installVisibility(context) {
  await context.addInitScript({content:`window.__ws8bmSeleniumVisible = element => element.isConnected && (${atom})(element, false);`});
}
export function setVisibilityTimeout(page,timeout) {defaults.set(page,timeout);}
async function visibleIndices(locator) {
  return locator.evaluateAll(elements=>elements.flatMap((element,index)=>window.__ws8bmSeleniumVisible(element)?[index]:[]));
}
export async function visibleCount(locator) {return (await visibleIndices(locator)).length;}
export async function isSeleniumVisible(locator) {return (await visibleCount(locator))>0;}

async function waitForMatches(locator,accept,options,description) {
  const timeout=options.timeout??defaults.get(locator.page())??30000;
  const started=Date.now();let count;
  do {
    const indices=await visibleIndices(locator);count=indices.length;
    if(accept(count)&&(timeout===0||Date.now()-started<=timeout)) return indices;
    const remaining=timeout-(Date.now()-started);
    if(timeout!==0&&remaining<=0) break;
    await new Promise(resolve=>setTimeout(resolve,timeout===0?16:Math.min(16,remaining)));
  } while(true);
  throw new errors.TimeoutError(`Selenium visibility ${description} timed out after ${timeout}ms: ${locator}; last visible count: ${count}`);
}
export async function waitForVisibleCount(locator,count,options={}) {
  await waitForMatches(locator,value=>value===count,options,`count=${count}`);
}
export async function visibleMatch(locator,options={}) {
  const indices=await waitForMatches(locator,count=>count>0,options,'visible');
  return locator.nth(indices[0]);
}
export async function waitForVisibility(locator,options={}) {
  const state=options.state??'visible';
  // These are explicit DOM-presence checks, not visibility assertions.
  if(state==='attached'||state==='detached') return locator.waitFor(options);
  if(state!=='visible'&&state!=='hidden') throw new Error(`Unsupported visibility state: ${state}`);
  await waitForMatches(locator,count=>state==='visible'?count>0:count===0,options,state);
}
