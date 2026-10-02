// Actual board post read interactions; create/update submissions remain in the write slice.
import assert from "node:assert/strict"
import fs from "node:fs"
import { chromium } from "playwright"
import { diagnostics } from "../users/browser_diagnostics.mjs"
const base=process.env.WS12_BROWSER_URL
const labels=JSON.parse(fs.readFileSync(process.env.WS12_BROWSER_LABELS,"utf8"))
const boardPath="/rooms/"+labels["rooms.board"]
const browser=await chromium.launch({headless:true,args:["--no-sandbox"]})
let passed=0
async function scenario(name,run) {
 const context=await browser.newContext({viewport:{width:1440,height:1000}})
 await context.route("**/*",r=>new URL(r.request().url()).origin===new URL(base).origin?r.continue():r.abort())
 await context.addCookies([{name:"session_token",value:labels["session_cookies.david"],url:base}])
 const page=await context.newPage();const diagnose=diagnostics(page);page.setDefaultTimeout(12000)
 try {assert.equal((await page.goto(base+boardPath)).status(),200);await run(page);passed++;console.log(`${name}: passed`)}
 catch(e){await diagnose(e);throw e}finally{await context.close()}
}
try {
 await scenario("new-post-form-and-cancel",async p=> {
  await p.getByRole("link",{name:"New post",exact:true}).click()
  await p.getByRole("heading",{name:"Start a post",exact:true}).waitFor()
  assert.equal(await p.locator("#thread_work_status").inputValue(),"planned")
  assert.deepEqual(await p.locator("#thread_work_owner_id optgroup").evaluateAll(xs=>xs.map(x=>x.label)),["Members","Agents"])
  assert.deepEqual(await p.locator("#board-tag-suggestions option").evaluateAll(xs=>xs.map(x=>x.value)),["release","rust"])
  await p.getByRole("link",{name:"Cancel",exact:true}).click()
  await p.locator("#board-title").waitFor()
 })
 await scenario("post-discussion-template-and-controls",async p=> {
  await p.locator("#board_row_channel_thread_4 .board-row__link").click()
  await p.locator("#post-title").waitFor()
  assert.equal(await p.locator('meta[name="current-room-id"]').getAttribute("content"),String(labels["rooms.board"]))
  assert.equal(await p.locator('script[data-messages-target="template"]').count(),1)
  assert.equal(await p.locator(".board-post__messages .message").count(),1)
  assert.equal(await p.locator(".board-post__result-empty").textContent(),"No result recorded yet.")
  await p.getByText("Edit result",{exact:true}).click()
  assert.equal(await p.locator('textarea[name="thread[result_markdown]"]').isVisible(),true)
  assert.equal(await p.locator('textarea[name="thread[result_markdown]"]').getAttribute("maxlength"),"20000")
  await p.waitForFunction(()=> {
   const el=document.querySelector('[data-controller="messages drop-target"]')
   return el && window.Stimulus?.getControllerForElementAndIdentifier(el,"messages")
  })
 })
 await scenario("real-pane-fetch-and-anchor",async p=> {
  const path=boardPath+"/threads/4/content"
  const pane=await p.evaluate(async path=>{const r=await fetch(path);return {status:r.status,latest:r.headers.get("X-Thread-Content-At-Latest"),html:await r.text()}},path)
  assert.equal(pane.status,200);assert.equal(pane.latest,"true")
  const id=await p.evaluate(html=> {const root=document.createElement("div");root.innerHTML=html;return root.querySelector(".message[data-message-id]")?.dataset.messageId},pane.html)
  assert.ok(id,"pane carries real message id")
  const anchor=await p.evaluate(async path=>{const r=await fetch(path);return {status:r.status,latest:r.headers.get("X-Thread-Content-At-Latest"),html:await r.text()}},path+"?message_id="+id)
  assert.equal(anchor.status,200);assert.equal(anchor.latest,"false")
  assert.ok(anchor.html.includes(`data-messages-anchor-message-id-value="${id}"`))
 })
 console.log(`WS12 browser board posts: ${passed} passed; 0 failed; Chromium ${browser.version()}; real signed sessions, navigation, controls and pane fetches`)
} finally {await browser.close()}
