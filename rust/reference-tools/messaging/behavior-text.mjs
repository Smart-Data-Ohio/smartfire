// Selenium 4.35.0's getVisibleText routines, with only their Closure dependencies
// supplied here. Keep text visibility separate from raw DOM/source inspection.
import {readFileSync} from 'node:fs';
export const textRoutines=readFileSync(new URL('./selenium/visibleText.js',import.meta.url),'utf8');
export const textBootstrap=`(() => {
  const goog={
    array:{map:(a,f)=>Array.from(a).map(f),forEach:(a,f)=>Array.from(a).forEach(f),peek:a=>a.at(-1),contains:(a,v)=>a.includes(v)},
    string:{isEmptyOrWhitespace:s=>/^\\s*$/.test(s),endsWith:(s,v)=>s.endsWith(v),startsWith:(s,v)=>s.startsWith(v),canonicalizeNewlines:s=>s.replace(/\\r\\n|\\r/g,'\\n')},
    dom:{NodeType:{TEXT:3,ELEMENT:1},TagName:{BR:'BR',TD:'TD'},getPreviousElementSibling:e=>e.previousElementSibling},
    userAgent:{IE:false}
  };
  const bot={dom:{IS_SHADOW_DOM_ENABLED:typeof ShadowRoot==='function',
    isElement:(e,tag)=>e?.nodeType===1&&(!tag||e.tagName===tag),
    isShown:e=>window.__ws8bmSeleniumVisible(e),getEffectiveStyle:(e,p)=>getComputedStyle(e).getPropertyValue(p)
  }};
  ${textRoutines}
  window.__ws8bmVisibleText=bot.dom.getVisibleText;
})();`;
// A filter operates on the selected element itself; a text lookup finds the
// innermost matching node. Both use visible descendant text, never textContent.
export function textSelectorEngine() {
  function queryAll(root,selector) {
    const spec=JSON.parse(selector),expected=spec.regex?new RegExp(spec.value,spec.flags):spec.value;
    const matches=element=>{
      if(!window.__ws8bmSeleniumVisible(element)) return false;
      if(spec.mode==='visible') return true;
      const text=window.__ws8bmVisibleText(element);
      if(spec.regex) expected.lastIndex=0;
      return spec.regex?expected.test(text):spec.exact?text.replace(/\s+/g,' ').trim()===expected:text.includes(expected);
    };
    if(spec.mode==='filter'||spec.mode==='visible') return root.nodeType===1&&matches(root)?[root]:[];
    const elements=[...(root.nodeType===1?[root]:[]),...root.querySelectorAll('*')].filter(matches);
    return elements.filter(element=>!elements.some(child=>child!==element&&element.contains(child)));
  }
  return {queryAll,query:(root,selector)=>queryAll(root,selector)[0]||null};
}
function specification(value,options={},mode='filter') {
  return JSON.stringify(value instanceof RegExp?{mode,regex:true,value:value.source,flags:value.flags}: {mode,value,exact:!!options.exact});
}
export function filterVisibleText(locator,value,options={}) {return locator.locator('capybara-text='+specification(value,options));}
export function byVisibleText(scope,value,options={}) {return scope.locator('capybara-text='+specification(value,options,'find'));}
