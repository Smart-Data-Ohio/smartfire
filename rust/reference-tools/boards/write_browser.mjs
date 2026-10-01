// Actual board write forms, discussion composer and two-browser live rows on Rails and Rust.
import assert from "node:assert/strict"
import fs from "node:fs"
import { chromium } from "playwright"
import { diagnostics } from "../users/browser_diagnostics.mjs"
const base=process.env.WS12_BROWSER_URL
const labels=JSON.parse(fs.readFileSync(process.env.WS12_BROWSER_LABELS,"utf8"))
const path="/rooms/"+labels["rooms.board"]
const browser=await chromium.launch({headless:true,args:["--no-sandbox"]})
let passed=0
async function session() {
 const context=await browser.newContext({viewport:{width:1440,height:1000}})
 await context.route("**/*",r=>new URL(r.request().url()).origin===new URL(base).origin?r.continue():r.abort())
 await context.addCookies([{name:"session_token",value:labels["session_cookies.david"],url:base}])
 const page=await context.newPage(); page.setDefaultTimeout(20000)
 return {context,page}
}
async function scenario(name,run) {
 const {context,page}=await session();const diagnose=diagnostics(page)
 try {assert.equal((await page.goto(base+path)).status(),200);await run(page);passed++;console.log(`${name}: passed`)}
 catch(e){await diagnose(e);throw e}finally{await context.close()}
}
try {
 await scenario("create-result-status-and-live-board-rows",async p=> {
  const viewer=await session()
  try {
   await viewer.page.goto(base+path)
   await viewer.page.waitForFunction(()=>Array.from(document.querySelectorAll("turbo-cable-stream-source")).some(el=>el.hasAttribute("connected")))
   await p.getByRole("link",{name:"New post",exact:true}).click()
   await p.getByLabel("Title",{exact:true}).fill("Ship the launch")
   await p.locator('[name="thread[first_message]"]').fill("Everything must go out on Friday.")
   await p.getByLabel("Owner",{exact:true}).selectOption({label:"Bender Bot"})
   await p.locator('[name="thread[tags]"]').fill("launch, api")
   await p.getByRole("button",{name:"Create post",exact:true}).click()
   await p.getByRole("heading",{name:"Ship the launch",exact:true}).waitFor()
   assert.equal(await p.locator(".board-post__facts .agent-badge").textContent(),"agent")
   assert.deepEqual(await p.locator(".board-tag").allTextContents(),["api","launch"])
   await p.locator(".board-post__messages").filter({hasText:"Everything must go out on Friday."}).waitFor()
   assert.equal(await p.locator("body.board-post").count(),0)
   const id=new URL(p.url()).pathname.split("/").at(-1)
   await viewer.page.locator(`#board_row_channel_thread_${id} .board-row__status`).filter({hasText:"Planned"}).waitFor()
   await p.getByText("Edit result",{exact:true}).click()
   await p.getByLabel("Result in Markdown",{exact:true}).fill("## Shipped on Friday")
   await p.getByRole("button",{name:"Save result",exact:true}).click()
   await p.locator(".board-post__result-body").filter({hasText:"Shipped on Friday"}).waitFor()
   await p.locator(".board-post__history").filter({hasText:"David updated the result"}).waitFor()
   await p.getByText("Update work",{exact:true}).click()
   await p.getByLabel("Work status",{exact:true}).selectOption("in_progress")
   await p.getByRole("button",{name:"Save status",exact:true}).click()
   await p.locator(".board-post__facts").filter({hasText:"In progress"}).waitFor()
   await viewer.page.locator(`#board_row_channel_thread_${id} .board-row__status`).filter({hasText:"In progress"}).waitFor()
   await viewer.page.getByRole("link",{name:"Board",exact:true}).click()
   await viewer.page.locator('.board__column[aria-label="In progress"]').filter({hasText:"Ship the launch"}).waitFor()
   assert.equal(await viewer.page.locator('.board__column[aria-label="Planned"] .board-row').filter({hasText:"Ship the launch"}).count(),0)
  } finally {await viewer.context.close()}
 })
 await scenario("validation-retains-the-submitted-brief-and-tags",async p=> {
  await p.getByRole("link",{name:"New post",exact:true}).click()
  await p.getByLabel("Title",{exact:true}).fill("Rejected tag")
  await p.locator('[name="thread[first_message]"]').fill("Keep this brief")
  await p.locator('[name="thread[tags]"]').fill("invalid!")
  await p.getByRole("button",{name:"Create post",exact:true}).click()
  await p.getByRole("alert").filter({hasText:"Tags use lowercase letters"}).waitFor()
  assert.equal(await p.getByLabel("Title",{exact:true}).inputValue(),"Rejected tag")
  assert.equal(await p.locator('[name="thread[first_message]"]').inputValue(),"Keep this brief")
  assert.equal(await p.locator('[name="thread[tags]"]').inputValue(),"invalid!")
 })
 await scenario("reply-sends-and-clears-the-real-composer",async p=> {
  await p.locator("#board_row_channel_thread_4 .board-row__link").click()
  const editor=p.getByRole("textbox",{name:"Write a thread reply",exact:true})
  await editor.fill("Reply from the board post.")
  await p.getByRole("button",{name:"Send Reply",exact:true}).click()
  await p.locator(".board-post__messages").filter({hasText:"Reply from the board post."}).waitFor()
  await p.waitForFunction(()=>document.querySelector('trix-editor[aria-label="Write a thread reply"]')?.textContent.trim()==="")
 })
 console.log(`WS12 browser board writes: ${passed} passed; 0 failed; Chromium ${browser.version()}; real forms, signed sessions, composer and live Cable rows`)
} finally {await browser.close()}
