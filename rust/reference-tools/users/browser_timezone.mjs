// test/system/timezone_detection_test.rb at d7c7de92; real browser PATCH and server readback.
import assert from "node:assert/strict"
import fs from "node:fs"
import { chromium } from "playwright"

const base = process.env.WS8BR2_BROWSER_URL
const labels = JSON.parse(fs.readFileSync(process.env.WS8BR2_BROWSER_LABELS, "utf8"))
const browser = await chromium.launch({ headless: true, args: ["--no-sandbox"] })
let passed = 0
async function scenario(name, missingToken, run) {
  const context = await browser.newContext({ timezoneId: "America/Los_Angeles" })
  await context.route("**/*", r => new URL(r.request().url()).origin === new URL(base).origin ? r.continue() : r.abort())
  await context.addCookies([{ name: "session_token", value: labels["session_cookies.david"], url: base }])
  if (missingToken) {
    // Rails' system test turns off forgery protection to omit this tag. Production servers
    // keep it enabled; supply the same missing-token DOM input before the real controller
    // connects. This is a negative test input, never a mask on a parity comparison.
    await context.addInitScript(() => {
      const remove = () => document.querySelectorAll('meta[name="csrf-token"]').forEach(el => el.remove())
      new MutationObserver(remove).observe(document, { childList: true, subtree: true })
      remove()
    })
  }
  const page = await context.newPage()
  page.setDefaultTimeout(15000)
  const reports = []
  page.on("request", request => {
    if (new URL(request.url()).pathname === "/users/me/time_zone" && request.method() === "PATCH") reports.push(request)
  })
  try {
    await run(page, reports)
    console.log(`${name}: passed`)
    passed++
  } finally { await context.close() }
}
const connected = page => page.waitForFunction(() => !!window.Stimulus?.getControllerForElementAndIdentifier(document.body, "timezone"))
async function savedZone(page) {
  const response = await page.request.get(base + "/users/me/profile")
  assert.equal(response.status(), 200)
  return page.evaluate(html => new DOMParser().parseFromString(html, "text/html").querySelector('meta[name="current-user-time-zone"]').getAttribute("content"), await response.text())
}
try {
  await scenario("timezone-missing-token-sends-no-report", true, async (page, reports) => {
    assert.equal((await page.goto(base + "/users")).status(), 200)
    await connected(page)
    assert.equal(await page.locator('meta[name="csrf-token"]').count(), 0)
    await page.waitForTimeout(2000) // Rails uses the same settle window after connect.
    assert.equal(reports.length, 0)
    assert.equal(await savedZone(page), null)
  })
  await scenario("timezone-detected-and-persisted-once", false, async (page, reports) => {
    const detection = page.waitForResponse(response => new URL(response.url()).pathname === "/users/me/time_zone" && response.request().method() === "PATCH")
    assert.equal((await page.goto(base + "/users")).status(), 200)
    await connected(page)
    const response = await detection
    assert.equal(response.status(), 200)
    const zone = await page.evaluate(() => Intl.DateTimeFormat().resolvedOptions().timeZone)
    assert.equal(zone, "America/Los_Angeles")
    assert.deepEqual(await response.json(), { time_zone: zone })
    assert.ok(reports[0].headers()["x-csrf-token"])
    assert.equal(await savedZone(page), zone)
    await page.goto(base + "/users/me/profile")
    await connected(page)
    await page.waitForTimeout(2000)
    assert.equal(reports.length, 1)
  })
  console.log(`WS8br2 browser timezone: ${passed} passed; 0 failed; Chromium ${browser.version()}; missing-token guard, real CSRF PATCH and persisted-zone readback`)
} finally { await browser.close() }
