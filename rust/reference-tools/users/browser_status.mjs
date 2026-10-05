// Approved #163 behavior from the sidebar card; member-panel composition remains WS8b-m/WS17.
import assert from "node:assert/strict"
import fs from "node:fs"
import { chromium } from "playwright"
import { diagnostics } from "./browser_diagnostics.mjs"
import { statusField } from "./browser_scopes.mjs"
const base=process.env.WS8BR2_BROWSER_URL
const labels=JSON.parse(fs.readFileSync(process.env.WS8BR2_BROWSER_LABELS,"utf8"))
const browser=await chromium.launch({headless:true,args:["--no-sandbox"]})
let passed=0
async function scenario(name,run,phone=false) {
  if(process.env.WS8BR2_BROWSER_SCOPE_CONTROL && name!=="sidebar-popup-save-closes-and-persists")return
  const context=await browser.newContext({viewport:phone?{width:390,height:844}:{width:1440,height:1000}})
  await context.route("**/*",r=>new URL(r.request().url()).origin===new URL(base).origin?r.continue():r.abort())
  await context.addCookies([{name:"session_token",value:labels["session_cookies.david"],url:base}])
  const p=await context.newPage();const diagnose=diagnostics(p);p.setDefaultTimeout(12000)
  try {
    assert.equal((await p.goto(base+"/users")).status(),200)
    await p.waitForFunction(()=>window.Stimulus?.getControllerForElementAndIdentifier(document.body,"profile-card"))
    if(phone) await p.getByRole("button",{name:"Open workspace navigation",exact:true}).click()
    await p.locator("aside#sidebar a.workspace-user").click()
    await p.locator("#user_card .profile-card__name").filter({hasText:"David"}).waitFor()
    assert.equal(new URL(p.url()).pathname,"/users")
    await p.locator("#user_card").getByRole("link",{name:"Set a status",exact:true}).click()
    await p.locator("#user_card .status-popup__title").waitFor()
    await p.evaluate(()=>{window.statusChanges=0;window.addEventListener("user-status:changed",()=>window.statusChanges++)})
    if(process.env.WS8BR2_BROWSER_SCOPE_CONTROL==='status-field') {
      await p.locator('#user_card #status_popup_custom_status_text').evaluate(input=>{
        input.form.id='status-scope-form'
        input.setAttribute('form','status-scope-form')
        input.closest('#user_card').after(input)
        Object.assign(input.style,{position:'fixed',top:'20px',left:'20px',width:'250px',zIndex:'9999',pointerEvents:'auto'})
      })
      assert.equal(await p.locator('#user_card #status_popup_custom_status_text').count(),0,'INVALID_CONTROL no text field in card')
      assert.equal(await p.locator('#status_popup_custom_status_text').count(),1,'INVALID_CONTROL real form-associated field retained')
      console.log('ORIGINAL_MUTATION real status text field outside user card with form association retained')
    }
    await run(p)
    console.log(`${name}: passed`);passed++
  } catch(e) {await diagnose(e);throw e} finally {await context.close()}
}
const text=p=>statusField(p,"#status_popup_custom_status_text")
const emoji=p=>statusField(p,"#status_popup_custom_status_emoji")
const save=p=>p.locator("#user_card").getByRole("button",{name:"Save",exact:true})
async function closed(p) {
  await p.locator("#profile-card-popover[hidden]").waitFor({state:"attached"})
  assert.equal(await p.evaluate(()=>window.statusChanges),1)
}
try {
  await scenario("sidebar-popup-save-closes-and-persists",async p=>{
    await statusField(p,"#status_popup_presence_setting").selectOption("dnd")
    await emoji(p).fill("🚂");await text(p).fill("On a train")
    await statusField(p,"#status_popup_custom_status_expires_in").selectOption("hour_1")
    await save(p).click();await closed(p)
    assert.equal((await p.goto(base+"/users/me/profile")).status(),200)
    assert.equal(await p.locator("#user_custom_status_emoji").inputValue(),"🚂")
    assert.equal(await p.locator("#user_custom_status_text").inputValue(),"On a train")
    assert.equal(await p.locator("#user_presence_setting").inputValue(),"dnd")
  })
  await scenario("sidebar-popup-clear-closes-and-persists",async p=>{
    await p.locator("#user_card").getByRole("button",{name:"Clear status",exact:true}).click();await closed(p)
    await p.goto(base+"/users/me/profile")
    assert.equal(await p.locator("#user_custom_status_emoji").inputValue(),"")
    assert.equal(await p.locator("#user_custom_status_text").inputValue(),"")
  })
  await scenario("sidebar-popup-cancel-does-not-save",async p=>{
    const before=await text(p).inputValue();await text(p).fill("Never saved")
    await p.locator("#user_card").getByRole("link",{name:"Cancel",exact:true}).click()
    await p.locator("#user_card .profile-card__name").filter({hasText:"David"}).waitFor()
    assert.equal(await p.evaluate(()=>window.statusChanges),0)
    await p.goto(base+"/users/me/profile");assert.equal(await p.locator("#user_custom_status_text").inputValue(),before)
  })
  await scenario("sidebar-popup-invalid-save-stays-open",async p=>{
    await statusField(p,"#status_popup_presence_setting").evaluate(el=>el.add(new Option("Away","away")))
    await statusField(p,"#status_popup_presence_setting").selectOption("away")
    const response=p.waitForResponse(r=>new URL(r.url()).pathname==="/users/me/status"&&r.request().method()!=="GET")
    await save(p).click();assert.equal((await response).status(),422)
    await p.locator("#user_card .field_with_errors").first().waitFor()
    assert.equal(await p.locator("#profile-card-popover:not([hidden])").count(),1)
    assert.equal(await p.evaluate(()=>window.statusChanges),0)
  })
  await scenario("sidebar-popup-phone-width",async p=>{
    // The phone assertion is document-wide in Rails; only form interactions use within.
    assert.equal(await p.locator("#status_popup_custom_status_text").isVisible(),true)
    assert.equal(await p.locator(".profile-card-popover__panel").evaluate(el=>el.scrollWidth<=el.clientWidth),true)
  },true)
  console.log(`WS8br2 browser status: ${passed} passed; 0 failed; Chromium ${browser.version()}; real signed session and CSRF; member-panel integration deferred`)
} finally {await browser.close()}
