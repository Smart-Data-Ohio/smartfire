import assert from "node:assert/strict"
import fs from "node:fs"
import { chromium } from "playwright"
import { diagnostics } from "./browser_diagnostics.mjs"
const base=process.env.WS8BR2_BROWSER_URL
const labels=JSON.parse(fs.readFileSync(process.env.WS8BR2_BROWSER_LABELS,"utf8"))
const browser=await chromium.launch({headless:true,args:["--no-sandbox"]})
let passed=0
async function scenario(name,run,phone=false) {
  const context=await browser.newContext({viewport:phone?{width:390,height:844}:{width:1400,height:1400}})
  await context.route("**/*",route=>new URL(route.request().url()).origin===new URL(base).origin?route.continue():route.abort())
  await context.addCookies([{name:"session_token",value:labels["session_cookies.david"],url:base}])
  const page=await context.newPage();const diagnose=diagnostics(page);page.setDefaultTimeout(12000)
  try {
    assert.equal((await page.goto(base+"/users")).status(),200)
    if(phone) await page.getByRole("button",{name:"Open workspace navigation",exact:true}).click()
    await page.getByRole("link",{name:"New direct message",exact:true}).click()
    await page.waitForFunction(()=>window.Stimulus?.getControllerForElementAndIdentifier(document.querySelector("[data-controller~='dm-picker']"),"dm-picker"))
    await run(page)
    console.log(`${name}: passed`);passed++
  } catch(e) {await diagnose(e);throw e} finally {await context.close()}
}
const filter=p=>p.locator("#dm_picker_filter")
const rows=p=>p.locator(".dm-picker__row:not([hidden])")
const box=(p,user)=>p.locator(`#pick_user_${labels[`users.${user}`]}`)
const message=(p,n)=>p.locator("#direct_rooms_control").getByRole("button",{name:`Message (${n})`,exact:true})
try {
  await scenario("picker-filter-case-accents-and-empty",async p=>{
    assert.equal(await p.locator("#direct_rooms_control suggestion-option").count(),0)
    assert.equal(await p.getByText("Start Ping",{exact:true}).count(),0)
    const total=await rows(p).count();assert.ok(total>2)
    for(const query of ["chad","CHA","puter"]) {await filter(p).fill(query);assert.equal(await rows(p).count(),1);assert.match(await rows(p).innerText(),/Chad Puterbaugh/)}
    await filter(p).fill("renee");assert.equal(await rows(p).count(),1);assert.match(await rows(p).innerText(),/Renée Dupont/)
    await filter(p).fill("zzz-no-one");assert.equal(await rows(p).count(),0)
    assert.equal(await p.locator("[data-dm-picker-target='empty']").isVisible(),true)
    await filter(p).fill("");assert.equal(await rows(p).count(),total)
    assert.equal(await p.locator("[data-dm-picker-target='empty']").isVisible(),false)
  })
  await scenario("picker-selection-survives-and-posts-dm",async p=>{
    await box(p,"chad").check();await filter(p).fill("kevin")
    assert.equal(await p.locator(`.dm-picker__row:not([hidden]) #pick_user_${labels["users.chad"]}`).count(),0)
    assert.equal(await message(p,1).isVisible(),true)
    await filter(p).fill("");assert.equal(await box(p,"chad").isChecked(),true)
    await message(p,1).click();await p.waitForURL(/\/rooms\/\d+(\?.*)?$/)
    await p.locator(".room--current").filter({hasText:"Chad"}).waitFor()
  })
  await scenario("picker-enter-single-match-only",async p=>{
    await filter(p).fill("j");assert.ok(await rows(p).count()>1);await filter(p).press("Enter")
    assert.equal(await p.locator("#direct_rooms_control input[type='checkbox']:checked").count(),0)
    await filter(p).fill("chad");await filter(p).press("Enter");assert.equal(await box(p,"chad").isChecked(),true)
    await filter(p).fill("kevin");await filter(p).press("Enter")
    assert.equal(await box(p,"chad").isChecked(),true);assert.equal(await box(p,"kevin").isChecked(),false)
    assert.equal(await message(p,1).isVisible(),true)
  })
  await scenario("picker-row-toggle-and-name-card",async p=>{
    const row=p.locator(".dm-picker__row").filter({hasText:"Kevin"})
    await row.evaluate(el=>el.click());assert.equal(await box(p,"kevin").isChecked(),true)
    await row.locator("button.profile-card-name").click()
    await p.locator("#profile-card-popover:not([hidden])").waitFor({state:"attached"})
    await p.locator("#user_card .profile-card__name").filter({hasText:"Kevin"}).waitFor()
  })
  await scenario("picker-phone-targets-and-width",async p=>{
    await filter(p).fill("j")
    assert.equal(await p.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),true)
    const rect=await rows(p).first().evaluate(el=>({height:el.getBoundingClientRect().height,avatar:el.querySelector(".avatar").getBoundingClientRect().width}))
    assert.ok(rect.height>=44,JSON.stringify(rect));assert.ok(Math.abs(rect.avatar-32)<=1,JSON.stringify(rect))
  },true)
  console.log(`WS8br2 browser picker: ${passed} passed; 0 failed; Chromium ${browser.version()}; real signed session and CSRF`)
} finally {await browser.close()}
