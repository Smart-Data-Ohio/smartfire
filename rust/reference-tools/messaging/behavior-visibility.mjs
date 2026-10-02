// Use the pinned Selenium implementation, including inherited opacity,
// overflow, closed details, image maps and shadow-tree visibility.
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {CAPYBARA_DEFAULT} from './behavior-deadlines.mjs';
const require=createRequire(new URL('../../parity/package.json',import.meta.url));
const {errors}=require('playwright');
const atom=readFileSync(new URL('./selenium/isDisplayed.js',import.meta.url),'utf8');
const defaults=new WeakMap();

export async function installVisibility(context) {
  await context.addInitScript({content:`window.__ws8bmSeleniumVisible = element => element.isConnected && (${atom})(element, false);`});
}
export function setVisibilityTimeout(page,timeout) {defaults.set(page,timeout);}
async function visibleIndices(locator,condition=null) {
  return locator.evaluateAll((elements,condition)=>elements.flatMap((element,index)=>{
    if(!window.__ws8bmSeleniumVisible(element)) return [];
    if(condition) {
      const actual=condition.kind==='attribute'?element.getAttribute(condition.name):element[condition.name];
      if(actual!==condition.value) return [];
    }
    return [index];
  }),condition);
}
export async function visibleCount(locator) {return (await visibleIndices(locator)).length;}
export async function isSeleniumVisible(locator) {return (await visibleCount(locator))>0;}

async function waitForMatches(locator,accept,options,description,condition=null) {
  const timeout=options.timeout??defaults.get(locator.page())??30000;
  const started=Date.now();let count;
  do {
    const indices=await visibleIndices(locator,condition);count=indices.length;
    if(accept(count)&&(timeout===0||Date.now()-started<=timeout)) return indices;
    const remaining=timeout-(Date.now()-started);
    if(timeout!==0&&remaining<=0) break;
    await new Promise(resolve=>setTimeout(resolve,timeout===0?16:Math.min(16,remaining)));
  } while(true);
  throw new errors.TimeoutError(`Selenium visibility ${description} timed out after ${timeout}ms: ${locator}; last visible count: ${count}`);
}
export async function waitForVisibleCount(locator,count,options={}) {
  await waitForMatches(locator,value=>value===count,{timeout:CAPYBARA_DEFAULT,...options},`count=${count}`);
}
export async function visibleMatch(locator,options={}) {
  const indices=await waitForMatches(locator,count=>count>0,options,'visible');
  return locator.nth(indices[0]);
}
// Capybara field/link assertions require the visible node and its value or
// attribute together, not a property on any hidden match in the document.
export async function waitForVisibleProperty(locator,name,value,options={}) {
  await waitForMatches(locator,count=>count>0,{timeout:CAPYBARA_DEFAULT,...options},`${name}=${JSON.stringify(value)}`,{kind:'property',name,value});
}
export async function waitForVisibleAttribute(locator,name,value,options={}) {
  await waitForMatches(locator,count=>count>0,{timeout:CAPYBARA_DEFAULT,...options},`${name}=${JSON.stringify(value)}`,{kind:'attribute',name,value});
}
// Explicit Capybara find/find_field/click_link queries are visibility scoped.
// Share their deadline with the action instead of adding a second full wait.
export async function actOnVisible(locator,action,options={},args=[]) {
  const timeout=options.timeout??CAPYBARA_DEFAULT,started=Date.now();
  const match=await visibleMatch(locator,{timeout});
  const remaining=timeout-(Date.now()-started);
  if(remaining<=0) throw new errors.TimeoutError(`Visible ${action} timed out after ${timeout}ms: ${locator}`);
  return match[action](...args,{...options,timeout:remaining});
}
export async function waitForVisibility(locator,options={}) {
  const state=options.state??'visible';
  // These are explicit DOM-presence checks, not visibility assertions.
  if(state==='attached'||state==='detached') return locator.waitFor(options);
  if(state!=='visible'&&state!=='hidden') throw new Error(`Unsupported visibility state: ${state}`);
  await waitForMatches(locator,count=>state==='visible'?count>0:count===0,options,state);
}
