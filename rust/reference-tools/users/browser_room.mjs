import assert from "node:assert/strict"
import fs from "node:fs"
import { chromium } from "playwright"
import { diagnostics } from "./browser_diagnostics.mjs"
const base = process.env.WS8BR2_BROWSER_URL
const labels = JSON.parse(fs.readFileSync(process.env.WS8BR2_BROWSER_LABELS, "utf8"))
const browser = await chromium.launch({headless:true, args:["--no-sandbox"]})
let passed = 0
async function scenario(name, run) {
  const context = await browser.newContext({viewport:{width:1440,height:1000}})
  await context.route("**/*", r => new URL(r.request().url()).origin === new URL(base).origin ? r.continue() : r.abort())
  await context.addCookies([{name:"session_token",value:labels["session_cookies.david"],url:base}])
  const p = await context.newPage(); p.setDefaultTimeout(12000)
  const diagnose = diagnostics(p)
  try {
    assert.equal((await p.goto(base + "/rooms/" + labels["rooms.designers"])).status(), 200)
    await p.waitForFunction(() => window.Stimulus?.getControllerForElementAndIdentifier(document.body,"profile-card"))
    await run(p)
    passed++; console.log(`${name}: passed`)
  } catch (e) { await diagnose(e); throw e } finally { await context.close() }
}
const card = p => p.locator("#profile-card-popover:not([hidden])")
try {
  await scenario("message-author-card-message-opens-existing-dm", async p => {
    // The pinned first-message fixture's client_message_id is 0001.
    await p.locator("#message_0001 .message__avatar a").click()
    await card(p).waitFor({state:"attached"})
    await p.locator("#user_card .profile-card__name").filter({hasText:"Jason"}).waitFor()
    assert.equal(await p.locator("#user_card").getByRole("button",{name:"Start call",exact:true}).count(),1)
    await p.locator("#user_card").getByRole("button",{name:"Message",exact:true}).click()
    await p.waitForURL(base + "/rooms/" + labels["rooms.david_and_jason"])
    await p.locator(".room--current").filter({hasText:"Jason"}).waitFor()
  })
  await scenario("message-author-keyboard-card-traps-and-returns-focus", async p => {
    const trigger = p.locator("#message_0001 .message__author button")
    await trigger.press("Enter")
    await card(p).waitFor({state:"attached"})
    await p.locator("#user_card .profile-card__name").filter({hasText:"Jason"}).waitFor()
    await p.locator(".profile-card-popover__panel").press("Tab")
    assert.equal(await p.locator(".profile-card-popover__close").evaluate(el => el === document.activeElement),true)
    await p.locator(".profile-card-popover__close").press("Shift+Tab")
    assert.equal(await p.evaluate(() => document.querySelector(".profile-card-popover__panel").contains(document.activeElement)),true)
    await p.keyboard.press("Escape")
    await p.locator("#profile-card-popover[hidden]").waitFor({state:"attached"})
    assert.equal(await trigger.evaluate(el => el === document.activeElement),true)
  })
  console.log(`WS8br2 browser room cards: ${passed} passed; 0 failed; Chromium ${browser.version()}; real message author triggers, signed session and CSRF`)
} finally { await browser.close() }
