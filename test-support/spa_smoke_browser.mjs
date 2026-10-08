// Real-server SPA smoke: the actual campfire binary with the production SPA embedded,
// driven like a person. The Rust test owns the private seed/server; every step here runs
// against real pages, forms, cookies and APIs — no mocks, no stubbed network.
import assert from "node:assert/strict"
import { createHmac } from "node:crypto"
import { readFile } from "node:fs/promises"
import { createRequire } from "node:module"

const { chromium } = process.env.SPA_SMOKE_LOCAL === "1"
  ? createRequire(new URL("../frontend/package.json", import.meta.url))("@playwright/test")
  : await import("playwright")
const origin = process.env.SPA_SMOKE_TARGET
assert.ok(origin, "the Rust test supplies its private server")
const email = process.env.SPA_SMOKE_EMAIL
assert.ok(email, "the Rust test supplies the seed user's email")
const room = process.env.SPA_SMOKE_ROOM
assert.ok(room, "the Rust test supplies the room id")
const labels = JSON.parse(await readFile(new URL("../parity/.seed/default/labels.json", import.meta.url), "utf8"))
const contract = JSON.parse(await readFile(new URL("../crates/spa/compat/urls.json", import.meta.url), "utf8"))

// Later cutover flips must need only JSON expectation edits: every browser-case URL and
// expectation below comes from the contract, with its static placeholders expanded here.
function expand(template, what) {
  assert.equal(typeof template, "string", `browser case ${what} is a string`)
  return template.replace(/\{([A-Z0-9_]+)\}/g, (match, name) => {
    const value = contract.placeholders[name]
    assert.ok(value !== undefined, `urls.json has no placeholder ${name} for browser case ${what}`)
    return String(value)
  })
}

function browserCases() {
  assert.ok(Array.isArray(contract.browser_cases) && contract.browser_cases.length > 0,
    "urls.json must declare browser navigation cases")
  return contract.browser_cases
}

// The real setup page supplies its fresh secret; the Rust fixture uses the seed's frozen clock.
function authenticatorCode(secret) {
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567"
  const bits = [...secret.toUpperCase()].map((character) => alphabet.indexOf(character).toString(2).padStart(5, "0")).join("")
  const key = Buffer.from(bits.match(/.{8}/g).map((byte) => Number.parseInt(byte, 2)))
  const counter = Buffer.alloc(8)
  counter.writeBigUInt64BE(BigInt(Math.floor(new Date(labels["clock.now"]).getTime() / 30_000)))
  const digest = createHmac("sha1", key).update(counter).digest()
  const offset = digest.at(-1) & 15
  return String((digest.readUInt32BE(offset) & 0x7fffffff) % 1_000_000).padStart(6, "0")
}

const step = (name, detail = "") => console.log(`SPA_SMOKE_STEP ${name}${detail ? ` ${detail}` : ""}`)

const browser = await chromium.launch({ channel: "chromium", headless: true })
try {
  const context = await browser.newContext()
  const page = await context.newPage()
  const pageErrors = []
  page.on("pageerror", (error) => pageErrors.push(`pageerror: ${error.message}`))
  page.on("response", (response) => {
    if (response.status() >= 400) step("http-error", `${response.status()} ${new URL(response.url()).pathname}`)
  })
  page.on("console", (message) => {
    if (message.type() !== "error") return
    const url = message.location().url
    // This seed has no LiveKit configuration; the browser also probes the absent favicon.
    // Allow only those exact HTTP diagnostics, keeping script errors and broken assets fatal.
    const expected = (url === `${origin}/api/v1/huddles` && message.text() === "Failed to load resource: the server responded with a status of 503 (Service Unavailable)")
      || (url === `${origin}/favicon.ico` && message.text() === "Failed to load resource: the server responded with a status of 404 (Not Found)")
    if (expected) step("expected-resource-error", url)
    else pageErrors.push(`console.error: ${message.text()} (${url})`)
  })

  // Password sign-in can land on two-factor enrollment (the seed user is unenrolled), either
  // directly or after the SPA shell redirects there. Complete the application's existing
  // security flow instead of bypassing that requirement.
  async function enrollIfNeeded() {
    if (new URL(page.url()).pathname !== "/two_factor_setup") return false
    const secret = (await page.locator("#two_factor_manual_key").innerText()).replace(/\s+/g, "")
    await page.getByLabel("Authenticator code", { exact: true }).fill(authenticatorCode(secret))
    await page.getByRole("button", { name: "Verify and continue", exact: true }).click()
    await page.getByRole("heading", { name: "Save your backup codes", exact: true }).waitFor()
    await page.getByRole("link", { name: "Continue", exact: true }).click()
    await page.waitForURL((url) => !url.pathname.startsWith("/two_factor"))
    step("two-factor-enrolled", new URL(page.url()).pathname)
    return true
  }

  try {
    await page.goto(`${origin}/session/new`, { waitUntil: "load" })
    step("sign-in-page", new URL(page.url()).pathname)
    await page.locator('input[name="email_address"]').fill(email)
    await page.locator('input[name="password"]').fill("secret123456")
    await page.getByRole("button", { name: "Sign in", exact: true }).click()
    await page.waitForURL((url) => url.pathname !== "/session/new")
    const landing = new URL(page.url()).pathname
    step("sign-in-landing", landing)
    await enrollIfNeeded()

    // The SPA shell, then the room a person would open.
    await page.goto(`${origin}/app/r/${room}`)
    if (await enrollIfNeeded() && new URL(page.url()).pathname !== `/app/r/${room}`) {
      await page.goto(`${origin}/app/r/${room}`)
    }
    await page.getByRole("button", { name: "Your account", exact: true }).waitFor()
    const timeline = page.getByRole("log", { name: "Messages" })
    await timeline.waitFor()
    // A history page can move the virtual window; use its visible jump control when needed.
    async function showInTimeline(row) {
      const present = page.locator('.timeline-jump[data-open="true"]').getByRole("button")
      const deadline = Date.now() + 30000
      let lastError
      // Late history pages can replace the window between locating and scrolling a row.
      while (Date.now() < deadline) {
        try {
          await row.or(present).first().waitFor({ timeout: 3000 })
          if (!(await row.isVisible())) await present.click({ timeout: 3000 })
          await row.click({ trial: true, timeout: 3000 })
          return
        } catch (error) {
          if (error.name !== "TimeoutError" && !error.message.includes("Element is not attached")) throw error
          lastError = error
        }
      }
      throw lastError ?? new Error("the timeline never displayed the requested message")
    }
    await showInTimeline(timeline.locator(`article[data-message-row][data-message-id="${contract.placeholders.MSG_JZ}"]`))
    const composer = page.getByRole("textbox", { name: /Message/ })
    await composer.waitFor()
    step("room-opened", `/app/r/${room}`)

    const body = `smoke ${Date.now().toString(36)} ${Math.floor(Math.random() * 1e6).toString(36)}`
    await composer.fill(body)
    const posted = page.waitForResponse((response) => response.request().method() === "POST"
      && new URL(response.url()).pathname === `/api/v1/rooms/${room}/messages`)
    await page.getByRole("button", { name: "Send message", exact: true }).click()
    assert.equal((await posted).status(), 201, "the actual server accepted the message")
    await showInTimeline(timeline.locator("article[data-message-row][data-message-id]").filter({ hasText: body }))
    step("message-sent", body)

    await page.reload({ waitUntil: "load" })
    await showInTimeline(page.getByRole("log", { name: "Messages" }).locator("article[data-message-row][data-message-id]").filter({ hasText: body }))
    step("message-persisted", body)
    step("timeline-asserted", "role=log name=Messages after reload")

    // Legacy message links keep their #message fragment through the server redirect into
    // the SPA: the fragment never reaches the server, so the browser reapplies it to the
    // redirect target, and the SPA must not strip it. Same signed-in session throughout.
    const verifiedCases = []
    for (const browserCase of await browserCases()) {
      assert.equal(typeof browserCase.id, "string", "browser case has a string id")
      const from = expand(browserCase.path, `${browserCase.id}.path`)
      const expectPath = expand(browserCase.expect_path, `${browserCase.id}.expect_path`)
      const expectHash = expand(browserCase.expect_hash, `${browserCase.id}.expect_hash`)
      const expectQuery = browserCase.expect_query === undefined
        ? null
        : expand(browserCase.expect_query, `${browserCase.id}.expect_query`)
      await page.goto(`${origin}${from}`, { waitUntil: "load" })
      // The permalink child renders nothing itself (router.tsx): the room timeline below it
      // is the assertion surface for every room/permalink case.
      await page.getByRole("log", { name: "Messages" }).waitFor()
      const location = new URL(page.url())
      assert.equal(location.pathname, expectPath, `${browserCase.id}: SPA path after redirect`)
      assert.equal(location.hash, expectHash, `${browserCase.id}: fragment preserved through redirect`)
      if (expectQuery !== null) {
        const actual = [...new URLSearchParams(location.search).entries()].sort()
        const expected = [...new URLSearchParams(expectQuery).entries()].sort()
        assert.deepEqual(actual, expected, `${browserCase.id}: query preserved through redirect`)
      }
      step("browser-case", `${browserCase.id} ${location.pathname}${location.search}${location.hash}`)
      verifiedCases.push(browserCase.id)
    }

    const finalUrl = page.url()
    console.log(`SPA_SMOKE_RECEIPT ${JSON.stringify({ room: Number(room), landing, persisted: true, timeline_asserted: true, final_url: finalUrl, browser_cases: verifiedCases })}`)
    assert.deepEqual(pageErrors, [], `the smoke ran without page errors: ${JSON.stringify(pageErrors)}`)
    await context.close()
  } catch (error) {
    let snippet = "<unreadable>"
    try {
      snippet = ((await page.evaluate(() => document.body?.innerText?.slice(0, 2000) ?? "")) || "<empty>").replace(/\n/g, " | ")
    } catch {
      // The page is gone; the URL below is the whole story.
    }
    console.log(`SPA_SMOKE_FAILURE ${JSON.stringify({
      url: page.url(),
      message: error.message?.slice(0, 500),
      body: snippet,
      pageErrors,
    })}`)
    throw error
  }
} finally {
  await browser.close()
}
