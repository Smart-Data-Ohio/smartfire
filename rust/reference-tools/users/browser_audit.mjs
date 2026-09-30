// test/system/audit_log_test.rb at d7c7de92; actual CSV download additionally uses WS9 sudo.
import assert from "node:assert/strict"
import fs from "node:fs"
import { chromium } from "playwright"

const base = process.env.WS8BR2_BROWSER_URL
const labels = JSON.parse(fs.readFileSync(process.env.WS8BR2_BROWSER_LABELS, "utf8"))
const browser = await chromium.launch({ headless: true, args: ["--no-sandbox"] })
let passed = 0
async function scenario(name, run, phone = false) {
  const context = await browser.newContext({ viewport: phone ? { width: 390, height: 844 } : { width: 1440, height: 1000 } })
  await context.route("**/*", r => new URL(r.request().url()).origin === new URL(base).origin ? r.continue() : r.abort())
  await context.addCookies([{ name: "session_token", value: labels["session_cookies.david"], url: base }])
  const page = await context.newPage()
  page.setDefaultTimeout(15000)
  try {
    assert.equal((await page.goto(base + "/account/audit_log")).status(), 200)
    assert.equal(await page.locator("h1").innerText(), "Audit log")
    await run(page)
    console.log(`${name}: passed`)
    passed++
  } finally { await context.close() }
}
try {
  await scenario("audit-filter-and-real-csv-download", async page => {
    assert.equal(await page.locator("tbody tr").count(), 2)
    assert.equal(await page.locator("tbody td code").filter({ hasText: "user.ban" }).count(), 1)
    assert.match(await page.locator("tbody").innerText(), /Kevin <kevin@37signals.com>/)
    const action = page.locator('select[name="audit_action"]')
    assert.match(await action.locator("xpath=..").innerText(), /Action/)
    await action.selectOption("user.ban")
    await page.getByRole("button", { name: "Filter", exact: true }).click()
    await page.waitForFunction(() => new URL(location.href).searchParams.get("audit_action") === "user.ban")
    assert.equal(await page.locator("tbody tr").count(), 1)
    assert.equal(await page.locator("tbody td code").innerText(), "user.ban")
    const exportLink = page.getByRole("link", { name: "Export CSV", exact: true })
    const href = await exportLink.getAttribute("href")
    assert.match(href, /audit_log\.csv/)
    assert.equal(new URL(href, base).searchParams.get("audit_action"), "user.ban")
    await exportLink.click()
    await page.getByRole("heading", { name: "Confirm it's you", exact: true }).waitFor()
    await page.getByPlaceholder("Enter your password", { exact: true }).fill(labels["passwords.all"])
    const download = page.waitForEvent("download")
    await page.locator('form:has(input[name="password"])').getByRole("button", { name: "Confirm", exact: true }).click()
    const file = await download
    assert.equal(await file.failure(), null)
    assert.match(file.suggestedFilename(), /^audit-log-\d{8}-\d{6}\.csv$/)
    const chunks = []
    for await (const chunk of await file.createReadStream()) chunks.push(chunk)
    const bytes = Buffer.concat(chunks)
    const compare = actual => assert.deepEqual(actual, Buffer.from(labels["browser.audit_csv"], "utf8"))
    compare(bytes)
    const broken = Buffer.from(bytes)
    broken[0] ^= 1
    assert.throws(() => compare(broken), assert.AssertionError)
    console.log("audit CSV: byte-identical to the pinned Rails producer after real password confirmation")
    console.log("audit CSV discrimination: changed response byte detected")
  })
  await scenario("audit-phone-filter-and-scroll-region", async page => {
    assert.equal(await page.getByLabel("Actor", { exact: true }).isVisible(), true)
    assert.equal(await page.locator("tbody td code").filter({ hasText: "user.ban" }).count(), 1)
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true)
    const region = page.getByRole("region", { name: "Audit log entries", exact: true })
    assert.equal(await region.getAttribute("tabindex"), "0")
    await region.focus()
    assert.equal(await region.evaluate(el => el === document.activeElement), true)
  }, true)
  console.log(`WS8br2 browser audit: ${passed} passed; 0 failed; Chromium ${browser.version()}; real signed session, filters, sudo and CSV download`)
} finally { await browser.close() }
