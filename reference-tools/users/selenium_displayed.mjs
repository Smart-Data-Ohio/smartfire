// Run the actual Selenium 4.35 atom used by the pinned Rails Capybara finder.
// Provenance and its byte digest are beside the tools-only vendored atom.
import fs from 'node:fs'
import crypto from 'node:crypto'
const atom=fs.readFileSync(new URL('./selenium-is-displayed.js',import.meta.url),'utf8')
const provenance=JSON.parse(fs.readFileSync(new URL('./selenium-is-displayed.provenance.json',import.meta.url),'utf8'))
if(crypto.createHash('sha256').update(atom).digest('hex')!==provenance.sha256)throw Error('Selenium displayed atom drift')
const textAtom=fs.readFileSync(new URL('./selenium-get-text.js',import.meta.url),'utf8')
const textProvenance=JSON.parse(fs.readFileSync(new URL('./selenium-get-text.provenance.json',import.meta.url),'utf8'))
if(crypto.createHash('sha256').update(textAtom).digest('hex')!==textProvenance.sha256)throw Error('Selenium text atom drift')
// Construct the function in Node, so Playwright invokes it with element nodes
// and the argument. A string expression is evaluated without those arguments.
const observe=new Function(String.raw`return (nodes,{value,all})=>{
  const isDisplayed=(${atom});
  const getText=(${textAtom});
  return nodes.filter(el=>{
   if(!all&&!isDisplayed.call(null,el))return false;
   if(value.text===null)return true;
   // Capybara Selenium::Node#all_text uses textContent with normalize_spacing;
   // visible_text delegates to ChromeDriver GET_TEXT, including opacity and
   // overflow clipping rather than the browser's innerText semantics.
   const content=all?(el.textContent||'').replace(/[\u200b\u200e\u200f]/g,'').replace(/[ \n\f\t\v\u2028\u2029]/g,' ').replace(/ +/g,' ').replace(/^[\t\n\v\f\r \u0085\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]+|[\t\n\v\f\r \u0085\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]+$/g,'').replace(/\u00a0/g,' '):getText.call(null,el);
   if(value.source!==undefined)return new RegExp(value.source,value.flags).test(content);
   return content.includes(value.text);
  }).length;
 }`)()
export async function displayedCount(locator,text=null,all=false) {
 const value=text instanceof RegExp?{source:text.source,flags:text.flags}:{text}
 return locator.evaluateAll(observe,{value,all})
}
export async function displayedText(locator) {
 const evaluation=new Function(`return el=>{
  const getText=(${textAtom});
  return getText.call(null,el);
 }`)()
 return locator.evaluate(evaluation)
}
export async function evaluateWithDisplayed(page,fn,args) {
 const evaluation=new Function(`return async args=>{
  const isDisplayed=(${atom});
  return (${fn.toString()})(args,isDisplayed);
 }`)()
 return page.evaluate(evaluation,args)
}
