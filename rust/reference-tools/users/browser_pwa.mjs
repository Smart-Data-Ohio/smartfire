// test/system/service_worker_test.rb and test/controllers/pwa_controller_test.rb at d7c7de92.
import assert from "node:assert/strict"
import fs from "node:fs"
import { spawnSync } from "node:child_process"
import { fileURLToPath } from "node:url"
import { chromium } from "playwright"
import { diagnostics } from "./browser_diagnostics.mjs"

const base = process.env.WS8BR2_BROWSER_URL
const labels = JSON.parse(fs.readFileSync(process.env.WS8BR2_BROWSER_LABELS, "utf8"))
const browser = await chromium.launch({ headless: true, args: ["--no-sandbox"] })
let passed = 0
async function scenario(name, run) {
  const context = await browser.newContext()
  // Leave worker requests unintercepted to exercise Chromium's normal cache/fetch path.
  // These scenarios use only the private seeded app and its same-origin endpoints.
  await context.addCookies([
    { name: "session_token", value: labels["session_cookies.david"], url: base },
    { name: "enable_service_worker", value: "1", url: base }
  ])
  const page = await context.newPage()
  const diagnose = diagnostics(page)
  page.setDefaultTimeout(15000)
  try {
    await run(page)
    console.log(`${name}: passed`)
    passed++
  } catch(e) {await diagnose(e);throw e} finally { await context.close() }
}

try {
  await scenario("worker-caches-static-assets-only", async page => {
    assert.equal((await page.goto(base + "/users")).status(), 200)
    // Registration uses the served application initializer, not test-side registration.
    await page.waitForFunction(async () => {
      const registration = await navigator.serviceWorker.getRegistration()
      return !!registration?.active && !!navigator.serviceWorker.controller
    })
    await page.locator("aside#sidebar a.workspace-user").click()
    await page.locator("#user_card a[href='/users/me/profile']").click()
    await page.waitForURL(base + "/users/me/profile")
    // This page reuses the first navigation's assets. Fetch an actual linked stylesheet through
    // the now-controlling worker instead of relying on Chrome's module/stylesheet memory cache.
    assert.equal(await page.evaluate(async () => {
      const asset = document.querySelector('link[rel="stylesheet"]')
      if (!navigator.serviceWorker.controller) throw new Error("profile navigation lost its controlling worker")
      return (await fetch(asset.href, { cache: "reload" })).status
    }), 200)
    const responses = await page.evaluate(async () => Promise.all([
      fetch("/users/me/sidebar").then(r => r.status),
      fetch("/account/audit_log", { headers: { Accept: "text/html" } }).then(r => r.status)
    ]))
    assert.deepEqual(responses, [200, 200])
    const paths = await page.evaluate(async () => {
      const paths = []
      for (const name of await caches.keys()) {
        for (const request of await (await caches.open(name)).keys()) paths.push(new URL(request.url).pathname)
      }
      return paths
    })
    assert.ok(paths.includes("/offline.html"), JSON.stringify(paths))
    assert.ok(paths.some(path => path.startsWith("/assets/")), JSON.stringify(paths))
    assert.deepEqual(paths.filter(path => path !== "/offline.html" && !path.startsWith("/assets/")), [])
  })
  await scenario("offline-shell-retry-reloads", async page => {
    assert.equal((await page.goto(base + "/offline.html")).status(), 200)
    assert.equal(await page.locator("h1").innerText(), "You’re offline — reconnecting…")
    assert.match(await page.getByRole("status").innerText(), /automatically when your connection returns/)
    const navigation = page.waitForEvent("framenavigated", frame => frame === page.mainFrame())
    await page.getByRole("button", { name: "Try again now", exact: true }).click()
    await navigation
    assert.equal(new URL(page.url()).pathname, "/offline.html")
    assert.equal(await page.locator("h1").innerText(), "You’re offline — reconnecting…")
  })

  const request = await browser.newContext()
  try {
    const response = await request.request.get(base + "/service-worker.js")
    assert.equal(response.status(), 200)
    assert.match(response.headers()["content-type"], /^text\/javascript/)
    const source = (await response.body()).toString("utf8")
    const harness = fileURLToPath(new URL("service_worker_harness.mjs", import.meta.url))
    const drive = source => spawnSync(process.execPath, [harness], { input: source, encoding: "utf8", timeout: 15000 })
    const original = drive(source)
    assert.equal(original.status, 0, original.stderr + original.stdout)
    assert.match(original.stdout, /service worker harness: all checks passed/)
    process.stdout.write(original.stdout)
    console.log(`served-worker-event-harness: ${original.stdout.split("\n").filter(line => line.startsWith("ok - ")).length} passed`)
    passed++
    // Deliberate mutations of the real response must be caught by the unchanged Rails assertions.
    for (const [name, before, after] of [
      ["authenticated-cache-write", 'return url.pathname === OFFLINE_URL || url.pathname.startsWith("/assets/")', "return true"],
      ["offline-fallback", "if (offline) return offline", "if (false) return offline"],
      ["notification-focus", "return existing.focus()", "return existing"]
    ]) {
      assert.equal(source.split(before).length, 2, name + " mutation site drifted")
      const result = drive(source.replace(before, after))
      assert.notEqual(result.status, 0, name + " regression was missed")
      assert.ok(result.status !== null, result.error?.message)
      console.log(`worker discrimination ${name}: detected`)
    }
  } finally { await request.close() }
  console.log(`WS8br2 browser PWA: ${passed} passed; 0 failed; 3 worker regressions detected; Chromium ${browser.version()}; actual served worker and offline shell`)
} finally { await browser.close() }
