// Four behaviour ports of test/system/starred_people_test.rb. No screenshot/pixel checks.
import assert from "node:assert/strict"
import fs from "node:fs"
import { chromium } from "playwright"
import { diagnostics } from "./browser_diagnostics.mjs"
const base = process.env.WS12_BROWSER_URL
const labels = JSON.parse(fs.readFileSync(process.env.WS12_BROWSER_LABELS, "utf8"))
const browser = await chromium.launch({headless:true,args:["--no-sandbox"]})
let passed = 0
const id = labels["users.kevin"]
const row = p => p.locator(`#channel-members [data-member-id='${id}']`)
const menu = p => p.locator("#member-row-menu:not([hidden])")
async function openCard(p) {
  await row(p).locator("button.profile-card-name").click()
  await p.locator("#profile-card-popover:not([hidden]) #user_card .profile-card__name").filter({hasText:"Kevin"}).waitFor()
}
async function closeCard(p) {
  await p.locator(".profile-card-popover__close").click()
  await p.locator("#profile-card-popover[hidden]").waitFor({state:"attached"})
}
async function starred(p, expected) {
  await p.locator(`#channel-members [data-member-id='${id}'][data-starred='${expected}']`).waitFor({state:"attached"})
  await p.waitForFunction(({id,expected}) => Boolean(document.querySelector(`#channel-members [aria-label='Starred members'] [data-member-id='${id}']`)) === expected,{id,expected})
  assert.equal(await p.locator(`#channel-members [aria-label='Starred members'] [data-member-id='${id}']`).count(),expected ? 1 : 0)
  assert.equal(await row(p).locator(".member-panel__presence").count(),1)
}
async function scenario(name, viewport, run) {
  const context = await browser.newContext({viewport})
  await context.route("**/*", r => new URL(r.request().url()).origin === new URL(base).origin ? r.continue() : r.abort())
  await context.addCookies([{name:"session_token",value:labels["session_cookies.jz"],url:base}])
  const page = await context.newPage()
  const diagnose = diagnostics(page)
  page.setDefaultTimeout(12000)
  try {
    assert.equal((await page.goto(base + "/rooms/" + labels["rooms.designers"])).status(),200)
    await page.waitForFunction(() => window.Stimulus?.getControllerForElementAndIdentifier(document.body,"profile-card"))
    await run(page)
    passed++; console.log(`${name}: passed`)
  } catch(e) {await diagnose(e); throw e} finally {await context.close()}
}
const desktop = {width:1440,height:1000}
try {
  await scenario("profile-card-star-and-unstar-refetch-members",desktop,async p => {
    await openCard(p)
    await p.locator("#user_card").getByRole("button",{name:"☆ Star",exact:true}).click()
    await p.locator("#user_card").getByRole("button",{name:"★ Unstar",exact:true}).waitFor()
    await starred(p,true)
    await closeCard(p)
    await openCard(p)
    await p.locator("#user_card").getByRole("button",{name:"★ Unstar",exact:true}).click()
    await p.locator("#user_card").getByRole("button",{name:"☆ Star",exact:true}).waitFor()
    await starred(p,false)
    await closeCard(p)
  })
  await scenario("member-row-menu-mouse-keyboard-and-focus",desktop,async p => {
    await row(p).click({button:"right"})
    await menu(p).waitFor()
    assert.equal(await menu(p).locator("[role='menuitem']").evaluate(el => el===document.activeElement),true)
    await menu(p).getByRole("menuitem",{name:"☆ Star",exact:true}).click()
    await starred(p,true)
    await menu(p).getByRole("menuitem",{name:"★ Unstar",exact:true}).click()
    await starred(p,false)
    await menu(p).locator("[role='menuitem']").press("Escape")
    assert.equal(await menu(p).count(),0)
    assert.equal(await row(p).evaluate(el => el.contains(document.activeElement)),true)
    await row(p).locator("button.profile-card-name").focus()
    await p.keyboard.press("Shift+F10")
    await menu(p).waitFor()
    await menu(p).getByRole("menuitem",{name:"☆ Star",exact:true}).click()
    await starred(p,true)
    await menu(p).locator("[role='menuitem']").press("Escape")
    await p.locator(`#channel-members [data-member-id='${labels["users.jz"]}']`).click({button:"right"})
    assert.equal(await menu(p).count(),0)
    // Restore the preference for the remaining scenarios on this isolated server.
    await openCard(p)
    await p.locator("#user_card").getByRole("button",{name:"★ Unstar",exact:true}).click()
    await starred(p,false)
  })
  await scenario("escape-dismisses-menu-after-card-takes-focus",desktop,async p => {
    await row(p).click({button:"right"})
    await menu(p).waitFor()
    await row(p).locator("button.profile-card-avatar").click()
    await p.locator("#profile-card-popover:not([hidden]) #user_card").waitFor()
    await p.locator(".profile-card-popover__close").press("Escape")
    await p.locator("#profile-card-popover[hidden]").waitFor({state:"attached"})
    assert.equal(await menu(p).count(),0)
  })
  await scenario("phone-member-panel-star-and-presence",{width:390,height:844},async p => {
    assert.equal(await p.locator("#channel-members").isVisible(),false)
    await p.getByRole("button",{name:"Show members",exact:true}).click()
    await row(p).waitFor()
    await openCard(p)
    await p.locator("#user_card").getByRole("button",{name:"☆ Star",exact:true}).click()
    await p.locator("#user_card").getByRole("button",{name:"★ Unstar",exact:true}).waitFor()
    await closeCard(p)
    await starred(p,true)
  })
  console.log(`WS12 browser stars: ${passed} passed; 0 failed; Chromium ${browser.version()}; real signed sessions, CSRF and Stimulus`)
} finally {await browser.close()}
