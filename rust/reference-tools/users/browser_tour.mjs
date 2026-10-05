import assert from "node:assert/strict"
import fs from "node:fs"
import { chromium } from "playwright"
import { diagnostics } from "./browser_diagnostics.mjs"
import { tourKey } from "./browser_scopes.mjs"
const base = process.env.WS8BR2_BROWSER_URL
const labels = JSON.parse(fs.readFileSync(process.env.WS8BR2_BROWSER_LABELS,"utf8"))
const browser = await chromium.launch({headless:true,args:["--no-sandbox"]})
let passed = 0
async function scenario(name,user,run) {
  const context = await browser.newContext({viewport:{width:1440,height:1000}})
  await context.route("**/*",r => new URL(r.request().url()).origin === new URL(base).origin ? r.continue() : r.abort())
  await context.addCookies([{name:"session_token",value:labels[`session_cookies.${user}`],url:base}])
  const p = await context.newPage(); p.setDefaultTimeout(12000)
  const diagnose = diagnostics(p)
  try {
    assert.equal((await p.goto(base + "/rooms/" + labels["rooms.designers"])).status(),200)
    await p.waitForFunction(() => window.Stimulus?.getControllerForElementAndIdentifier(document.querySelector("#tour"),"tour"))
    await run(p)
    passed++; console.log(`${name}: passed`)
  } catch (e) { await diagnose(e); throw e } finally { await context.close() }
}
const card = p => p.locator("#tour .tour__card")
const next = p => p.locator("#tour [data-tour-target='next']")
async function step(p,n) { await p.locator(".tour__progress").filter({hasText:`Step ${n} of 5`}).waitFor() }
async function stamped(p,action) {
  const result = p.waitForResponse(r => new URL(r.url()).pathname === "/users/me/tour" && r.request().method() !== "GET")
  await action(); assert.equal((await result).status(),204)
  await card(p).waitFor({state:"hidden"})
  // Reload proves the persisted stamp, not only the tour controller's local state.
  await p.reload(); await p.waitForFunction(() => window.Stimulus?.getControllerForElementAndIdentifier(document.querySelector("#tour"),"tour"))
  assert.equal(await p.locator("#tour").getAttribute("data-tour-auto-start-value"),"false")
  assert.equal(await card(p).isVisible(),false)
}
try {
  await scenario("tour-keyboard-finish-persists","jz",async p => {
    await step(p,1); assert.equal(await p.locator(".tour__title").innerText(),"Your rooms live here")
    assert.equal(await p.locator("#sidebar.tour__target").count(),1)
    await next(p).click(); await step(p,2); assert.equal(await p.locator("#composer.tour__target").count(),1)
    await tourKey(p).press("ArrowRight"); await step(p,3); assert.equal(await p.locator(".tour__card--center").count(),1)
    await tourKey(p).press("ArrowRight"); await step(p,4); assert.equal(await p.locator(".tour__title").innerText(),"Jump anywhere with Ctrl+K")
    await tourKey(p).press("ArrowLeft"); await step(p,3)
    await tourKey(p).press("ArrowRight"); await tourKey(p).press("ArrowRight"); await step(p,5)
    assert.equal(await p.locator(".tour__title").innerText(),"Shortcuts live under ?")
    assert.equal(await p.locator("#help-menu-button.tour__target").count(),1)
    await stamped(p,() => next(p).click())
  })
  await scenario("tour-escape-skip-persists","jason",async p => {
    await step(p,1); await stamped(p,() => tourKey(p).press("Escape"))
  })
  await scenario("tour-restarts-from-help","kevin",async p => {
    await step(p,1); await stamped(p,() => p.getByRole("button",{name:"Skip tour",exact:true}).click())
    await p.locator("#help-menu-button").click()
    await p.getByRole("menuitem",{name:"Restart tour",exact:true}).click()
    await step(p,1); assert.equal(await card(p).isVisible(),true)
  })
  await scenario("completed-tour-does-not-auto-start","david",async p => {
    assert.equal(await p.locator("#tour").getAttribute("data-tour-auto-start-value"),"false")
    assert.equal(await card(p).isVisible(),false)
    assert.equal(await p.locator("#help-menu-button").isVisible(),true)
  })
  console.log(`WS8br2 browser tour: ${passed} passed; 0 failed; Chromium ${browser.version()}; keyboard, persistence and help-menu behavior; real signed sessions and CSRF`)
} finally { await browser.close() }
