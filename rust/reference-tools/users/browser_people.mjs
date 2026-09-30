// Real browser requests and original Stimulus modules. No DOM or response normalization.
import assert from "node:assert/strict"
import fs from "node:fs"
import { chromium } from "playwright"

const base = process.env.WS8BR2_BROWSER_URL
const labels = JSON.parse(fs.readFileSync(process.env.WS8BR2_BROWSER_LABELS, "utf8"))
const browser = await chromium.launch({ headless: true, args: ["--no-sandbox"] })
const results = []
async function scenario(name, run) {
  const context = await browser.newContext({ viewport: { width: 1400, height: 1400 } })
  await context.route("**/*", route => {
    const url = new URL(route.request().url())
    return url.origin === new URL(base).origin ? route.continue() : route.abort()
  })
  await context.addCookies([{ name: "session_token", value: labels["session_cookies.david"], url: base }])
  const page = await context.newPage()
  page.setDefaultTimeout(12000)
  try {
    const response = await page.goto(base + "/users")
    assert.equal(response.status(), 200)
    await page.waitForFunction(() => window.Stimulus?.getControllerForElementAndIdentifier(document.querySelector("[data-controller~='multi-select']"), "multi-select"))
    await page.waitForFunction(() => window.Stimulus?.getControllerForElementAndIdentifier(document.body, "profile-card"))
    await run(page)
    results.push(name)
    console.log(`${name}: passed`)
  } finally { await context.close() }
}
const box = (page, user) => page.locator(`#select_user_${labels[`users.${user}`]}`)
const bar = page => page.locator(".people-directory").locator("..").locator("[data-multi-select-target='bar']")
const visibleCard = page => page.locator("#profile-card-popover:not([hidden])")
try {
  await scenario("closed-card-escape", async page => {
    assert.equal(await page.evaluate(() => {
      const event = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true })
      window.Stimulus.getControllerForElementAndIdentifier(document.body, "profile-card").close(event)
      return event.defaultPrevented
    }), false)
  })
  await scenario("keyboard-card-focus-and-escape", async page => {
    const trigger = page.locator(".people-directory__row button.profile-card-name").filter({ hasText: "Jason" })
    await trigger.press("Enter")
    await visibleCard(page).waitFor({ state: "attached" })
    await page.locator(".profile-card-popover__panel").waitFor()
    await page.locator("#user_card .profile-card__name").filter({ hasText: "Jason" }).waitFor()
    await page.locator(".profile-card-popover__panel").press("Tab")
    assert.equal(await page.locator(".profile-card-popover__close").evaluate(el => el === document.activeElement), true)
    await page.locator(".profile-card-popover__close").press("Shift+Tab")
    assert.equal(await page.evaluate(() => document.querySelector(".profile-card-popover__panel").contains(document.activeElement)), true)
    await page.keyboard.press("Escape")
    await page.locator("#profile-card-popover[hidden]").waitFor({ state: "attached" })
    assert.equal(await trigger.evaluate(el => el === document.activeElement), true)
  })
  await scenario("sidebar-avatar-keyboard-card", async page => {
    const trigger = page.locator("#direct_rooms button.profile-card-avatar").first()
    await trigger.press("Enter")
    await visibleCard(page).waitFor({ state: "attached" })
    await page.locator("#user_card .profile-card__name").waitFor()
    assert.equal(new URL(page.url()).pathname, "/users")
    await page.keyboard.press("Escape")
    await page.locator("#profile-card-popover[hidden]").waitFor({ state: "attached" })
    assert.equal(await trigger.evaluate(el => el === document.activeElement), true)
  })
  await scenario("shift-click-range", async page => {
    // The parity seed includes Deploy Bot: this range contains two bots and three humans.
    await box(page, "bender").check()
    await box(page, "kevin").click({ modifiers: ["Shift"] })
    assert.equal(await bar(page).getByRole("button", { name: "Message (5)", exact: true }).count(), 1)
    assert.equal(await bar(page).getByRole("button", { name: "Start huddle (3)", exact: true }).count(), 1)
  })
  await scenario("touch-long-press-selection", async page => {
    const row = page.locator(".people-directory__row").filter({ hasText: "Jason" })
    await row.evaluate(el => {
      const touch = new Touch({ identifier: 1, target: el, clientX: 10, clientY: 10 })
      el.dispatchEvent(new TouchEvent("touchstart", { touches: [touch], bubbles: true, cancelable: true }))
    })
    await page.waitForTimeout(700)
    await row.evaluate(el => el.dispatchEvent(new TouchEvent("touchend", { bubbles: true, cancelable: true })))
    await row.locator("button.profile-card-name").evaluate(el => el.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true, clientX: 10, clientY: 10 })))
    assert.equal(await box(page, "jason").isChecked(), true)
    assert.equal(await bar(page).getByRole("button", { name: "Message (1)", exact: true }).count(), 1)
    assert.equal(await visibleCard(page).count(), 0)
  })
  await scenario("agents-message-only", async page => {
    await box(page, "bender").check()
    assert.equal(await bar(page).getByRole("button", { name: "Message (1)", exact: true }).isEnabled(), true)
    assert.equal(await bar(page).getByRole("button", { name: "Start huddle (0)", exact: true }).isDisabled(), true)
    assert.match(await bar(page).innerText(), /Agents can't join huddles\./)
  })
  await scenario("mixed-huddle-selection", async page => {
    await box(page, "jason").check()
    await box(page, "bender").check()
    assert.equal(await bar(page).getByRole("button", { name: "Message (2)", exact: true }).count(), 1)
    assert.equal(await bar(page).getByRole("button", { name: "Start huddle (1)", exact: true }).count(), 1)
    assert.match(await bar(page).innerText(), /1 agent stays in the DM but won't be rung\./)
  })
  await scenario("three-people-group-dm", async page => {
    for (const user of ["jason", "kevin", "jz"]) await box(page, user).check()
    await bar(page).getByRole("button", { name: "Message (3)", exact: true }).click()
    await page.waitForURL(/\/rooms\/\d+(\?.*)?$/)
    await page.locator(".room--current").filter({ hasText: "Jason, JZ, Kevin" }).waitFor()
  })
  console.log(`WS8br2 browser people: ${results.length} passed; 0 failed; Chromium ${browser.version()}; real signed session and CSRF`)
} finally { await browser.close() }
