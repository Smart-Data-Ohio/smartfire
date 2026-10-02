// Read-only board behaviour slice; original create/result/pane scenarios remain deferred.
import assert from "node:assert/strict"
import fs from "node:fs"
import { chromium } from "playwright"
import { diagnostics } from "../users/browser_diagnostics.mjs"
const base = process.env.WS12_BROWSER_URL
const labels = JSON.parse(fs.readFileSync(process.env.WS12_BROWSER_LABELS, "utf8"))
const boardPath = "/rooms/" + labels["rooms.board"]
const browser = await chromium.launch({headless:true,args:["--no-sandbox"]})
let passed = 0
async function ready(p) {
  await p.waitForFunction(() => {
    const el = document.querySelector("[data-controller='board-list']")
    return el && window.Stimulus?.getControllerForElementAndIdentifier(el,"board-list")
  })
}
async function scenario(name, run) {
  const context = await browser.newContext({viewport:{width:1440,height:1000}})
  await context.route("**/*", r => new URL(r.request().url()).origin === new URL(base).origin ? r.continue() : r.abort())
  await context.addCookies([{name:"session_token",value:labels["session_cookies.david"],url:base}])
  const page = await context.newPage()
  const diagnose = diagnostics(page)
  page.setDefaultTimeout(12000)
  try {
    assert.equal((await page.goto(base + boardPath)).status(),200)
    await ready(page)
    await run(page)
    passed++; console.log(`${name}: passed`)
  } catch(e) {await diagnose(e); throw e} finally {await context.close()}
}
const titles = p => p.locator("[data-board-row] .board-row__title").allTextContents()
async function filter(p, key, value) {
  await p.locator("#" + key).selectOption(value)
  await p.getByRole("button",{name:"Filter",exact:true}).click()
  await p.waitForURL(url => url.searchParams.get(key) === value)
  await ready(p)
}
try {
  await scenario("list-status-and-owner-filter-submit",async p => {
    assert.deepEqual(await titles(p),["Blocked work","In progress work","Planned work"])
    await filter(p,"status","done")
    assert.deepEqual(await titles(p),["Done work"])
    await filter(p,"status","all")
    await filter(p,"owner","agents")
    assert.deepEqual(await titles(p),["Blocked work"])
    await p.getByRole("link",{name:"Clear",exact:true}).click()
    await p.waitForURL(url => !url.searchParams.has("owner"))
    await ready(p)
    assert.deepEqual(await titles(p),["Blocked work","In progress work","Planned work"])
  })
  await scenario("columns-show-done-and-preserve-owner-filter",async p => {
    await p.getByRole("link",{name:"Board",exact:true}).click()
    await p.waitForURL(url => url.searchParams.get("view") === "board")
    await ready(p)
    assert.equal(await p.locator(".board__column").count(),4)
    for (const [status,title] of [["planned","Planned work"],["in_progress","In progress work"],["blocked","Blocked work"],["done","Done work"]]) {
      assert.deepEqual(await p.locator(`#board_column_${status} .board-row__title`).allTextContents(),[title])
    }
    await filter(p,"owner","agents")
    assert.deepEqual(await titles(p),["Blocked work"])
    await p.getByRole("link",{name:"List",exact:true}).click()
    await p.waitForURL(url => url.searchParams.get("view") !== "board")
    await ready(p)
    assert.equal(await p.locator("#owner").inputValue(),"agents")
    assert.deepEqual(await titles(p),["Blocked work"])
  })
  await scenario("stimulus-filters-new-rows-by-server-data-attributes",async p => {
    await filter(p,"owner","me")
    await filter(p,"tag","rust")
    assert.deepEqual(await titles(p),["In progress work","Planned work"])
    // Add the real seeded row shape with one changed attribute each, as Turbo does.
    // The separate socket test verifies real server delivery and rollback silence.
    await p.evaluate(() => {
      const list = document.querySelector("#board_posts")
      const source = list.querySelector("[data-board-row]")
      for (const [suffix,key,value] of [["status","status","done"],["owner","ownerId","394959859"],["tag","tags","release"],["valid","status","planned"]]) {
        const row = source.cloneNode(true)
        row.id = "browser_added_" + suffix
        row.dataset[key] = value
        list.prepend(row)
      }
    })
    await p.locator("#browser_added_valid").waitFor()
    await p.waitForFunction(() => ["status","owner","tag"].every(suffix => !document.querySelector("#browser_added_" + suffix)))
    assert.equal(await p.locator("[data-board-row]").count(),3)
  })
  console.log(`WS12 browser board reads: ${passed} passed; 0 failed; Chromium ${browser.version()}; real signed sessions, GET forms and Stimulus`)
} finally {await browser.close()}
